//! Integration tests for iam-service.

#[cfg(test)]
mod integration_tests {
    use uuid::Uuid;
    use crate::commands::{
        IamCommandHandler, Authenticate, CreateApiKey, RevokeApiKey,
        SubmitChange, ReviewChange, CommandHandler,
    };
    use crate::queries::{IamQueries, QueryHandler};
    use crate::domain::{Principal, ApiKeyStatus, ChangeStatus};
    use crate::repository::{InMemoryIamRepository, IamRepository};

    fn setup() -> (IamCommandHandler<InMemoryIamRepository>, IamQueries<InMemoryIamRepository>) {
        let repo = InMemoryIamRepository::new();
        let handler = IamCommandHandler::new(repo.clone(), "test-jwt-secret".into());
        let queries = IamQueries::new(repo);
        (handler, queries)
    }

    fn create_test_principal(repo: &impl IamRepository) -> Principal {
        use argon2::{Argon2, PasswordHasher};
        use argon2::password_hash::SaltString;
        use argon2::password_hash::rand_core::OsRng;
        let salt = SaltString::generate(&mut OsRng);
        let hash = Argon2::default()
            .hash_password(b"password123", &salt)
            .expect("Argon2 hashing failed");
        let principal = Principal::new_human(
            Uuid::now_v7(),
            "admin@test.com".into(),
            hash.to_string().into_bytes(),
        );
        let _ = futures::executor::block_on(repo.save_principal(&principal));
        principal
    }

    #[tokio::test]
    async fn test_full_auth_flow() {
        let repo = InMemoryIamRepository::new();
        create_test_principal(&repo);
        let handler = IamCommandHandler::new(repo, "test-secret".into());

        let result = handler.authenticate(Authenticate {
            email: "admin@test.com".into(),
            password: "password123".into(),
            ip_address: "10.0.0.1".parse().unwrap(),
            user_agent: "test-agent".into(),
        }).await.unwrap();

        assert!(!result.access_token.is_empty());
        assert!(!result.refresh_token.is_empty());
    }

    #[tokio::test]
    async fn test_api_key_lifecycle() {
        let repo = InMemoryIamRepository::new();
        let principal = create_test_principal(&repo);
        let handler = IamCommandHandler::new(repo.clone(), "test-secret".into());
        let queries = IamQueries::new(repo);

        let created = handler.create_api_key(CreateApiKey {
            principal_id: principal.id,
            name: "Test Key".into(),
            scopes: vec!["payments:read".into(), "payments:write".into()],
            expires_in_days: Some(90),
        }).await.unwrap();

        assert_eq!(created.api_key.status, ApiKeyStatus::Active);

        let keys = queries.list_api_keys(principal.id).await.unwrap();
        assert_eq!(keys.len(), 1);

        handler.revoke_api_key(RevokeApiKey {
            api_key_id: created.api_key.api_key_id,
            principal_id: principal.id,
        }).await.unwrap();

        let keys = queries.list_api_keys(principal.id).await.unwrap();
        assert_eq!(keys[0].status, ApiKeyStatus::Revoked);
    }

    #[tokio::test]
    async fn test_maker_checker_full_flow() {
        let repo = InMemoryIamRepository::new();
        let maker_id = Uuid::now_v7();
        let checker_id = Uuid::now_v7();
        let handler = IamCommandHandler::new(repo, "test-secret".into());

        let submitted = handler.submit_change(SubmitChange {
            change_type: "update_routing_policy".into(),
            maker_id,
            payload: b"{\"priority\": 1}".to_vec(),
            maker_note: Some("Update routing priority".into()),
        }).await.unwrap();

        assert_eq!(submitted.change.status, ChangeStatus::Pending);

        let approved = handler.review_change(ReviewChange {
            change_id: submitted.change.change_id,
            checker_id,
            approved: true,
            checker_note: Some("Approved after review".into()),
        }).await.unwrap();

        assert_eq!(approved.change.status, ChangeStatus::Approved);
        assert_eq!(approved.change.checker_id, Some(checker_id));
    }

    #[tokio::test]
    async fn test_wrong_password_locks_account() {
        let repo = InMemoryIamRepository::new();
        create_test_principal(&repo);
        let handler = IamCommandHandler::new(repo, "test-secret".into());

        for _ in 0..5 {
            let _ = handler.authenticate(Authenticate {
                email: "admin@test.com".into(),
                password: "wrong".into(),
                ip_address: "10.0.0.1".parse().unwrap(),
                user_agent: "test".into(),
            }).await;
        }

        let result = handler.authenticate(Authenticate {
            email: "admin@test.com".into(),
            password: "password123".into(),
            ip_address: "10.0.0.1".parse().unwrap(),
            user_agent: "test".into(),
        }).await;

        assert!(result.is_err());
    }
}
