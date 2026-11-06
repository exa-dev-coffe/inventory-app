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
    pub quantity_required: Decimal,
    pub unit: String,
    #[serde(default)]
    pub cost_per_unit: Decimal,
    #[serde(default)]
    pub subtotal_cost: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct MenuCostSummary {
    pub menu_id: i64,
    pub total_cogs: Decimal,
    pub item_count: i64,
}

#[derive(Debug, Deserialize)]
pub struct SetRecipeDto {
    #[serde(default)]
    pub menu_id: i64,
    #[serde(alias = "items")]
    pub ingredients: Vec<RecipeIngredientItemDto>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RecipeIngredientItemDto {
    pub ingredient_id: Uuid,
    #[serde(alias = "quantity_required")]
    pub amount: Decimal,
}

