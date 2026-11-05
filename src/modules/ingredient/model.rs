use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Ingredient {
    pub id: Uuid,
    pub sku: String,
    pub name: String,
    pub category: String,
    pub unit: String,
    pub current_stock: Decimal,
    pub min_stock_alert: Decimal,
    pub cost_per_unit: Decimal,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateIngredientDto {
    pub sku: String,
    pub name: String,
    pub category: String,
    pub unit: String,
    pub initial_stock: Option<Decimal>,
    pub min_stock_alert: Option<Decimal>,
    pub cost_per_unit: Option<Decimal>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateIngredientDto {
    pub name: Option<String>,
    pub category: Option<String>,
    pub unit: Option<String>,
    pub min_stock_alert: Option<Decimal>,
    pub cost_per_unit: Option<Decimal>,
    pub is_active: Option<bool>,
}

