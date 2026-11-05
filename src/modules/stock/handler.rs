use axum::{
    extract::{Query, State},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use std::sync::Arc;

use uuid::Uuid;

use crate::pkg::{
    auth::{RequireInventoryEdit, RequireInventoryView},
    response::BaseResponse,
};
use super::model::StockAdjustmentDto;
use super::service::StockService;

#[derive(Debug, Deserialize)]
pub struct LogsQuery {
    pub ingredient_id: Option<Uuid>,
    pub change_type: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

pub fn stock_router(service: Arc<StockService>) -> Router {
    Router::new()
        .route("/adjust", post(adjust_stock))
        .route("/logs", get(get_logs))
        .with_state(service)
}

async fn adjust_stock(
    _perm: RequireInventoryEdit,
    State(service): State<Arc<StockService>>,
    Json(dto): Json<StockAdjustmentDto>,
) -> Result<impl IntoResponse, crate::pkg::errors::AppError> {
    let log = service.adjust_stock(dto).await?;
    Ok(BaseResponse::success(log, "Stock adjusted successfully"))
}

async fn get_logs(
    _perm: RequireInventoryView,
    State(service): State<Arc<StockService>>,
    Query(query): Query<LogsQuery>,
) -> Result<impl IntoResponse, crate::pkg::errors::AppError> {
    let logs = service
        .get_logs(
            query.ingredient_id,
            query.change_type.as_deref(),
            query.limit,
            query.offset,
        )
        .await?;
    Ok(BaseResponse::success(logs, "Stock logs retrieved successfully"))
}
