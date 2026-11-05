use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RecipeItem {
    pub id: Uuid,
    pub menu_id: i64,
    pub ingredient_id: Uuid,
    pub amount: Decimal,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct RecipeWithIngredient {
    pub id: Uuid,
    pub menu_id: i64,
    pub ingredient_id: Uuid,
    pub ingredient_name: String,
    pub ingredient_sku: String,
    pub ingredient_unit: String,
    pub amount: Decimal,
}

#[derive(Debug, Deserialize)]
pub struct SetRecipeDto {
    pub menu_id: i64,
    pub ingredients: Vec<RecipeIngredientItemDto>,
}

#[derive(Debug, Deserialize)]
pub struct RecipeIngredientItemDto {
    pub ingredient_id: Uuid,
    pub amount: Decimal,
}

