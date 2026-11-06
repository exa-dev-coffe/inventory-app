pub mod config;
pub mod constants;
pub mod database;
pub mod modules;
pub mod pkg;

use axum::{extract::State, response::IntoResponse, routing::get, Json, Router};
use chrono::Utc;
use serde_json::{json, Value};
use sqlx::PgPool;
use std::{sync::Arc, time::Duration};
use tower_http::cors::CorsLayer;

use crate::{
    config::AppConfig,
    modules::{
        ingredient::{
            handler::ingredient_router, repository::IngredientRepository,
            service::IngredientService,
        },
        opname::{handler::opname_router, repository::OpnameRepository, service::OpnameService},
        recipe::{
            handler::recipe_router, repository::RecipeRepository, service::RecipeService,
        },
        realtime::realtime_router,
        stock::{
            handler::stock_router, internal_handler::internal_stock_router,
            repository::StockRepository, service::StockService,
        },
        user::{handler::user_router, service::UserService},
    },
    pkg::{
        auth::jwt::{AuthenticatedUser, OptionalAuthUser},
        idempotency::{idempotency_layer, IdempotencyStore},
        logging::middleware::create_trace_layer,
        realtime::RealtimeHub,
        response::BaseResponse,
    },
};

#[derive(Clone)]
pub struct AppState {
    pub config: AppConfig,
    pub pool: PgPool,
    pub hub: Arc<RealtimeHub>,
}

pub async fn root_handler() -> impl IntoResponse {
    BaseResponse::success(
        json!({
            "service": "inventory-app",
            "version": "0.1.0",
            "status": "running",
            "time": Utc::now().to_rfc3339()
        }),
        "Welcome to Coffe Inventory & Recipe (BOM) Microservice",
    )
}

pub async fn health_check_handler(State(state): State<AppState>) -> Json<Value> {
    let mut is_healthy = true;

    // 1. Database check
    let db_status = match sqlx::query("SELECT 1").execute(&state.pool).await {
        Ok(_) => "connected",
        Err(_) => {
            is_healthy = false;
            "error"
        }
    };

    // 2. Redis check
    let redis_status = if state.config.redis_enabled {
        match crate::pkg::cache::RedisCache::new(&state.config.redis_url).await {
            Ok(cache) => match cache.health_check().await {
                Ok(_) => "connected",
                Err(_) => {
                    is_healthy = false;
                    "error"
                }
            },
            Err(_) => {
                is_healthy = false;
                "error"
            }
        }
    } else {
        "disabled"
    };

    // 3. RabbitMQ check
    let rabbitmq_status = if state.config.rabbitmq_enabled {
        match crate::pkg::queue::RabbitMQBroker::new(&state.config.rabbitmq_url).await {
            Ok(broker) => match broker.health_check().await {
                Ok(_) => "connected",
                Err(_) => {
                    is_healthy = false;
                    "error"
                }
            },
            Err(_) => {
                is_healthy = false;
                "error"
            }
        }
    } else {
        "disabled"
    };

    // 4. Realtime status
    let realtime_status = if state.config.realtime_ws_enabled || state.config.realtime_sse_enabled {
        "connected"
    } else {
        "disabled"
    };

    Json(json!({
        "status": if is_healthy { "healthy" } else { "degraded" },
        "app": state.config.app_name,
        "environment": state.config.environment,
        "database": db_status,
        "redis": redis_status,
        "rabbitmq": rabbitmq_status,
        "realtime": realtime_status,
        "ports": {
            "http": state.config.port,
            "grpc": state.config.grpc_port,
        },
        "features": {
            "redis": state.config.redis_enabled,
            "rabbitmq": state.config.rabbitmq_enabled,
            "idempotency": state.config.idempotency_enabled,
            "realtime_ws": state.config.realtime_ws_enabled,
            "realtime_sse": state.config.realtime_sse_enabled,
            "realtime_bridge": state.config.realtime_bridge,
            "otel_enabled": state.config.otel_enabled,
        },
        "timestamp": Utc::now().to_rfc3339()
    }))
}

pub async fn protected_data_handler(auth_user: AuthenticatedUser) -> Json<Value> {
    Json(json!({
        "message": "This is protected data",
        "user_id": auth_user.id,
        "email": auth_user.email,
        "role": auth_user.role,
    }))
}

pub async fn public_or_private_handler(optional_user: OptionalAuthUser) -> Json<Value> {
    if let Some(user) = optional_user.0 {
        Json(json!({
            "message": "Private data",
            "authenticated": true,
            "user_id": user.id,
            "email": user.email,
        }))
    } else {
        Json(json!({
            "message": "Public data",
            "authenticated": false,
        }))
    }
}

pub struct AppInstance {
    pub router: Router,
    pub stock_service: Arc<StockService>,
}

pub fn create_app_instance(config: AppConfig, pool: PgPool, hub: Arc<RealtimeHub>) -> AppInstance {
    create_app_instance_with_broker(config, pool, hub, None)
}

pub fn create_app_instance_with_broker(
    config: AppConfig,
    pool: PgPool,
    hub: Arc<RealtimeHub>,
    broker: Option<Arc<crate::pkg::queue::RabbitMQBroker>>,
) -> AppInstance {
    let state = AppState {
        config: config.clone(),
        pool: pool.clone(),
        hub: hub.clone(),
    };

    let ingredient_repo = Arc::new(IngredientRepository::new(pool.clone()));
    let ingredient_service = Arc::new(IngredientService::new(ingredient_repo));

    let recipe_repo = Arc::new(RecipeRepository::new(pool.clone()));
    let recipe_service = Arc::new(RecipeService::new(recipe_repo));

    let stock_repo = Arc::new(StockRepository::new(pool.clone()));
    let stock_service = Arc::new(StockService::with_rabbitmq(stock_repo, broker));

    let opname_repo = Arc::new(OpnameRepository::new(pool.clone()));
    let opname_service = Arc::new(OpnameService::with_stock_service(opname_repo, stock_service.clone()));

    let user_service = Arc::new(UserService::with_default_repo(pool.clone()));

    let mut api_routes = Router::new()
        .nest("/ingredients", ingredient_router(ingredient_service))
        .nest("/recipes", recipe_router(recipe_service))
        .nest("/stock", stock_router(stock_service.clone()))
        .nest("/opnames", opname_router(opname_service))
        .nest("/users", user_router(user_service))
        .nest("/realtime", realtime_router(hub))
        .route("/protected/data", get(protected_data_handler))
        .route("/public-or-private", get(public_or_private_handler));

    if config.idempotency_enabled {
        let idempotency_store = Arc::new(IdempotencyStore::new(
            Duration::from_secs(config.idempotency_lifetime_hours * 3600),
            Duration::from_secs(config.idempotency_lock_timeout_seconds),
        ));
        api_routes = api_routes.layer(axum::middleware::from_fn(move |req, next| {
            idempotency_layer(idempotency_store.clone(), req, next)
        }));
    }

    let health_router = Router::new()
        .route("/health", get(health_check_handler))
        .with_state(state);

    let internal_routes = Router::new()
        .nest("/stock", internal_stock_router(stock_service.clone()));

    let router = Router::new()
        .route("/", get(root_handler))
        .merge(health_router)
        .nest("/api/v1", api_routes.clone())
        .nest("/api/1.0", api_routes)
        .nest("/api/internal", internal_routes)
        .layer(create_trace_layer::<axum::body::Body>())
        .layer(CorsLayer::permissive());

    AppInstance {
        router,
        stock_service,
    }
}

pub fn build_app(config: AppConfig, pool: PgPool, hub: Arc<RealtimeHub>) -> Router {
    create_app_instance(config, pool, hub).router
}
