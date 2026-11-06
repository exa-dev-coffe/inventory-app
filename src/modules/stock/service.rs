use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

use crate::modules::grpc::{
    CheckStockResponse, DeductStockResponse, OrderItemPayload, ReplenishItemPayload,
    ReplenishStockResponse, RollbackStockResponse, UnavailableItem,
};
use crate::pkg::{errors::AppError, queue::RabbitMQBroker};
use super::model::{StockAdjustmentDto, StockLog};
use super::repository::StockRepository;

#[derive(Clone)]
pub struct StockService {
    repo: Arc<StockRepository>,
    rabbitmq: Option<Arc<RabbitMQBroker>>,
}

impl StockService {
    pub fn new(repo: Arc<StockRepository>) -> Self {
        Self {
            repo,
            rabbitmq: None,
        }
    }

    pub fn with_rabbitmq(
        repo: Arc<StockRepository>,
        rabbitmq: Option<Arc<RabbitMQBroker>>,
    ) -> Self {
        Self { repo, rabbitmq }
    }

    pub async fn publish_menu_availability_if_changed(&self, ingredient_ids: &[Uuid]) {
        if ingredient_ids.is_empty() {
            return;
        }

        let menus = match self.repo.get_menus_using_ingredients(ingredient_ids).await {
            Ok(m) => m,
            Err(e) => {
                tracing::error!(error = %e, "Failed to retrieve menus for ingredient check");
                return;
            }
        };

        if menus.is_empty() {
            return;
        }

        for menu_id in menus {
            if let Ok(is_available) = self.repo.check_menu_availability(menu_id).await {
                if let Some(broker) = &self.rabbitmq {
                    let payload = serde_json::json!({
                        "menu_id": menu_id,
                        "is_available": is_available
                    });
                    if let Ok(bytes) = serde_json::to_vec(&payload) {
                        if let Err(e) = broker.publish("", "menu.update_availability", &bytes).await {
                            tracing::warn!(error = %e, menu_id, "Failed to publish menu availability event to RabbitMQ");
                        } else {
                            tracing::info!(menu_id, is_available, "Published menu.update_availability event to RabbitMQ");
                        }
                    }
                }
            }
        }
    }

    pub async fn get_logs(
        &self,
        ingredient_id: Option<Uuid>,
        change_type: Option<&str>,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<StockLog>, AppError> {
        self.repo
            .get_logs(
                ingredient_id,
                change_type,
                limit.unwrap_or(50),
                offset.unwrap_or(0),
            )
            .await
            .map_err(AppError::Database)
    }

    pub async fn adjust_stock(&self, dto: StockAdjustmentDto) -> Result<StockLog, AppError> {
        let ing_id = dto.ingredient_id;
        let log = self.repo
            .adjust_stock(
                dto.ingredient_id,
                &dto.change_type,
                dto.quantity,
                dto.reference_id.as_deref(),
                dto.notes.as_deref(),
            )
            .await
            .map_err(AppError::Database)?;

        self.publish_menu_availability_if_changed(&[ing_id]).await;

        if dto.change_type == "OUT_WASTE" || dto.change_type == "OUT_SPOILAGE" {
            if let Ok(Some((name, cost_per_unit))) = self.repo.get_ingredient_cost_info(ing_id).await {
                let qty = dto.quantity.abs();
                let total_loss = qty * cost_per_unit;
                if let Some(broker) = &self.rabbitmq {
                    let payload = serde_json::json!({
                        "ingredient_id": ing_id,
                        "ingredient_name": name,
                        "quantity": qty,
                        "unit_cost": cost_per_unit,
                        "total_loss": total_loss,
                        "notes": dto.notes.as_deref().unwrap_or("Stock spoilage / waste"),
                        "timestamp": chrono::Utc::now().to_rfc3339()
                    });
                    if let Ok(bytes) = serde_json::to_vec(&payload) {
                        let _ = broker.publish("", "inventory.spoilage_recorded", &bytes).await;
                        tracing::info!(ingredient = %name, %total_loss, "Published inventory.spoilage_recorded event to RabbitMQ");
                    }
                }
            }
        }

        Ok(log)
    }

    async fn calculate_requirements(
        &self,
        items: &[OrderItemPayload],
    ) -> Result<(HashMap<Uuid, Decimal>, Vec<(i64, Uuid, String, Decimal, Decimal)>), AppError> {
        let menu_ids: Vec<i64> = items.iter().map(|i| i.menu_id).collect();
        let requirements = self
            .repo
            .get_requirements_for_menus(&menu_ids)
            .await
            .map_err(AppError::Database)?;

        let menu_qty_map: HashMap<i64, Decimal> = items
            .iter()
            .map(|i| (i.menu_id, Decimal::from(i.quantity)))
            .collect();

        let mut total_needed: HashMap<Uuid, Decimal> = HashMap::new();
        let mut detail_list = Vec::new();

        for req in requirements {
            if let Some(&qty) = menu_qty_map.get(&req.menu_id) {
                let needed = req.amount_per_unit * qty;
                *total_needed.entry(req.ingredient_id).or_insert(Decimal::ZERO) += needed;
                detail_list.push((
                    req.menu_id,
                    req.ingredient_id,
                    req.ingredient_name,
                    needed,
                    req.current_stock,
                ));
            }
        }

        Ok((total_needed, detail_list))
    }

    pub async fn check_availability(
        &self,
        items: Vec<OrderItemPayload>,
    ) -> Result<CheckStockResponse, AppError> {
        let (total_needed, detail_list) = self.calculate_requirements(&items).await?;
        let mut missing_items = Vec::new();

        for (menu_id, ingredient_id, name, needed, stock) in detail_list {
            let total_for_this_ing = total_needed.get(&ingredient_id).copied().unwrap_or(needed);
            if total_for_this_ing > stock {
                missing_items.push(UnavailableItem {
                    menu_id,
                    ingredient_name: name,
                    stock_available: stock.to_f64().unwrap_or(0.0),
                    stock_needed: total_for_this_ing.to_f64().unwrap_or(0.0),
                });
            }
        }

        let is_available = missing_items.is_empty();
        Ok(CheckStockResponse {
            is_available,
            missing_items,
        })
    }

    pub async fn deduct_stock(
        &self,
        transaction_id: &str,
        items: Vec<OrderItemPayload>,
    ) -> Result<DeductStockResponse, AppError> {
        let (total_needed, _) = self.calculate_requirements(&items).await?;
        let deductions: Vec<(Uuid, Decimal)> = total_needed.into_iter().collect();

        if deductions.is_empty() {
            return Ok(DeductStockResponse {
                success: true,
                message: "No ingredients required for this order".to_string(),
            });
        }

        let success = self
            .repo
            .deduct_batch(transaction_id, &deductions)
            .await
            .map_err(AppError::Database)?;

        if success {
            let ing_ids: Vec<Uuid> = deductions.iter().map(|(id, _)| *id).collect();
            self.publish_menu_availability_if_changed(&ing_ids).await;

            Ok(DeductStockResponse {
                success: true,
                message: format!("Stock deducted successfully for tx {}", transaction_id),
            })
        } else {
            Ok(DeductStockResponse {
                success: false,
                message: "Insufficient stock to complete deduction".to_string(),
            })
        }
    }

    pub async fn rollback_stock(
        &self,
        transaction_id: &str,
        items: Vec<OrderItemPayload>,
    ) -> Result<RollbackStockResponse, AppError> {
        let (total_needed, _) = self.calculate_requirements(&items).await?;
        let restorations: Vec<(Uuid, Decimal)> = total_needed.into_iter().collect();

        if !restorations.is_empty() {
            self.repo
                .rollback_batch(transaction_id, &restorations)
                .await
                .map_err(AppError::Database)?;

            let ing_ids: Vec<Uuid> = restorations.iter().map(|(id, _)| *id).collect();
            self.publish_menu_availability_if_changed(&ing_ids).await;
        }

        Ok(RollbackStockResponse {
            success: true,
            message: format!("Stock rollback completed for tx {}", transaction_id),
        })
    }

    pub async fn replenish_stock(
        &self,
        purchase_id: &str,
        items: Vec<ReplenishItemPayload>,
    ) -> Result<ReplenishStockResponse, AppError> {
        let mut parsed_items = Vec::new();
        for item in items {
            let ing_id = Uuid::parse_str(&item.ingredient_id)
                .map_err(|_| AppError::bad_request("Invalid ingredient UUID", "INVALID_UUID"))?;
            let qty = Decimal::from_f64_retain(item.quantity).unwrap_or(Decimal::ZERO);
            let unit_cost = Decimal::from_f64_retain(item.unit_cost).unwrap_or(Decimal::ZERO);
            parsed_items.push((ing_id, qty, unit_cost));
        }

        self.repo
            .replenish_batch(purchase_id, &parsed_items)
            .await
            .map_err(AppError::Database)?;

        let ing_ids: Vec<Uuid> = parsed_items.iter().map(|(id, _, _)| *id).collect();
        self.publish_menu_availability_if_changed(&ing_ids).await;

        Ok(ReplenishStockResponse {
            success: true,
            message: format!("Stock successfully replenished for PO {}", purchase_id),
        })
    }
}
