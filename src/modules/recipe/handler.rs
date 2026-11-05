use axum::{
    extract::{Path, State},
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use std::sync::Arc;

use crate::pkg::response::BaseResponse;
use super::model::SetRecipeDto;
use super::service::RecipeService;

pub fn recipe_router(service: Arc<RecipeService>) -> Router {
    Router::new()
        .route(
            "/:menu_id",
            get(get_recipe).post(set_recipe).delete(delete_recipe),
        )
        .with_state(service)
}

async fn get_recipe(
    State(service): State<Arc<RecipeService>>,
    Path(menu_id): Path<i64>,
) -> Result<impl IntoResponse, crate::pkg::errors::AppError> {
    let items = service.get_by_menu_id(menu_id).await?;
    Ok(BaseResponse::success(items, "Recipe retrieved successfully"))
}

async fn set_recipe(
    State(service): State<Arc<RecipeService>>,
    Path(menu_id): Path<i64>,
    Json(mut dto): Json<SetRecipeDto>,
) -> Result<impl IntoResponse, crate::pkg::errors::AppError> {
    dto.menu_id = menu_id;
    let items = service.set_recipe(dto).await?;
    Ok(BaseResponse::success(items, "Recipe saved successfully"))
}

async fn delete_recipe(
    State(service): State<Arc<RecipeService>>,
    Path(menu_id): Path<i64>,
) -> Result<impl IntoResponse, crate::pkg::errors::AppError> {
    service.delete_recipe(menu_id).await?;
    Ok(BaseResponse::success((), "Recipe deleted successfully"))
}

