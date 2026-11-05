use async_trait::async_trait;
use chrono::Utc;
use sqlx::{PgPool, Postgres, Transaction};
use std::sync::Arc;
use inventory_app::{
    modules::user::{
        dto::{ListUsersQuery, UpdateProfileRequest, UpdateUserRequest, UserResponse},
        repository::{NewUser, UserRecord, UserRepository},
        service::UserService,
    },
    pkg::errors::AppError,
};
use uuid::Uuid;

// Mock UserRepository
struct MockUserRepo;

#[async_trait]
impl UserRepository for MockUserRepo {
    async fn find_by_id(&self, _pool: &PgPool, id: Uuid) -> Result<Option<UserResponse>, AppError> {
        Ok(Some(UserResponse {
            id,
            name: "Mock User".to_string(),
            email: "mock@example.com".to_string(),
            phone: Some("081234567890".to_string()),
            role: "user".to_string(),
            is_active: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }))
    }

    async fn find_by_email(
        &self,
        _pool: &PgPool,
        email: &str,
    ) -> Result<Option<UserRecord>, AppError> {
        if email == "exists@example.com" {
            Ok(Some(UserRecord {
                id: Uuid::new_v4(),
                name: "Existing".to_string(),
                email: email.to_string(),
                phone: None,
                password: "hash".to_string(),
                role: "user".to_string(),
                is_active: true,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            }))
        } else {
            Ok(None)
        }
    }

    async fn exists_by_id(&self, _pool: &PgPool, _id: Uuid) -> Result<bool, AppError> {
        Ok(true)
    }

    async fn exists_by_email(&self, _pool: &PgPool, email: &str) -> Result<bool, AppError> {
        Ok(email == "exists@example.com")
    }

    async fn create(
        &self,
        _tx: &mut Transaction<'_, Postgres>,
        new_user: NewUser<'_>,
    ) -> Result<UserResponse, AppError> {
        Ok(UserResponse {
            id: Uuid::new_v4(),
            name: new_user.name.to_string(),
            email: new_user.email.to_string(),
            phone: new_user.phone.map(|s| s.to_string()),
            role: new_user.role.to_string(),
            is_active: new_user.is_active,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
    }

    async fn update(
        &self,
        _tx: &mut Transaction<'_, Postgres>,
        id: Uuid,
        req: &UpdateUserRequest,
    ) -> Result<UserResponse, AppError> {
        Ok(UserResponse {
            id,
            name: req.name.clone().unwrap_or_else(|| "Updated".to_string()),
            email: req
                .email
                .clone()
                .unwrap_or_else(|| "updated@example.com".to_string()),
            phone: req.phone.clone(),
            role: req.role.clone().unwrap_or_else(|| "user".to_string()),
            is_active: req.is_active.unwrap_or(true),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
    }

    async fn update_profile(
        &self,
        _tx: &mut Transaction<'_, Postgres>,
        id: Uuid,
        req: &UpdateProfileRequest,
    ) -> Result<UserResponse, AppError> {
        Ok(UserResponse {
            id,
            name: req.name.clone().unwrap_or_else(|| "Profile".to_string()),
            email: "user@example.com".to_string(),
            phone: req.phone.clone(),
            role: "user".to_string(),
            is_active: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
    }

    async fn delete(&self, _tx: &mut Transaction<'_, Postgres>, _id: Uuid) -> Result<(), AppError> {
        Ok(())
    }

    async fn hard_delete(
        &self,
        _tx: &mut Transaction<'_, Postgres>,
        _id: Uuid,
    ) -> Result<(), AppError> {
        Ok(())
    }

    async fn list(
        &self,
        _pool: &PgPool,
        _query: &ListUsersQuery,
        _limit: i64,
        _offset: i64,
    ) -> Result<(Vec<UserResponse>, i64), AppError> {
        let users = vec![UserResponse {
            id: Uuid::new_v4(),
            name: "User 1".to_string(),
            email: "user1@example.com".to_string(),
            phone: None,
            role: "user".to_string(),
            is_active: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }];
        Ok((users, 1))
    }
}

// Mock AuthRepository
#[tokio::test]
async fn test_user_service_get_profile() {
    let mock_user_repo = Arc::new(MockUserRepo);
    // Dummy pool created without active DB for unit tests using mock
    let pool_options = sqlx::postgres::PgPoolOptions::new();
    let dummy_pool = pool_options
        .connect_lazy("postgres://dummy:dummy@127.0.0.1:5432/dummy")
        .unwrap();

    let service = UserService::new(mock_user_repo, dummy_pool);
    let user_id = Uuid::new_v4();

    let profile = service
        .get_profile(user_id)
        .await
        .expect("Profile should return");
    assert_eq!(profile.id, user_id);
    assert_eq!(profile.email, "mock@example.com");
}

#[tokio::test]
async fn test_user_service_list_users() {
    let mock_user_repo = Arc::new(MockUserRepo);
    let pool_options = sqlx::postgres::PgPoolOptions::new();
    let dummy_pool = pool_options
        .connect_lazy("postgres://dummy:dummy@127.0.0.1:5432/dummy")
        .unwrap();

    let service = UserService::new(mock_user_repo, dummy_pool);

    let query = ListUsersQuery {
        page: Some(1),
        limit: Some(10),
        search: None,
        role: None,
        is_active: None,
    };

    let (users, page, limit, total) = service
        .list_users(query)
        .await
        .expect("Listing users should succeed");
    assert_eq!(page, 1);
    assert_eq!(limit, 10);
    assert_eq!(total, 1);
    assert_eq!(users.len(), 1);
}
