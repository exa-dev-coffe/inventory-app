use rust_decimal::Decimal;
use sqlx::{PgPool, Result};
use uuid::Uuid;

use super::model::{CreateIngredientDto, Ingredient, UpdateIngredientDto};

#[derive(Clone)]
pub struct IngredientRepository {
    pool: PgPool,
}

impl IngredientRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn find_all(&self) -> Result<Vec<Ingredient>> {
        sqlx::query_as::<_, Ingredient>(
            "SELECT id, sku, name, category, unit, current_stock, min_stock_alert, cost_per_unit, is_active, created_at, updated_at
             FROM ingredients ORDER BY name ASC"
        )
        .fetch_all(&self.pool)
        .await
    }

    pub async fn find_by_id(&self, id: Uuid) -> Result<Option<Ingredient>> {
        sqlx::query_as::<_, Ingredient>(
            "SELECT id, sku, name, category, unit, current_stock, min_stock_alert, cost_per_unit, is_active, created_at, updated_at
             FROM ingredients WHERE id = $1"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn find_by_sku(&self, sku: &str) -> Result<Option<Ingredient>> {
        sqlx::query_as::<_, Ingredient>(
            "SELECT id, sku, name, category, unit, current_stock, min_stock_alert, cost_per_unit, is_active, created_at, updated_at
             FROM ingredients WHERE sku = $1"
        )
        .bind(sku)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn create(&self, dto: &CreateIngredientDto) -> Result<Ingredient> {
        let initial_stock = dto.initial_stock.unwrap_or(Decimal::ZERO);
        let min_stock = dto.min_stock_alert.unwrap_or(Decimal::from(10));
        let cost = dto.cost_per_unit.unwrap_or(Decimal::ZERO);

        sqlx::query_as::<_, Ingredient>(
            "INSERT INTO ingredients (sku, name, category, unit, current_stock, min_stock_alert, cost_per_unit)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             RETURNING id, sku, name, category, unit, current_stock, min_stock_alert, cost_per_unit, is_active, created_at, updated_at"
        )
        .bind(&dto.sku)
        .bind(&dto.name)
        .bind(&dto.category)
        .bind(&dto.unit)
        .bind(initial_stock)
        .bind(min_stock)
        .bind(cost)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn update(&self, id: Uuid, dto: &UpdateIngredientDto) -> Result<Option<Ingredient>> {
        let existing = self.find_by_id(id).await?;
        let current = match existing {
            Some(i) => i,
            None => return Ok(None),
        };

        let name = dto.name.as_deref().unwrap_or(&current.name);
        let category = dto.category.as_deref().unwrap_or(&current.category);
        let unit = dto.unit.as_deref().unwrap_or(&current.unit);
        let min_stock = dto.min_stock_alert.unwrap_or(current.min_stock_alert);
        let cost = dto.cost_per_unit.unwrap_or(current.cost_per_unit);
        let is_active = dto.is_active.unwrap_or(current.is_active);

        let updated = sqlx::query_as::<_, Ingredient>(
            "UPDATE ingredients
             SET name = $1, category = $2, unit = $3, min_stock_alert = $4, cost_per_unit = $5, is_active = $6, updated_at = NOW()
             WHERE id = $7
             RETURNING id, sku, name, category, unit, current_stock, min_stock_alert, cost_per_unit, is_active, created_at, updated_at"
        )
        .bind(name)
        .bind(category)
        .bind(unit)
        .bind(min_stock)
        .bind(cost)
        .bind(is_active)
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(updated)
    }

    pub async fn delete(&self, id: Uuid) -> Result<bool> {
        // Check if ingredient is linked in recipes
        let recipe_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM recipes WHERE ingredient_id = $1")
            .bind(id)
            .fetch_one(&self.pool)
            .await?;

        if recipe_count.0 > 0 {
            // Soft-deactivate to preserve recipe integrity and prevent foreign key violations
            let res = sqlx::query("UPDATE ingredients SET is_active = false, updated_at = NOW() WHERE id = $1")
                .bind(id)
                .execute(&self.pool)
                .await?;
            return Ok(res.rows_affected() > 0);
        }

        let res = sqlx::query("DELETE FROM ingredients WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(res.rows_affected() > 0)
    }
}

