use axum::{
    extract::{Path, State},
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use std::sync::Arc;
use uuid::Uuid;

use crate::pkg::{
    auth::{
        RequireInventoryCreate, RequireInventoryDelete, RequireInventoryEdit, RequireInventoryView,
    },
    response::BaseResponse,
};
use super::model::{CreateIngredientDto, UpdateIngredientDto};
use super::service::IngredientService;

pub fn ingredient_router(service: Arc<IngredientService>) -> Router {
    Router::new()
        .route("/", get(list_ingredients).post(create_ingredient))
        .route(
            "/:id",
            get(get_ingredient)
                .put(update_ingredient)
                .delete(delete_ingredient),
        )
        .with_state(service)
}

async fn list_ingredients(
    _perm: RequireInventoryView,
    State(service): State<Arc<IngredientService>>,
) -> Result<impl IntoResponse, crate::pkg::errors::AppError> {
    let items = service.list().await?;
    Ok(BaseResponse::success(items, "Ingredients retrieved successfully"))
}

async fn get_ingredient(
    _perm: RequireInventoryView,
    State(service): State<Arc<IngredientService>>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, crate::pkg::errors::AppError> {
    let item = service.get(id).await?;
    Ok(BaseResponse::success(item, "Ingredient retrieved successfully"))
}

async fn create_ingredient(
    _perm: RequireInventoryCreate,
    State(service): State<Arc<IngredientService>>,
    Json(dto): Json<CreateIngredientDto>,
) -> Result<impl IntoResponse, crate::pkg::errors::AppError> {
    let item = service.create(dto).await?;
    Ok(BaseResponse::created(item, "Ingredient created successfully"))
}

async fn update_ingredient(
    _perm: RequireInventoryEdit,
    State(service): State<Arc<IngredientService>>,
    Path(id): Path<Uuid>,
    Json(dto): Json<UpdateIngredientDto>,
) -> Result<impl IntoResponse, crate::pkg::errors::AppError> {
    let item = service.update(id, dto).await?;
    Ok(BaseResponse::success(item, "Ingredient updated successfully"))
}

async fn delete_ingredient(
    _perm: RequireInventoryDelete,
    State(service): State<Arc<IngredientService>>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, crate::pkg::errors::AppError> {
    service.delete(id).await?;
    Ok(BaseResponse::success((), "Ingredient deleted successfully"))
}
