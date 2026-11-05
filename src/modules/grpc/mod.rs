pub mod inventory_proto {
    tonic::include_proto!("inventory");
}

pub use inventory_proto::*;

use std::sync::Arc;
use tonic::{Request, Response, Status};
use crate::modules::stock::service::StockService;
use inventory_proto::inventory_service_server::InventoryService;

pub struct InventoryGrpcHandler {
    stock_service: Arc<StockService>,
}

impl InventoryGrpcHandler {
    pub fn new(stock_service: Arc<StockService>) -> Self {
        Self { stock_service }
    }
}

#[tonic::async_trait]
impl InventoryService for InventoryGrpcHandler {
    async fn check_stock_availability(
        &self,
        request: Request<CheckStockRequest>,
    ) -> Result<Response<CheckStockResponse>, Status> {
        let req = request.into_inner();
        let result = self
            .stock_service
            .check_availability(req.items)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(result))
    }

    async fn deduct_stock(
        &self,
        request: Request<DeductStockRequest>,
    ) -> Result<Response<DeductStockResponse>, Status> {
        let req = request.into_inner();
        let result = self
            .stock_service
            .deduct_stock(&req.transaction_id, req.items)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(result))
    }

    async fn rollback_stock(
        &self,
        request: Request<RollbackStockRequest>,
    ) -> Result<Response<RollbackStockResponse>, Status> {
        let req = request.into_inner();
        let result = self
            .stock_service
            .rollback_stock(&req.transaction_id, req.items)
            .await
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(result))
    }
}

