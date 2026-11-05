use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

use crate::modules::grpc::{
    CheckStockResponse, DeductStockResponse, OrderItemPayload, ReplenishItemPayload,
    ReplenishStockResponse, RollbackStockResponse, UnavailableItem,
};
use crate::pkg::errors::AppError;
use super::model::{StockAdjustmentDto, StockLog};
use super::repository::StockRepository;

#[derive(Clone)]
pub struct StockService {
    repo: Arc<StockRepository>,
}

impl StockService {
    pub fn new(repo: Arc<StockRepository>) -> Self {
        Self { repo }
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
        self.repo
            .adjust_stock(
                dto.ingredient_id,
                &dto.change_type,
                dto.quantity,
                dto.reference_id.as_deref(),
                dto.notes.as_deref(),
            )
            .await
            .map_err(AppError::Database)
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

        Ok(ReplenishStockResponse {
            success: true,
            message: format!("Stock successfully replenished for PO {}", purchase_id),
        })
    }
}
