use chrono::Utc;
use rand::Rng;
use std::sync::Arc;
use uuid::Uuid;

use super::model::{
    CreateOpnameDto, OpnameQuery, StockOpname, StockOpnameDetail, SubmitOpnameCountDto,
};
use super::repository::OpnameRepository;
use crate::pkg::errors::AppError;

#[derive(Clone)]
pub struct OpnameService {
    repo: Arc<OpnameRepository>,
}

impl OpnameService {
    pub fn new(repo: Arc<OpnameRepository>) -> Self {
        Self { repo }
    }

    fn generate_opname_number() -> String {
        let now = Utc::now();
        let rand_suffix: u32 = rand::thread_rng().gen_range(1000..9999);
        format!("OPN-{}-{:04}", now.format("%Y%m%d"), rand_suffix)
    }

    pub async fn create_opname(
        &self,
        dto: CreateOpnameDto,
        created_by: &str,
    ) -> Result<StockOpnameDetail, AppError> {
        let opname_number = Self::generate_opname_number();
        self.repo
            .create_opname(
                &opname_number,
                dto.notes.as_deref(),
                created_by,
                dto.ingredient_ids.as_deref(),
            )
            .await
            .map_err(AppError::Database)
    }

    pub async fn get_opnames(&self, query: OpnameQuery) -> Result<Vec<StockOpname>, AppError> {
        self.repo
            .get_opnames(query)
            .await
            .map_err(AppError::Database)
    }

    pub async fn get_opname_detail(&self, id: Uuid) -> Result<StockOpnameDetail, AppError> {
        self.repo
            .get_opname_detail(id)
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::not_found("Stock opname session not found", "OPNAME_NOT_FOUND"))
    }

    pub async fn submit_counts(
        &self,
        id: Uuid,
        dto: SubmitOpnameCountDto,
    ) -> Result<StockOpnameDetail, AppError> {
        self.repo
            .submit_counts(id, &dto.items)
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| {
                AppError::bad_request(
                    "Cannot submit counts for non-existent or already finalized opname",
                    "INVALID_OPNAME_STATUS",
                )
            })
    }

    pub async fn approve_opname(
        &self,
        id: Uuid,
        approved_by: &str,
    ) -> Result<StockOpnameDetail, AppError> {
        self.repo
            .approve_opname(id, approved_by)
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| {
                AppError::bad_request(
                    "Cannot approve non-existent or already finalized opname",
                    "INVALID_OPNAME_STATUS",
                )
            })
    }
}
