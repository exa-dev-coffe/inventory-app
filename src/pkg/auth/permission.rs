use axum::{async_trait, extract::FromRequestParts, http::request::Parts};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::RwLock};
use tracing::{debug, info, warn};

use crate::{
    constants::codes::CODE_FORBIDDEN,
    pkg::{
        auth::jwt::{get_jwt_secret, verify_access_token, AuthenticatedUser},
        errors::AppError,
    },
};

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct PermissionAction {
    #[serde(default)]
    pub view: bool,
    #[serde(default)]
    pub create: bool,
    #[serde(default)]
    pub edit: bool,
    #[serde(default)]
    pub delete: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionType {
    View,
    Create,
    Edit,
    Delete,
}

impl ActionType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ActionType::View => "view",
            ActionType::Create => "create",
            ActionType::Edit => "edit",
            ActionType::Delete => "delete",
        }
    }
}

pub trait PermissionRequirement: Send + Sync + 'static {
    fn feature() -> &'static str;
    fn action() -> ActionType;
}

/// Extractor for routes guarded by RBAC permissions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequirePermission<T: PermissionRequirement> {
    pub user: AuthenticatedUser,
    #[serde(skip)]
    _marker: std::marker::PhantomData<T>,
}

impl<T: PermissionRequirement> std::ops::Deref for RequirePermission<T> {
    type Target = AuthenticatedUser;
    fn deref(&self) -> &Self::Target {
        &self.user
    }
}

// In-memory permission cache for testing without live Redis / account-service
static TEST_PERMISSIONS: RwLock<Option<HashMap<i32, HashMap<String, PermissionAction>>>> =
    RwLock::new(None);

pub fn set_test_role_permissions(role_id: i32, perms: HashMap<String, PermissionAction>) {
    let mut lock = TEST_PERMISSIONS.write().unwrap();
    let map = lock.get_or_insert_with(HashMap::new);
    map.insert(role_id, perms);
}

pub fn clear_test_role_permissions() {
    let mut lock = TEST_PERMISSIONS.write().unwrap();
    *lock = None;
}

fn get_redis_url() -> Option<String> {
    std::env::var("REDIS_URL").ok().or_else(|| {
        let host = std::env::var("REDIS_HOST").unwrap_or_else(|_| "localhost".to_string());
        let port = std::env::var("REDIS_PORT").unwrap_or_else(|_| "6379".to_string());
        let pass = std::env::var("REDIS_PASSWORD").unwrap_or_default();
        let db = std::env::var("REDIS_DB").unwrap_or_else(|_| "0".to_string());
        if pass.is_empty() {
            Some(format!("redis://{}:{}/{}", host, port, db))
        } else {
            Some(format!("redis://:{}@{}:{}/{}", pass, host, port, db))
        }
    })
}

pub async fn fetch_role_permissions(
    role_id: i32,
) -> Result<HashMap<String, PermissionAction>, AppError> {
    // 0. Check in-memory test overrides (for unit tests)
    if let Ok(lock) = TEST_PERMISSIONS.read() {
        if let Some(map) = lock.as_ref() {
            if let Some(perms) = map.get(&role_id) {
                return Ok(perms.clone());
            }
        }
    }

    let redis_key = format!("auth:role_permissions:{}", role_id);

    // 1. Try Redis cache if configured / reachable
    if let Some(redis_url) = get_redis_url() {
        if let Ok(cache) = crate::pkg::cache::RedisCache::new(&redis_url).await {
            if let Ok(Some(cached_json)) = cache.get(&redis_key).await {
                if let Ok(perms) =
                    serde_json::from_str::<HashMap<String, PermissionAction>>(&cached_json)
                {
                    debug!(role_id = role_id, "Fetched role permissions from Redis cache");
                    return Ok(perms);
                }
            }
        }
    }

    // 2. Cache miss: Fallback to Account Service internal API
    let account_service_url = std::env::var("SERVICE_ACCOUNT_URL")
        .or_else(|_| std::env::var("ACCOUNT_SERVICE_URL"))
        .unwrap_or_else(|_| "http://account-service.coffe.svc.cluster.local:8080".to_string());

    let timestamp = chrono::Utc::now().to_rfc3339();
    let secret = std::env::var("APP_SECRET")
        .or_else(|_| std::env::var("SECRET"))
        .unwrap_or_else(|_| "secret".to_string());

    let message = format!("{}{}{}", "", timestamp, "");
    let signature = crate::pkg::auth::hmac::generate_hmac(&secret, &message);

    let url = format!(
        "{}/api/internal/roles/{}/permissions",
        account_service_url.trim_end_matches('/'),
        role_id
    );

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(4))
        .build()
        .map_err(|e| AppError::internal(format!("HTTP client build error: {}", e)))?;

    match client
        .get(&url)
        .header("X-Signature", signature)
        .header("X-Timestamp", timestamp)
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => {
            #[derive(Deserialize)]
            struct RolePermResponse {
                data: HashMap<String, PermissionAction>,
            }

            if let Ok(body) = resp.json::<RolePermResponse>().await {
                // Warm Redis cache
                if let Some(redis_url) = get_redis_url() {
                    if let Ok(cache) = crate::pkg::cache::RedisCache::new(&redis_url).await {
                        if let Ok(json_str) = serde_json::to_string(&body.data) {
                            let _ = cache
                                .set(
                                    &redis_key,
                                    &json_str,
                                    Some(std::time::Duration::from_secs(86400)),
                                )
                                .await;
                        }
                    }
                }
                info!(role_id = role_id, "Synchronized role permissions from account-service");
                return Ok(body.data);
            }
        }
        Ok(resp) => {
            warn!(
                role_id = role_id,
                status = %resp.status(),
                "Account-service returned non-200 for internal role permissions"
            );
        }
        Err(err) => {
            warn!(
                role_id = role_id,
                error = %err,
                "Failed to reach account-service for internal role permissions"
            );
        }
    }

    Err(AppError::forbidden(
        "Forbidden: Unable to verify role permissions",
        CODE_FORBIDDEN,
    ))
}

pub async fn check_permission(
    user: &AuthenticatedUser,
    feature: &str,
    action: ActionType,
    extra_permissions: Option<&HashMap<String, PermissionAction>>,
) -> Result<(), AppError> {
    // 1. Super Admin full access bypass
    let role_lower = user.role.to_lowercase();
    if role_lower == "admin" || role_lower == "superadmin" || user.role_id == Some(1) {
        return Ok(());
    }

    // 2. Direct permissions in token claims (if provided in extra["permissions"])
    if let Some(perms) = extra_permissions {
        if let Some(perm) = perms.get(&feature.to_lowercase()) {
            let allowed = match action {
                ActionType::View => perm.view,
                ActionType::Create => perm.create,
                ActionType::Edit => perm.edit,
                ActionType::Delete => perm.delete,
            };
            if allowed {
                return Ok(());
            } else {
                return Err(AppError::forbidden(
                    format!(
                        "Forbidden: You do not have permission to {} {}",
                        action.as_str(),
                        feature
                    ),
                    CODE_FORBIDDEN,
                ));
            }
        }
    }

    // 3. Fallback to role_id lookup
    let role_id = user.role_id.ok_or_else(|| {
        AppError::forbidden(
            "Forbidden: Role information missing in token",
            CODE_FORBIDDEN,
        )
    })?;

    let permissions = fetch_role_permissions(role_id).await?;
    let perm = permissions.get(&feature.to_lowercase()).ok_or_else(|| {
        AppError::forbidden(
            format!(
                "Forbidden: You do not have permission to access {}",
                feature
            ),
            CODE_FORBIDDEN,
        )
    })?;

    let allowed = match action {
        ActionType::View => perm.view,
        ActionType::Create => perm.create,
        ActionType::Edit => perm.edit,
        ActionType::Delete => perm.delete,
    };

    if !allowed {
        return Err(AppError::forbidden(
            format!(
                "Forbidden: You do not have permission to {} {}",
                action.as_str(),
                feature
            ),
            CODE_FORBIDDEN,
        ));
    }

    Ok(())
}

#[async_trait]
impl<S, T> FromRequestParts<S> for RequirePermission<T>
where
    S: Send + Sync,
    T: PermissionRequirement,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let user = AuthenticatedUser::from_request_parts(parts, state).await?;

        // Extract any direct permissions from JWT if present in Authorization header
        let mut direct_perms = None;
        if let Some(auth_header) = parts
            .headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|h| h.to_str().ok())
        {
            if auth_header.starts_with("Bearer ") {
                let token = &auth_header[7..];
                let secret = get_jwt_secret();
                if let Ok(claims) = verify_access_token(token, &secret) {
                    if let Some(perms_val) = claims.extra.get("permissions") {
                        if let Ok(perms) = serde_json::from_value::<HashMap<String, PermissionAction>>(
                            perms_val.clone(),
                        ) {
                            direct_perms = Some(perms);
                        }
                    }
                }
            }
        }

        check_permission(&user, T::feature(), T::action(), direct_perms.as_ref()).await?;

        Ok(RequirePermission {
            user,
            _marker: std::marker::PhantomData,
        })
    }
}

macro_rules! define_permission {
    ($struct_name:ident, $type_alias:ident, $feature:expr, $action:ident) => {
        #[derive(Debug, Clone, Copy, Default)]
        pub struct $struct_name;
        impl $crate::pkg::auth::permission::PermissionRequirement for $struct_name {
            fn feature() -> &'static str {
                $feature
            }
            fn action() -> $crate::pkg::auth::permission::ActionType {
                $crate::pkg::auth::permission::ActionType::$action
            }
        }
        pub type $type_alias = $crate::pkg::auth::permission::RequirePermission<$struct_name>;
    };
}

// Inventory permissions
define_permission!(InventoryView, RequireInventoryView, "inventory", View);
define_permission!(InventoryCreate, RequireInventoryCreate, "inventory", Create);
define_permission!(InventoryEdit, RequireInventoryEdit, "inventory", Edit);
define_permission!(InventoryDelete, RequireInventoryDelete, "inventory", Delete);
