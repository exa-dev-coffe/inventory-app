use chrono::Utc;
use inventory_app::pkg::auth::hmac::{generate_hmac, validate_timestamp, verify_hmac};

#[test]
fn test_generate_and_verify_hmac() {
    let secret = "my-secret-key-123";
    let message = "ids=1,2,32026-11-05T09:00:00Z";

    let signature = generate_hmac(secret, message);
    assert!(!signature.is_empty());

    // Valid verification
    assert!(verify_hmac(secret, message, &signature).is_ok());

    // Tampered message
    assert!(verify_hmac(secret, "tampered-message", &signature).is_err());

    // Wrong secret
    assert!(verify_hmac("wrong-secret", message, &signature).is_err());

    // Invalid base64
    assert!(verify_hmac(secret, message, "not-valid-base64!@#").is_err());
}

#[test]
fn test_validate_timestamp() {
    let now = Utc::now().to_rfc3339();
    assert!(validate_timestamp(&now, 300).is_ok());

    // Expired timestamp (10 minutes ago with 5-minute tolerance)
    let ten_mins_ago = (Utc::now() - chrono::Duration::minutes(10)).to_rfc3339();
    assert!(validate_timestamp(&ten_mins_ago, 300).is_err());

    // Invalid format
    assert!(validate_timestamp("invalid-date-string", 300).is_err());
}
