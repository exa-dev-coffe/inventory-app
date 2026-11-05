use rust_decimal::Decimal;
use sqlx::{PgPool, Result};
use uuid::Uuid;

use super::model::{
    OpnameQuery, PhysicalCountItemDto, StockOpname, StockOpnameDetail, StockOpnameItem,
};

#[derive(Clone)]
pub struct OpnameRepository {
    pool: PgPool,
}

impl OpnameRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create_opname(
        &self,
        opname_number: &str,
        notes: Option<&str>,
        created_by: &str,
        ingredient_ids: Option<&[Uuid]>,
    ) -> Result<StockOpnameDetail> {
        let mut tx = self.pool.begin().await?;

        // 1. Insert header
        let opname: StockOpname = sqlx::query_as(
            "INSERT INTO stock_opnames (opname_number, notes, created_by, status)
             VALUES ($1, $2, $3, 'DRAFT')
             RETURNING id, opname_number, opname_date, status, total_loss_value, notes, created_by, approved_by, created_at, updated_at",
        )
        .bind(opname_number)
        .bind(notes)
        .bind(created_by)
        .fetch_one(&mut *tx)
        .await?;

        // 2. Fetch target ingredients to snapshot
        let ingredients: Vec<(Uuid, String, String, Decimal, Decimal)> = match ingredient_ids {
            Some(ids) if !ids.is_empty() => {
                sqlx::query_as(
                    "SELECT id, name, unit, current_stock, cost_per_unit
                     FROM ingredients
                     WHERE id = ANY($1) AND is_active = true",
                )
                .bind(ids)
                .fetch_all(&mut *tx)
                .await?
            }
            _ => {
                sqlx::query_as(
                    "SELECT id, name, unit, current_stock, cost_per_unit
                     FROM ingredients
                     WHERE is_active = true
                     ORDER BY name ASC",
                )
                .fetch_all(&mut *tx)
                .await?
            }
        };

        let mut items = Vec::new();
        for (ing_id, ing_name, unit, stock, cost) in ingredients {
            let item: StockOpnameItem = sqlx::query_as(
                "INSERT INTO stock_opname_items (opname_id, ingredient_id, system_stock, physical_stock, difference, unit_cost, loss_value)
                 VALUES ($1, $2, $3, $3, 0, $4, 0)
                 RETURNING id, opname_id, ingredient_id, $5 as ingredient_name, $6 as unit, system_stock, physical_stock, difference, unit_cost, loss_value, notes, created_at",
            )
            .bind(opname.id)
            .bind(ing_id)
            .bind(stock)
            .bind(cost)
            .bind(&ing_name)
            .bind(&unit)
            .fetch_one(&mut *tx)
            .await?;

            items.push(item);
        }

        tx.commit().await?;

        Ok(StockOpnameDetail { opname, items })
    }

    pub async fn get_opnames(&self, query: OpnameQuery) -> Result<Vec<StockOpname>> {
        let limit = query.limit.unwrap_or(20);
        let offset = query.offset.unwrap_or(0);

        if let Some(status) = query.status {
            sqlx::query_as(
                "SELECT id, opname_number, opname_date, status, total_loss_value, notes, created_by, approved_by, created_at, updated_at
                 FROM stock_opnames
                 WHERE status = $1
                 ORDER BY created_at DESC
                 LIMIT $2 OFFSET $3",
            )
            .bind(status)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
        } else {
            sqlx::query_as(
                "SELECT id, opname_number, opname_date, status, total_loss_value, notes, created_by, approved_by, created_at, updated_at
                 FROM stock_opnames
                 ORDER BY created_at DESC
                 LIMIT $1 OFFSET $2",
            )
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
        }
    }

    pub async fn get_opname_detail(&self, id: Uuid) -> Result<Option<StockOpnameDetail>> {
        let opname: Option<StockOpname> = sqlx::query_as(
            "SELECT id, opname_number, opname_date, status, total_loss_value, notes, created_by, approved_by, created_at, updated_at
             FROM stock_opnames
             WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        let opname = match opname {
            Some(o) => o,
            None => return Ok(None),
        };

        let items: Vec<StockOpnameItem> = sqlx::query_as(
            "SELECT oi.id, oi.opname_id, oi.ingredient_id, i.name as ingredient_name, i.unit, oi.system_stock, oi.physical_stock, oi.difference, oi.unit_cost, oi.loss_value, oi.notes, oi.created_at
             FROM stock_opname_items oi
             JOIN ingredients i ON oi.ingredient_id = i.id
             WHERE oi.opname_id = $1
             ORDER BY i.name ASC",
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await?;

        Ok(Some(StockOpnameDetail { opname, items }))
    }

    pub async fn submit_counts(
        &self,
        opname_id: Uuid,
        items: &[PhysicalCountItemDto],
    ) -> Result<Option<StockOpnameDetail>> {
        let mut tx = self.pool.begin().await?;

        for item in items {
            // Update individual item counts & difference
            sqlx::query(
                "UPDATE stock_opname_items
                 SET physical_stock = $1,
                     difference = $1 - system_stock,
                     loss_value = CASE WHEN ($1 - system_stock) < 0 THEN (system_stock - $1) * unit_cost ELSE 0 END,
                     notes = COALESCE($2, notes)
                 WHERE opname_id = $3 AND ingredient_id = $4",
            )
            .bind(item.physical_stock)
            .bind(item.notes.as_deref())
            .bind(opname_id)
            .bind(item.ingredient_id)
            .execute(&mut *tx)
            .await?;
        }

        // Calculate total loss
        let total_loss: (Decimal,) = sqlx::query_as(
            "SELECT COALESCE(SUM(loss_value), 0)
             FROM stock_opname_items
             WHERE opname_id = $1",
        )
        .bind(opname_id)
        .fetch_one(&mut *tx)
        .await?;

        // Update header to SUBMITTED
        sqlx::query(
            "UPDATE stock_opnames
             SET status = 'SUBMITTED', total_loss_value = $1, updated_at = NOW()
             WHERE id = $2 AND status IN ('DRAFT', 'SUBMITTED')",
        )
        .bind(total_loss.0)
        .bind(opname_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        self.get_opname_detail(opname_id).await
    }

    pub async fn approve_opname(
        &self,
        opname_id: Uuid,
        approved_by: &str,
    ) -> Result<Option<StockOpnameDetail>> {
        let mut tx = self.pool.begin().await?;

        let opname: Option<StockOpname> = sqlx::query_as(
            "SELECT id, opname_number, opname_date, status, total_loss_value, notes, created_by, approved_by, created_at, updated_at
             FROM stock_opnames
             WHERE id = $1
             FOR UPDATE",
        )
        .bind(opname_id)
        .fetch_optional(&mut *tx)
        .await?;

        let opname = match opname {
            Some(o) if o.status != "APPROVED" && o.status != "REJECTED" => o,
            _ => return Ok(None),
        };

        // Fetch all items for adjustment
        let items: Vec<StockOpnameItem> = sqlx::query_as(
            "SELECT oi.id, oi.opname_id, oi.ingredient_id, i.name as ingredient_name, i.unit, oi.system_stock, oi.physical_stock, oi.difference, oi.unit_cost, oi.loss_value, oi.notes, oi.created_at
             FROM stock_opname_items oi
             JOIN ingredients i ON oi.ingredient_id = i.id
             WHERE oi.opname_id = $1",
        )
        .bind(opname_id)
        .fetch_all(&mut *tx)
        .await?;

        for item in &items {
            if item.difference != Decimal::ZERO {
                // Update ingredient current_stock to physical_stock
                sqlx::query(
                    "UPDATE ingredients
                     SET current_stock = $1, updated_at = NOW()
                     WHERE id = $2",
                )
                .bind(item.physical_stock)
                .bind(item.ingredient_id)
                .execute(&mut *tx)
                .await?;

                // Insert into stock_logs
                let note = format!(
                    "Opname {} adjustment (difference: {}, notes: {})",
                    opname.opname_number,
                    item.difference,
                    item.notes.as_deref().unwrap_or("-")
                );

                sqlx::query(
                    "INSERT INTO stock_logs (ingredient_id, change_type, quantity, balance_after, reference_id, notes)
                     VALUES ($1, 'ADJUSTMENT_OPNAME', $2, $3, $4, $5)",
                )
                .bind(item.ingredient_id)
                .bind(item.difference)
                .bind(item.physical_stock)
                .bind(&opname.opname_number)
                .bind(note)
                .execute(&mut *tx)
                .await?;
            }
        }

        // Set status to APPROVED
        sqlx::query(
            "UPDATE stock_opnames
             SET status = 'APPROVED', approved_by = $1, updated_at = NOW()
             WHERE id = $2",
        )
        .bind(approved_by)
        .bind(opname_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        self.get_opname_detail(opname_id).await
    }
}
