use axum::{
    extract::State,
    middleware,
    response::IntoResponse,
    routing::post,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::modules::grpc::OrderItemPayload;
use crate::pkg::{auth::hmac::validate_hmac_middleware, errors::AppError, response::BaseResponse};
use super::service::StockService;

#[derive(Debug, Deserialize, Serialize)]
pub struct OrderItemDto {
    pub menu_id: i64,
    pub quantity: i32,
}

#[derive(Debug, Deserialize)]
pub struct CheckStockDto {
    pub items: Vec<OrderItemDto>,
}

#[derive(Debug, Deserialize)]
pub struct DeductStockDto {
    pub transaction_id: String,
    pub items: Vec<OrderItemDto>,
}

#[derive(Debug, Deserialize)]
pub struct RollbackStockDto {
    pub transaction_id: String,
    pub items: Vec<OrderItemDto>,
}

pub fn internal_stock_router(service: Arc<StockService>) -> Router {
    Router::new()
        .route("/check", post(check_stock))
        .route("/deduct", post(deduct_stock))
        .route("/rollback", post(rollback_stock))
        .route_layer(middleware::from_fn(validate_hmac_middleware))
        .with_state(service)
}

async fn check_stock(
    State(service): State<Arc<StockService>>,
    Json(dto): Json<CheckStockDto>,
) -> Result<impl IntoResponse, AppError> {
    let items = dto
        .items
        .into_iter()
        .map(|i| OrderItemPayload {
            menu_id: i.menu_id,
            quantity: i.quantity,
        })
        .collect();

    let res = service.check_availability(items).await?;
    Ok(BaseResponse::success(res, "Stock checked successfully"))
}

async fn deduct_stock(
    State(service): State<Arc<StockService>>,
    Json(dto): Json<DeductStockDto>,
) -> Result<impl IntoResponse, AppError> {
    let items = dto
        .items
        .into_iter()
        .map(|i| OrderItemPayload {
            menu_id: i.menu_id,
            quantity: i.quantity,
        })
        .collect();

    let res = service.deduct_stock(&dto.transaction_id, items).await?;
    Ok(BaseResponse::success(res, "Stock deducted successfully"))
}

async fn rollback_stock(
    State(service): State<Arc<StockService>>,
    Json(dto): Json<RollbackStockDto>,
) -> Result<impl IntoResponse, AppError> {
    let items = dto
        .items
        .into_iter()
        .map(|i| OrderItemPayload {
            menu_id: i.menu_id,
            quantity: i.quantity,
        })
        .collect();

    let res = service.rollback_stock(&dto.transaction_id, items).await?;
    Ok(BaseResponse::success(res, "Stock rollback completed"))
}
