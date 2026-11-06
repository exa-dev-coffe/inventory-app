use rust_decimal::Decimal;
use sqlx::{PgPool, Result};
use uuid::Uuid;

use super::model::StockLog;

#[derive(Clone)]
pub struct StockRepository {
    pool: PgPool,
}

#[derive(Debug, sqlx::FromRow)]
pub struct IngredientRequirement {
    pub menu_id: i64,
    pub ingredient_id: Uuid,
    pub ingredient_name: String,
    pub amount_per_unit: Decimal,
    pub current_stock: Decimal,
}

impl StockRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn get_logs(
        &self,
        ingredient_id: Option<Uuid>,
        change_type: Option<&str>,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<StockLog>> {
        sqlx::query_as::<_, StockLog>(
            "SELECT l.id, l.ingredient_id, i.name as ingredient_name, l.change_type, l.quantity, l.balance_after, l.reference_id, l.notes, l.created_at
             FROM stock_logs l
             JOIN ingredients i ON l.ingredient_id = i.id
             WHERE ($3::UUID IS NULL OR l.ingredient_id = $3)
               AND ($4::TEXT IS NULL OR l.change_type = $4)
             ORDER BY l.created_at DESC
             LIMIT $1 OFFSET $2",
        )
        .bind(limit)
        .bind(offset)
        .bind(ingredient_id)
        .bind(change_type)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn get_ingredient_cost_info(&self, ingredient_id: Uuid) -> Result<Option<(String, Decimal)>> {
        sqlx::query_as::<_, (String, Decimal)>(
            "SELECT name, cost_per_unit FROM ingredients WHERE id = $1"
        )
        .bind(ingredient_id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn adjust_stock(
        &self,
        ingredient_id: Uuid,
        change_type: &str,
        quantity: Decimal,
        reference_id: Option<&str>,
        notes: Option<&str>,
    ) -> Result<StockLog> {
        let mut tx = self.pool.begin().await?;

        // 1. Lock ingredient row and update current_stock
        let updated = sqlx::query_as::<_, (Decimal,)>(
            "UPDATE ingredients
             SET current_stock = current_stock + $1, updated_at = NOW()
             WHERE id = $2
             RETURNING current_stock"
        )
        .bind(quantity)
        .bind(ingredient_id)
        .fetch_one(&mut *tx)
        .await?;

        let balance_after = updated.0;

        // 2. Insert into stock_logs
        let log = sqlx::query_as::<_, StockLog>(
            "INSERT INTO stock_logs (ingredient_id, change_type, quantity, balance_after, reference_id, notes)
             VALUES ($1, $2, $3, $4, $5, $6)
             RETURNING id, ingredient_id, NULL as ingredient_name, change_type, quantity, balance_after, reference_id, notes, created_at"
        )
        .bind(ingredient_id)
        .bind(change_type)
        .bind(quantity)
        .bind(balance_after)
        .bind(reference_id)
        .bind(notes)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(log)
    }

    pub async fn get_requirements_for_menus(&self, menu_ids: &[i64]) -> Result<Vec<IngredientRequirement>> {
        sqlx::query_as::<_, IngredientRequirement>(
            "SELECT r.menu_id, r.ingredient_id, i.name as ingredient_name, r.amount as amount_per_unit, i.current_stock
             FROM recipes r
             JOIN ingredients i ON r.ingredient_id = i.id
             WHERE r.menu_id = ANY($1) AND i.is_active = true"
        )
        .bind(menu_ids)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn deduct_batch(
        &self,
        transaction_id: &str,
        deductions: &[(Uuid, Decimal)], // (ingredient_id, quantity)
    ) -> Result<bool> {
        let mut tx = self.pool.begin().await?;

        for &(ingredient_id, qty) in deductions {
            // Atomic update: only decrement if stock is sufficient
            let res = sqlx::query(
                "UPDATE ingredients
                 SET current_stock = current_stock - $1, updated_at = NOW()
                 WHERE id = $2 AND current_stock >= $1"
            )
            .bind(qty)
            .bind(ingredient_id)
            .execute(&mut *tx)
            .await?;

            if res.rows_affected() == 0 {
                tx.rollback().await?;
                return Ok(false);
            }

            // Get current balance after deduction
            let bal: (Decimal,) = sqlx::query_as("SELECT current_stock FROM ingredients WHERE id = $1")
                .bind(ingredient_id)
                .fetch_one(&mut *tx)
                .await?;

            // Insert log
            sqlx::query(
                "INSERT INTO stock_logs (ingredient_id, change_type, quantity, balance_after, reference_id, notes)
                 VALUES ($1, 'OUT_SALE', $2, $3, $4, 'Automated POS Order Deduction')"
            )
            .bind(ingredient_id)
            .bind(-qty)
            .bind(bal.0)
            .bind(transaction_id)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(true)
    }

    pub async fn rollback_batch(
        &self,
        transaction_id: &str,
        restorations: &[(Uuid, Decimal)], // (ingredient_id, quantity)
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        for &(ingredient_id, qty) in restorations {
            let res: (Decimal,) = sqlx::query_as(
                "UPDATE ingredients
                 SET current_stock = current_stock + $1, updated_at = NOW()
                 WHERE id = $2
                 RETURNING current_stock"
            )
            .bind(qty)
            .bind(ingredient_id)
            .fetch_one(&mut *tx)
            .await?;

            sqlx::query(
                "INSERT INTO stock_logs (ingredient_id, change_type, quantity, balance_after, reference_id, notes)
                 VALUES ($1, 'IN_ROLLBACK', $2, $3, $4, 'Automated POS Order Rollback')"
            )
            .bind(ingredient_id)
            .bind(qty)
            .bind(res.0)
            .bind(transaction_id)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    pub async fn replenish_batch(
        &self,
        purchase_id: &str,
        items: &[(Uuid, Decimal, Decimal)], // (ingredient_id, quantity, unit_cost)
    ) -> Result<bool> {
        let mut tx = self.pool.begin().await?;

        for &(ingredient_id, qty, unit_cost) in items {
            let current: (Decimal, Decimal) = sqlx::query_as(
                "SELECT current_stock, cost_per_unit
                 FROM ingredients
                 WHERE id = $1
                 FOR UPDATE",
            )
            .bind(ingredient_id)
            .fetch_one(&mut *tx)
            .await?;

            let current_stock = current.0;
            let current_cost = current.1;
            let new_stock = current_stock + qty;

            // Weighted Moving Average: ((current_stock * current_cost) + (qty * unit_cost)) / (current_stock + qty)
            let new_cost = if new_stock > Decimal::ZERO {
                let total_value =
                    (current_stock.max(Decimal::ZERO) * current_cost) + (qty * unit_cost);
                total_value / new_stock
            } else {
                unit_cost
            };

            sqlx::query(
                "UPDATE ingredients
                 SET current_stock = $1, cost_per_unit = $2, updated_at = NOW()
                 WHERE id = $3",
            )
            .bind(new_stock)
            .bind(new_cost)
            .bind(ingredient_id)
            .execute(&mut *tx)
            .await?;

            let note = format!("Restocked via PO {}, unit cost: {}", purchase_id, unit_cost);
            sqlx::query(
                "INSERT INTO stock_logs (ingredient_id, change_type, quantity, balance_after, reference_id, notes)
                 VALUES ($1, 'IN_PURCHASE', $2, $3, $4, $5)",
            )
            .bind(ingredient_id)
            .bind(qty)
            .bind(new_stock)
            .bind(purchase_id)
            .bind(note)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(true)
    }

    pub async fn get_menus_using_ingredients(&self, ingredient_ids: &[Uuid]) -> Result<Vec<i64>> {
        if ingredient_ids.is_empty() {
            return Ok(vec![]);
        }
        let rows = sqlx::query_as::<_, (i64,)>(
            "SELECT DISTINCT menu_id FROM recipes WHERE ingredient_id = ANY($1)",
        )
        .bind(ingredient_ids)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|r| r.0).collect())
    }

    pub async fn check_menu_availability(&self, menu_id: i64) -> Result<bool> {
        let unavailable_count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*)
             FROM recipes r
             JOIN ingredients i ON r.ingredient_id = i.id
             WHERE r.menu_id = $1
               AND (i.is_active = false OR i.current_stock < r.amount)",
        )
        .bind(menu_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(unavailable_count == 0)
    }
}

