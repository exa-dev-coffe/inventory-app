use axum::{
    extract::FromRequestParts,
    http::{header::AUTHORIZATION, Request},
};
use inventory_app::pkg::auth::jwt::{generate_token_pair, AdminUser, OptionalAuthUser, StaffUser};

#[tokio::test]
async fn test_admin_user_extractor() {
    let secret = "test-secret-key-12345678901234567890";
    std::env::set_var("JWT_SECRET", secret);

    let admin_id = "1"; // Account-app numeric user id
    let admin_tokens =
        generate_token_pair(admin_id, "admin@example.com", "admin", secret, 15).unwrap();

    let user_id = "42";
    let user_tokens =
        generate_token_pair(user_id, "user@example.com", "customer", secret, 15).unwrap();

    // 1. Admin token should succeed
    let req = Request::builder()
        .header(
            AUTHORIZATION,
            format!("Bearer {}", admin_tokens.access_token),
        )
        .body(())
        .unwrap();
    let (mut parts, _) = req.into_parts();
    let admin = AdminUser::from_request_parts(&mut parts, &()).await;
    assert!(admin.is_ok());
    assert_eq!(admin.unwrap().role, "admin");

    // 2. Regular customer token should fail with forbidden
    let req_user = Request::builder()
        .header(
            AUTHORIZATION,
            format!("Bearer {}", user_tokens.access_token),
        )
        .body(())
        .unwrap();
    let (mut parts_user, _) = req_user.into_parts();
    let user_res = AdminUser::from_request_parts(&mut parts_user, &()).await;
    assert!(user_res.is_err());
}

#[tokio::test]
async fn test_staff_user_extractor() {
    let secret = "test-secret-key-12345678901234567890";
    std::env::set_var("JWT_SECRET", secret);

    let cashier_id = "2";
    let cashier_tokens =
        generate_token_pair(cashier_id, "cashier@example.com", "cashier", secret, 15).unwrap();

    let customer_id = "3";
    let customer_tokens =
        generate_token_pair(customer_id, "customer@example.com", "customer", secret, 15).unwrap();

    // 1. Cashier token should succeed for StaffUser
    let req = Request::builder()
        .header(
            AUTHORIZATION,
            format!("Bearer {}", cashier_tokens.access_token),
        )
        .body(())
        .unwrap();
    let (mut parts, _) = req.into_parts();
    let staff = StaffUser::from_request_parts(&mut parts, &()).await;
    assert!(staff.is_ok());
    assert_eq!(staff.unwrap().role, "cashier");

    // 2. Customer token should fail for StaffUser
    let req_cust = Request::builder()
        .header(
            AUTHORIZATION,
            format!("Bearer {}", customer_tokens.access_token),
        )
        .body(())
        .unwrap();
    let (mut parts_cust, _) = req_cust.into_parts();
    let cust_res = StaffUser::from_request_parts(&mut parts_cust, &()).await;
    assert!(cust_res.is_err());
}

#[tokio::test]
async fn test_optional_auth_user_extractor() {
    let secret = "test-secret-key-12345678901234567890";
    std::env::set_var("JWT_SECRET", secret);

    let user_id = "42";
    let tokens = generate_token_pair(user_id, "opt@example.com", "user", secret, 15).unwrap();

    // 1. Present valid token
    let req = Request::builder()
        .header(AUTHORIZATION, format!("Bearer {}", tokens.access_token))
        .body(())
        .unwrap();
    let (mut parts, _) = req.into_parts();
    let opt = OptionalAuthUser::from_request_parts(&mut parts, &())
        .await
        .unwrap();
    assert!(opt.is_some());
    assert_eq!(opt.0.unwrap().email, "opt@example.com");

    // 2. Absent token -> returns None without error
    let req_absent = Request::builder().body(()).unwrap();
    let (mut parts_absent, _) = req_absent.into_parts();
    let opt_absent = OptionalAuthUser::from_request_parts(&mut parts_absent, &())
        .await
        .unwrap();
    assert!(opt_absent.is_none());
}
