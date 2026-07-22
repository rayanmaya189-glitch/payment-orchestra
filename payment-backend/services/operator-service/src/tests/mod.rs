//! Integration tests for operator-service.
//! Tests execute against the in-memory repository for fast feedback.

#[cfg(test)]
mod integration_tests {
    use uuid::Uuid;
    use crate::commands::{OperatorCommandHandler, RegisterOperator, VerifyEmail, UpdateOperatorStatus};
    use crate::domain::{OperatorStatus, OperatorError};
    use crate::queries::OperatorQueries;
    use crate::repository::InMemoryOperatorRepository;

    fn setup() -> (OperatorCommandHandler<InMemoryOperatorRepository>, OperatorQueries<InMemoryOperatorRepository>) {
        let repo = InMemoryOperatorRepository::new();
        let handler = OperatorCommandHandler::new(repo.clone());
        let queries = OperatorQueries::new(repo);
        (handler, queries)
    }

    #[tokio::test]
    async fn test_full_lifecycle() {
        let (handler, queries) = setup();

        // 1. Register
        let result = handler.register(RegisterOperator {
            legal_name: "Full Cycle Corp".into(),
            trade_license_no: "FC-1001".into(),
            country: "AE".into(),
            email: "fullcycle@example.com".into(),
        }).await.unwrap();

        let op_id = result.operator.id;

        // Verify via query
        let op = queries.get_operator(op_id).await.unwrap().unwrap();
        assert_eq!(op.status, OperatorStatus::Pending);

        // 2. Verify email
        handler.verify_email(VerifyEmail {
            operator_id: op_id,
            verification_token: result.verification_token,
        }).await.unwrap();

        let op = queries.get_operator(op_id).await.unwrap().unwrap();
        assert_eq!(op.status, OperatorStatus::ActiveUnverified);

        // 3. Update to verified (compliance approval)
        handler.update_status(UpdateOperatorStatus {
            operator_id: op_id,
            new_status: OperatorStatus::ActiveVerified,
            reason: "KYB approved".into(),
            changed_by: Uuid::now_v7(),
        }).await.unwrap();

        let op = queries.get_operator(op_id).await.unwrap().unwrap();
        assert_eq!(op.status, OperatorStatus::ActiveVerified);
        assert!(op.status.can_process_live_transactions());

        // 4. Suspend
        handler.update_status(UpdateOperatorStatus {
            operator_id: op_id,
            new_status: OperatorStatus::Suspended,
            reason: "Compliance violation".into(),
            changed_by: Uuid::now_v7(),
        }).await.unwrap();

        let op = queries.get_operator(op_id).await.unwrap().unwrap();
        assert_eq!(op.status, OperatorStatus::Suspended);
        assert!(!op.status.can_process_live_transactions());
    }

    #[tokio::test]
    async fn test_register_with_existing_email_rejected() {
        let (handler, _) = setup();

        handler.register(RegisterOperator {
            legal_name: "First Corp".into(),
            trade_license_no: "FC-2001".into(),
            country: "AE".into(),
            email: "same@example.com".into(),
        }).await.unwrap();

        let result = handler.register(RegisterOperator {
            legal_name: "Second Corp".into(),
            trade_license_no: "FC-2002".into(),
            country: "AE".into(),
            email: "same@example.com".into(),
        }).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_list_operators_by_status() {
        let (handler, queries) = setup();

        handler.register(RegisterOperator {
            legal_name: "List Corp A".into(),
            trade_license_no: "LC-3001".into(),
            country: "AE".into(),
            email: "lista@example.com".into(),
        }).await.unwrap();

        handler.register(RegisterOperator {
            legal_name: "List Corp B".into(),
            trade_license_no: "LC-3002".into(),
            country: "AE".into(),
            email: "listb@example.com".into(),
        }).await.unwrap();

        let operators = queries.list_operators(None).await.unwrap();
        assert_eq!(operators.len(), 2);

        let pending = queries.list_operators(Some("pending")).await.unwrap();
        assert_eq!(pending.len(), 2);

        let verified = queries.list_operators(Some("active_verified")).await.unwrap();
        assert_eq!(verified.len(), 0);
    }

    #[tokio::test]
    async fn test_get_nonexistent_operator() {
        let (_, queries) = setup();
        let result = queries.get_operator(Uuid::now_v7()).await.unwrap();
        assert!(result.is_none());
    }
}
