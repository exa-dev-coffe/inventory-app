use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
    routing::{get, post, put},
    Json, Router,
};
use std::sync::Arc;
use uuid::Uuid;

use crate::pkg::{
    auth::jwt::{AdminUser, StaffUser},
    errors::AppError,
    response::BaseResponse,
};
use super::model::{CreateOpnameDto, OpnameQuery, SubmitOpnameCountDto};
use super::service::OpnameService;

pub fn opname_router(service: Arc<OpnameService>) -> Router {
    Router::new()
        .route("/", get(list_opnames).post(create_opname))
        .route("/:id", get(get_opname_detail))
        .route("/:id/counts", put(submit_counts))
        .route("/:id/approve", post(approve_opname))
        .with_state(service)
}

async fn list_opnames(
    _staff: StaffUser,
    State(service): State<Arc<OpnameService>>,
    Query(query): Query<OpnameQuery>,
) -> Result<impl IntoResponse, AppError> {
    let opnames = service.get_opnames(query).await?;
    Ok(BaseResponse::success(opnames, "Stock opnames retrieved successfully"))
}

async fn create_opname(
    staff: StaffUser,
    State(service): State<Arc<OpnameService>>,
    Json(dto): Json<CreateOpnameDto>,
) -> Result<impl IntoResponse, AppError> {
    let detail = service.create_opname(dto, &staff.email).await?;
    Ok(BaseResponse::created(detail, "Stock opname session started successfully"))
}

async fn get_opname_detail(
    _staff: StaffUser,
    State(service): State<Arc<OpnameService>>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let detail = service.get_opname_detail(id).await?;
    Ok(BaseResponse::success(detail, "Stock opname details retrieved"))
}

async fn submit_counts(
    _staff: StaffUser,
    State(service): State<Arc<OpnameService>>,
    Path(id): Path<Uuid>,
    Json(dto): Json<SubmitOpnameCountDto>,
) -> Result<impl IntoResponse, AppError> {
    let detail = service.submit_counts(id, dto).await?;
    Ok(BaseResponse::success(detail, "Physical counts submitted successfully"))
}

async fn approve_opname(
    admin: AdminUser,
    State(service): State<Arc<OpnameService>>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let detail = service.approve_opname(id, &admin.email).await?;
    Ok(BaseResponse::success(detail, "Stock opname approved and inventory synchronized"))
}
