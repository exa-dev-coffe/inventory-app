use sqlx::{PgPool, Result};
use super::model::{RecipeIngredientItemDto, RecipeWithIngredient};

#[derive(Clone)]
pub struct RecipeRepository {
    pool: PgPool,
}

impl RecipeRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn find_by_menu_id(&self, menu_id: i64) -> Result<Vec<RecipeWithIngredient>> {
        sqlx::query_as::<_, RecipeWithIngredient>(
            "SELECT r.id, r.menu_id, r.ingredient_id, i.name as ingredient_name, i.sku as ingredient_sku, i.unit as ingredient_unit, r.amount
             FROM recipes r
             JOIN ingredients i ON r.ingredient_id = i.id
             WHERE r.menu_id = $1"
        )
        .bind(menu_id)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn set_recipe(
        &self,
        menu_id: i64,
        items: &[RecipeIngredientItemDto],
    ) -> Result<Vec<RecipeWithIngredient>> {
        let mut tx = self.pool.begin().await?;

        // 1. Clear old recipe for this menu
        sqlx::query("DELETE FROM recipes WHERE menu_id = $1")
            .bind(menu_id)
            .execute(&mut *tx)
            .await?;

        // 2. Insert new ingredients
        for item in items {
            sqlx::query(
                "INSERT INTO recipes (menu_id, ingredient_id, amount)
                 VALUES ($1, $2, $3)"
            )
            .bind(menu_id)
            .bind(item.ingredient_id)
            .bind(item.amount)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;

        self.find_by_menu_id(menu_id).await
    }

    pub async fn delete_recipe(&self, menu_id: i64) -> Result<bool> {
        let res = sqlx::query("DELETE FROM recipes WHERE menu_id = $1")
            .bind(menu_id)
            .execute(&self.pool)
            .await?;
        Ok(res.rows_affected() > 0)
    }
}

