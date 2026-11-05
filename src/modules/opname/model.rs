use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct StockOpname {
    pub id: Uuid,
    pub opname_number: String,
    pub opname_date: DateTime<Utc>,
    pub status: String,
    pub total_loss_value: Decimal,
    pub notes: Option<String>,
    pub created_by: String,
    pub approved_by: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct StockOpnameItem {
    pub id: Uuid,
    pub opname_id: Uuid,
    pub ingredient_id: Uuid,
    pub ingredient_name: Option<String>,
    pub unit: Option<String>,
    pub system_stock: Decimal,
    pub physical_stock: Decimal,
    pub difference: Decimal,
    pub unit_cost: Decimal,
    pub loss_value: Decimal,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockOpnameDetail {
    #[serde(flatten)]
    pub opname: StockOpname,
    pub items: Vec<StockOpnameItem>,
}

#[derive(Debug, Deserialize)]
pub struct CreateOpnameDto {
    pub notes: Option<String>,
    pub ingredient_ids: Option<Vec<Uuid>>,
}

#[derive(Debug, Deserialize)]
pub struct PhysicalCountItemDto {
    pub ingredient_id: Uuid,
    pub physical_stock: Decimal,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SubmitOpnameCountDto {
    pub items: Vec<PhysicalCountItemDto>,
}

#[derive(Debug, Deserialize)]
pub struct OpnameQuery {
    pub status: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}
