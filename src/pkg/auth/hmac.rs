use axum::{
    body::{to_bytes, Body},
    extract::Request,
    middleware::Next,
    response::Response,
};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use sha2::Sha256;

use crate::pkg::errors::AppError;

type HmacSha256 = Hmac<Sha256>;

/// Generate base64-encoded HMAC-SHA256 signature for a message.
pub fn generate_hmac(secret: &str, message: &str) -> String {
    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC can take key of any size");
    mac.update(message.as_bytes());
    let result = mac.finalize();
    BASE64.encode(result.into_bytes())
}

/// Verify HMAC-SHA256 signature in constant time against a message.
pub fn verify_hmac(secret: &str, message: &str, signature_b64: &str) -> Result<(), AppError> {
    let expected_sig_bytes = BASE64.decode(signature_b64).map_err(|_| {
        AppError::unauthorized(
            "Invalid base64 signature format",
            crate::constants::codes::CODE_UNAUTHORIZED,
        )
    })?;

    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC can take key of any size");
    mac.update(message.as_bytes());

    mac.verify_slice(&expected_sig_bytes).map_err(|_| {
        AppError::unauthorized(
            "Invalid HMAC signature",
            crate::constants::codes::CODE_UNAUTHORIZED,
        )
    })
}

/// Validate request timestamp is within tolerance (default 5 minutes).
pub fn validate_timestamp(timestamp_str: &str, max_skew_seconds: i64) -> Result<(), AppError> {
    let req_time = DateTime::parse_from_rfc3339(timestamp_str).map_err(|_| {
        AppError::unauthorized(
            "Invalid RFC3339 timestamp format",
            crate::constants::codes::CODE_UNAUTHORIZED,
        )
    })?;

    let now = Utc::now();
    let skew = (now - req_time.with_timezone(&Utc)).num_seconds().abs();

    if skew > max_skew_seconds {
        return Err(AppError::unauthorized(
            "Timestamp expired or skewed beyond tolerance",
            crate::constants::codes::CODE_UNAUTHORIZED,
        ));
    }

    Ok(())
}

/// Axum middleware for internal service-to-service HMAC validation.
pub async fn validate_hmac_middleware(req: Request, next: Next) -> Result<Response, AppError> {
    let signature = req
        .headers()
        .get("x-signature")
        .or_else(|| req.headers().get("X-Signature"))
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .ok_or_else(|| {
            AppError::unauthorized(
                "Missing X-Signature header",
                crate::constants::codes::CODE_UNAUTHORIZED,
            )
        })?;

    let timestamp = req
        .headers()
        .get("x-timestamp")
        .or_else(|| req.headers().get("X-Timestamp"))
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .ok_or_else(|| {
            AppError::unauthorized(
                "Missing X-Timestamp header",
                crate::constants::codes::CODE_UNAUTHORIZED,
            )
        })?;

    validate_timestamp(&timestamp, 300)?;

    let (parts, body) = req.into_parts();
    let bytes = to_bytes(body, 2 * 1024 * 1024).await.map_err(|e| {
        AppError::bad_request(
            &format!("Failed to read request body: {}", e),
            crate::constants::codes::CODE_BAD_REQUEST,
        )
    })?;

    let body_str = std::str::from_utf8(&bytes).unwrap_or("");
    let query_str = parts.uri.query().unwrap_or("");
    let message = format!("{}{}{}", query_str, timestamp, body_str);

    let secret = std::env::var("APP_SECRET")
        .or_else(|_| std::env::var("SECRET"))
        .unwrap_or_else(|_| "secret".to_string());

    verify_hmac(&secret, &message, &signature)?;

    let req = Request::from_parts(parts, Body::from(bytes));
    Ok(next.run(req).await)
}
