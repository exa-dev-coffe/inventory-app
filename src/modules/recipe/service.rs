use std::sync::Arc;
use crate::pkg::errors::AppError;
use super::model::{RecipeWithIngredient, SetRecipeDto};
use super::repository::RecipeRepository;

#[derive(Clone)]
pub struct RecipeService {
    repo: Arc<RecipeRepository>,
}

impl RecipeService {
    pub fn new(repo: Arc<RecipeRepository>) -> Self {
        Self { repo }
    }

    pub async fn get_by_menu_id(&self, menu_id: i64) -> Result<Vec<RecipeWithIngredient>, AppError> {
        self.repo
            .find_by_menu_id(menu_id)
            .await
            .map_err(AppError::Database)
    }

    pub async fn set_recipe(&self, dto: SetRecipeDto) -> Result<Vec<RecipeWithIngredient>, AppError> {
        self.repo
            .set_recipe(dto.menu_id, &dto.ingredients)
            .await
            .map_err(AppError::Database)
    }

    pub async fn delete_recipe(&self, menu_id: i64) -> Result<(), AppError> {
        let deleted = self
            .repo
            .delete_recipe(menu_id)
            .await
            .map_err(AppError::Database)?;

        if !deleted {
            return Err(AppError::not_found(
                format!("Recipe for menu ID {} not found", menu_id),
                "RECIPE_NOT_FOUND",
            ));
        }
        Ok(())
    }
}
