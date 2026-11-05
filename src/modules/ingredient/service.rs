use std::sync::Arc;
use uuid::Uuid;

use crate::pkg::errors::AppError;
use super::model::{CreateIngredientDto, Ingredient, UpdateIngredientDto};
use super::repository::IngredientRepository;

#[derive(Clone)]
pub struct IngredientService {
    repo: Arc<IngredientRepository>,
}

impl IngredientService {
    pub fn new(repo: Arc<IngredientRepository>) -> Self {
        Self { repo }
    }

    pub async fn list(&self) -> Result<Vec<Ingredient>, AppError> {
        self.repo.find_all().await.map_err(AppError::Database)
    }

    pub async fn get(&self, id: Uuid) -> Result<Ingredient, AppError> {
        self.repo
            .find_by_id(id)
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| {
                AppError::not_found(
                    format!("Ingredient with ID {} not found", id),
                    "INGREDIENT_NOT_FOUND",
                )
            })
    }

    pub async fn create(&self, dto: CreateIngredientDto) -> Result<Ingredient, AppError> {
        if let Some(_) = self
            .repo
            .find_by_sku(&dto.sku)
            .await
            .map_err(AppError::Database)?
        {
            return Err(AppError::bad_request(
                format!("Ingredient SKU '{}' already exists", dto.sku),
                "SKU_ALREADY_EXISTS",
            ));
        }

        self.repo.create(&dto).await.map_err(AppError::Database)
    }

    pub async fn update(&self, id: Uuid, dto: UpdateIngredientDto) -> Result<Ingredient, AppError> {
        self.repo
            .update(id, &dto)
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| {
                AppError::not_found(
                    format!("Ingredient with ID {} not found", id),
                    "INGREDIENT_NOT_FOUND",
                )
            })
    }

    pub async fn delete(&self, id: Uuid) -> Result<(), AppError> {
        let deleted = self.repo.delete(id).await.map_err(AppError::Database)?;

        if !deleted {
            return Err(AppError::not_found(
                format!("Ingredient with ID {} not found", id),
                "INGREDIENT_NOT_FOUND",
            ));
        }
        Ok(())
    }
}
