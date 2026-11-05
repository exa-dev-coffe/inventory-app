use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct StockLog {
    pub id: Uuid,
    pub ingredient_id: Uuid,
    pub ingredient_name: Option<String>,
    pub change_type: String,
    pub quantity: Decimal,
    pub balance_after: Decimal,
    pub reference_id: Option<String>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct StockAdjustmentDto {
    pub ingredient_id: Uuid,
    pub change_type: String, // 'IN_PURCHASE', 'OUT_WASTE', 'ADJUSTMENT_OPNAME'
    pub quantity: Decimal,
    pub reference_id: Option<String>,
    pub notes: Option<String>,
}

