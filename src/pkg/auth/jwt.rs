use axum::{async_trait, extract::FromRequestParts, http::request::Parts};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

use crate::{
    constants::codes::{CODE_ACCESS_TOKEN_EXPIRED, CODE_INVALID_ACCESS_TOKEN, CODE_UNAUTHORIZED},
    pkg::errors::AppError,
};

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Claims {
    #[serde(default)]
    pub sub: Option<serde_json::Value>,

    #[serde(default)]
    pub email: Option<String>,

    #[serde(default)]
    pub role: Option<String>,

    #[serde(default)]
    pub token_type: Option<String>,

    #[serde(default)]
    pub full_name: Option<String>,

    #[serde(default)]
    pub exp: Option<usize>,

    #[serde(default)]
    pub iat: Option<usize>,

    #[serde(flatten)]
    pub extra: std::collections::HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticatedUser {
    pub id: String,
    pub email: String,
    pub role: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: chrono::DateTime<Utc>,
    pub token_type: String,
}

pub fn get_jwt_secret() -> String {
    std::env::var("SECRET_JWT")
        .or_else(|_| std::env::var("APP_JWT_SECRET"))
        .or_else(|_| std::env::var("JWT_SECRET"))
        .unwrap_or_else(|_| {
            "default-secret-jwt-key-for-development-and-testing-purposes-123456789".to_string()
        })
}

pub fn generate_token_pair(
    user_id: &str,
    email: &str,
    role: &str,
    secret: &str,
    access_expiry_minutes: i64,
) -> Result<TokenPair, AppError> {
    let now = Utc::now();
    let access_exp = now + Duration::minutes(access_expiry_minutes);

    let claims = Claims {
        sub: Some(serde_json::Value::String(user_id.to_string())),
        email: Some(email.to_string()),
        role: Some(role.to_string()),
        token_type: Some("ACCESS".to_string()),
        full_name: None,
        exp: Some(access_exp.timestamp() as usize),
        iat: Some(now.timestamp() as usize),
        extra: std::collections::HashMap::new(),
    };

    let access_token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| AppError::internal(format!("Token generation failed: {}", e)))?;

    let refresh_token = format!("{}.{}", uuid::Uuid::new_v4(), uuid::Uuid::new_v4());

    Ok(TokenPair {
        access_token,
        refresh_token,
        expires_at: access_exp,
        token_type: "Bearer".to_string(),
    })
}

pub fn verify_access_token(token: &str, secret: &str) -> Result<Claims, AppError> {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.algorithms = vec![Algorithm::HS256, Algorithm::HS512, Algorithm::HS384];
    validation.validate_exp = true;

    decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )
    .map(|data| data.claims)
    .map_err(|err| match err.kind() {
        jsonwebtoken::errors::ErrorKind::ExpiredSignature => {
            AppError::unauthorized("Access token has expired", CODE_ACCESS_TOKEN_EXPIRED)
        }
        _ => AppError::unauthorized("Invalid access token", CODE_INVALID_ACCESS_TOKEN),
    })
}

/// Axum extractor for authenticated routes.
#[async_trait]
impl<S> FromRequestParts<S> for AuthenticatedUser
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let auth_header = parts
            .headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|h| h.to_str().ok())
            .ok_or_else(|| {
                AppError::unauthorized("Authorization header missing", CODE_UNAUTHORIZED)
            })?;

        if !auth_header.starts_with("Bearer ") {
            return Err(AppError::unauthorized(
                "Invalid Authorization scheme, must be Bearer",
                CODE_UNAUTHORIZED,
            ));
        }

        let token = &auth_header[7..];
        let secret = get_jwt_secret();
        let claims = verify_access_token(token, &secret)?;

        let user_id = claims
            .extra
            .get("userId")
            .or_else(|| claims.extra.get("UserId"))
            .and_then(|v| match v {
                serde_json::Value::Number(n) => Some(n.to_string()),
                serde_json::Value::String(s) => Some(s.clone()),
                _ => None,
            })
            .or_else(|| match &claims.sub {
                Some(serde_json::Value::Number(n)) => Some(n.to_string()),
                Some(serde_json::Value::String(s)) => Some(s.clone()),
                _ => None,
            })
            .unwrap_or_else(|| "unknown".to_string());

        let email = claims
            .email
            .or_else(|| {
                claims
                    .extra
                    .get("Email")
                    .and_then(|v| v.as_str().map(String::from))
            })
            .unwrap_or_default();

        let role = claims
            .role
            .or_else(|| {
                claims
                    .extra
                    .get("Role")
                    .and_then(|v| v.as_str().map(String::from))
            })
            .unwrap_or_else(|| "user".to_string());

        Ok(AuthenticatedUser {
            id: user_id,
            email,
            role,
        })
    }
}

/// Extractor for routes that strictly require Admin privileges (`admin` or `superadmin`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminUser(pub AuthenticatedUser);

impl std::ops::Deref for AdminUser {
    type Target = AuthenticatedUser;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for AdminUser
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let user = AuthenticatedUser::from_request_parts(parts, state).await?;
        let role = user.role.to_lowercase();
        if role != "admin" && role != "superadmin" {
            return Err(AppError::forbidden(
                "Forbidden: Admin privileges required",
                crate::constants::codes::CODE_FORBIDDEN,
            ));
        }
        Ok(AdminUser(user))
    }
}

/// Extractor for routes that require Staff privileges (`admin`, `superadmin`, `manager`, `cashier`, `barista`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StaffUser(pub AuthenticatedUser);

impl std::ops::Deref for StaffUser {
    type Target = AuthenticatedUser;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for StaffUser
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let user = AuthenticatedUser::from_request_parts(parts, state).await?;
        let role = user.role.to_lowercase();
        let is_staff = matches!(
            role.as_str(),
            "admin" | "superadmin" | "manager" | "cashier" | "barista" | "staff"
        );
        if !is_staff {
            return Err(AppError::forbidden(
                "Forbidden: Staff or Admin privileges required",
                crate::constants::codes::CODE_FORBIDDEN,
            ));
        }
        Ok(StaffUser(user))
    }
}

/// Extractor for public routes that optionally accept an authenticated user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptionalAuthUser(pub Option<AuthenticatedUser>);

impl std::ops::Deref for OptionalAuthUser {
    type Target = Option<AuthenticatedUser>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for OptionalAuthUser
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        match parts.headers.get(axum::http::header::AUTHORIZATION) {
            Some(_) => {
                let user = AuthenticatedUser::from_request_parts(parts, state).await?;
                Ok(OptionalAuthUser(Some(user)))
            }
            None => Ok(OptionalAuthUser(None)),
        }
    }
}
