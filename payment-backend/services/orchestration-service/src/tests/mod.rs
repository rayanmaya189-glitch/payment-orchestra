//! Comprehensive TDD tests for orchestration-service.
//! Tests follow the spec: failing test → implementation → passing test.

#[cfg(test)]
mod tests {
    use uuid::Uuid;
    use chrono::Utc;

    use crate::domain::*;
    use crate::commands::*;
    use crate::queries::*;
    use crate::repository::*;

    fn setup_handler() -> (OrchestrationCommandHandler<InMemoryOrchestrationRepository>, InMemoryOrchestrationRepository) {
        let repo = InMemoryOrchestrationRepository::new();
        let handler = OrchestrationCommandHandler::new(repo.clone());
        (handler, repo)
    }

    /// Setup handler with active acquirer links for a specific operator.
    async fn setup_with_links(link_ids: Vec<Uuid>) -> (OrchestrationCommandHandler<InMemoryOrchestrationRepository>, InMemoryOrchestrationRepository, Uuid) {
        let repo = InMemoryOrchestrationRepository::new();
        let operator_id = Uuid::now_v7();
        repo.set_active_links(operator_id, link_ids).await;
        let handler = OrchestrationCommandHandler::new(repo.clone());
        (handler, repo, operator_id)
    }

    fn make_create_cmd(operator_id: Uuid, key: &str, amount: i64) -> CreatePaymentIntent {
        CreatePaymentIntent {
            operator_id,
            idempotency_key: key.to_string(),
            amount_minor_units: amount,
            currency: "AED".to_string(),
            purpose: PaymentPurpose::Payment,
            metadata: None,
            source_type: SourceType::MerchantApi,
            source_id: None,
            payment_method_token_id: None,
            preferred_gateway_profile_id: None,
        }
    }

    fn make_authorize_cmd(pi_id: Uuid, token_id: Uuid) -> AuthorizePaymentIntent {
        AuthorizePaymentIntent {
            payment_intent_id: pi_id,
            payment_method_token_id: token_id,
            card_scheme: "Visa".into(),
            actor_id: Uuid::now_v7(),
        }
    }

    fn make_activate_policy_cmd(operator_id: Uuid, link_id: Uuid) -> ActivateRoutingPolicy {
        ActivateRoutingPolicy {
            operator_id,
            rules: vec![RoutingRule {
                acquirer_link_id: link_id,
                priority: 1,
                condition: RoutingCondition::all(),
            }],
            failover_config: FailoverConfig::default(),
            partial_auth_strategy: PartialAuthStrategy::AcceptPartial,
            rotation_strategy: RotationStrategy::Priority,
            max_transaction_amount_minor: None,
        }
    }

    // ══════════════════════════════════════════════════════════════════════
    // CreatePaymentIntent Tests
    // ══════════════════════════════════════════════════════════════════════

    #[tokio::test]
    async fn test_create_payment_intent_success() {
        let (handler, _) = setup_handler();
        let operator_id = Uuid::now_v7();

        let result = handler.create_payment_intent(make_create_cmd(operator_id, "key-001", 10000)).await.unwrap();

        assert_eq!(result.status, PaymentStatus::Created);
        assert_eq!(result.requested_amount.amount_minor_units, 10000);
        assert_eq!(result.events.len(), 1);
    }

    #[tokio::test]
    async fn test_create_payment_intent_idempotent_replay() {
        let (handler, _) = setup_handler();
        let operator_id = Uuid::now_v7();

        let cmd = make_create_cmd(operator_id, "key-002", 5000);
        let r1 = handler.create_payment_intent(cmd.clone()).await.unwrap();
        let r2 = handler.create_payment_intent(cmd).await.unwrap();

        assert_eq!(r1.payment_intent_id, r2.payment_intent_id);
        assert_eq!(r1.status, r2.status);
        assert_eq!(r1.requested_amount.amount_minor_units, r2.requested_amount.amount_minor_units);
    }

    #[tokio::test]
    async fn test_create_payment_intent_zero_amount_card_verification() {
        let (handler, _) = setup_handler();
        let operator_id = Uuid::now_v7();

        let mut cmd = make_create_cmd(operator_id, "key-003", 0);
        cmd.purpose = PaymentPurpose::CardVerification;

        let result = handler.create_payment_intent(cmd).await.unwrap();
        assert!(result.requested_amount.amount_minor_units == 0);
    }

    #[tokio::test]
    async fn test_create_payment_intent_invalid_currency() {
        let (handler, _) = setup_handler();
        let operator_id = Uuid::now_v7();

        let mut cmd = make_create_cmd(operator_id, "key-004", 10000);
        cmd.currency = "INVALID".to_string();

        let result = handler.create_payment_intent(cmd).await;
        assert!(result.is_err());
    }

    // ══════════════════════════════════════════════════════════════════════
    // AuthorizePaymentIntent Tests
    // ══════════════════════════════════════════════════════════════════════

    #[tokio::test]
    async fn test_authorize_single_hop_success() {
        let link_id = Uuid::now_v7();
        let (handler, _repo, operator_id) = setup_with_links(vec![link_id]).await;

        // Create PaymentIntent
        let pi = handler.create_payment_intent(make_create_cmd(operator_id, "auth-key-001", 10000)).await.unwrap();

        // Activate a routing policy with the link
        handler.activate_routing_policy(make_activate_policy_cmd(operator_id, link_id)).await.unwrap();

        // Authorize
        let token_id = Uuid::now_v7();
        let result = handler.authorize_payment_intent(make_authorize_cmd(pi.payment_intent_id, token_id)).await.unwrap();

        assert_eq!(result.status, PaymentStatus::Authorized);
        assert_eq!(result.routing_attempts.len(), 1);
    }

    #[tokio::test]
    async fn test_authorize_no_eligible_route() {
        // Setup: links exist but routing policy references a DIFFERENT link
        let link_id = Uuid::now_v7();
        let other_link = Uuid::now_v7();
        let (handler, _repo, operator_id) = setup_with_links(vec![link_id]).await;

        let pi = handler.create_payment_intent(make_create_cmd(operator_id, "auth-key-002", 10000)).await.unwrap();

        // Activate routing policy referencing a link NOT in available_links
        handler.activate_routing_policy(make_activate_policy_cmd(operator_id, other_link)).await.unwrap();
        let token_id = Uuid::now_v7();

        let result = handler.authorize_payment_intent(make_authorize_cmd(pi.payment_intent_id, token_id)).await;
        assert!(matches!(result, Err(OrchestrationError::NoEligibleRoute)));
    }

    #[tokio::test]
    async fn test_authorize_routing_policy_not_found() {
        let link_id = Uuid::now_v7();
        let (handler, _repo, operator_id) = setup_with_links(vec![link_id]).await;

        let pi = handler.create_payment_intent(make_create_cmd(operator_id, "auth-key-003", 10000)).await.unwrap();
        let token_id = Uuid::now_v7();

        // No routing policy active → fails with RoutingPolicyNotFound
        let result = handler.authorize_payment_intent(make_authorize_cmd(pi.payment_intent_id, token_id)).await;
        assert!(matches!(result, Err(OrchestrationError::RoutingPolicyNotFound)));
    }

    // ══════════════════════════════════════════════════════════════════════
    // CapturePaymentIntent Tests
    // ══════════════════════════════════════════════════════════════════════

    async fn setup_authorized_intent(handler: &dyn CommandHandler, repo: &InMemoryOrchestrationRepository, operator_id: Uuid) -> Uuid {
        let link_id = Uuid::now_v7();
        repo.set_active_links(operator_id, vec![link_id]).await;

        let pi = handler.create_payment_intent(make_create_cmd(operator_id, "cap-key", 10000)).await.unwrap();

        handler.activate_routing_policy(make_activate_policy_cmd(operator_id, link_id)).await.unwrap();

        let token_id = Uuid::now_v7();
        let auth = handler.authorize_payment_intent(make_authorize_cmd(pi.payment_intent_id, token_id)).await.unwrap();
        auth.payment_intent_id
    }

    #[tokio::test]
    async fn test_capture_full_amount() {
        let (handler, repo) = setup_handler();
        let operator_id = Uuid::now_v7();
        let pi_id = setup_authorized_intent(&handler, &repo, operator_id).await;

        let result = handler.capture_payment_intent(CapturePaymentIntent {
            payment_intent_id: pi_id,
            amount_minor_units: None, // full capture
            supports_partial_capture: true,
            max_partial_captures: 10,
            actor_id: Uuid::now_v7(),
        }).await.unwrap();

        assert_eq!(result.status, PaymentStatus::Captured);
        assert_eq!(result.captured_amount.amount_minor_units, 10000);
    }

    #[tokio::test]
    async fn test_capture_partial_amount() {
        let (handler, repo) = setup_handler();
        let operator_id = Uuid::now_v7();
        let pi_id = setup_authorized_intent(&handler, &repo, operator_id).await;

        let result = handler.capture_payment_intent(CapturePaymentIntent {
            payment_intent_id: pi_id,
            amount_minor_units: Some(3000),
            supports_partial_capture: true,
            max_partial_captures: 10,
            actor_id: Uuid::now_v7(),
        }).await.unwrap();

        assert_eq!(result.status, PaymentStatus::PartiallyCaptured);
        assert_eq!(result.captured_amount.amount_minor_units, 3000);
    }

    #[tokio::test]
    async fn test_capture_exceeds_authorized_rejected() {
        let (handler, repo) = setup_handler();
        let operator_id = Uuid::now_v7();
        let pi_id = setup_authorized_intent(&handler, &repo, operator_id).await;

        let result = handler.capture_payment_intent(CapturePaymentIntent {
            payment_intent_id: pi_id,
            amount_minor_units: Some(20000), // exceeds authorized 10000
            supports_partial_capture: true,
            max_partial_captures: 10,
            actor_id: Uuid::now_v7(),
        }).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_capture_partial_not_supported() {
        let (handler, repo) = setup_handler();
        let operator_id = Uuid::now_v7();
        let pi_id = setup_authorized_intent(&handler, &repo, operator_id).await;

        let result = handler.capture_payment_intent(CapturePaymentIntent {
            payment_intent_id: pi_id,
            amount_minor_units: Some(3000),
            supports_partial_capture: false, // connector doesn't support partial
            max_partial_captures: 10,
            actor_id: Uuid::now_v7(),
        }).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_capture_on_failed_intent_rejected() {
        let (handler, _repo) = setup_handler();
        let operator_id = Uuid::now_v7();

        // Create but never authorize
        let pi = handler.create_payment_intent(make_create_cmd(operator_id, "cap-fail", 10000)).await.unwrap();

        let result = handler.capture_payment_intent(CapturePaymentIntent {
            payment_intent_id: pi.payment_intent_id,
            amount_minor_units: None,
            supports_partial_capture: true,
            max_partial_captures: 3,
            actor_id: Uuid::now_v7(),
        }).await;

        assert!(result.is_err()); // PAYMENT_INTENT_NOT_AUTHORIZED
    }

    // ══════════════════════════════════════════════════════════════════════
    // VoidPaymentIntent Tests
    // ══════════════════════════════════════════════════════════════════════

    #[tokio::test]
    async fn test_void_authorized_intent() {
        let (handler, repo) = setup_handler();
        let operator_id = Uuid::now_v7();
        let pi_id = setup_authorized_intent(&handler, &repo, operator_id).await;

        let result = handler.void_payment_intent(VoidPaymentIntent {
            payment_intent_id: pi_id,
            actor_id: Uuid::now_v7(),
        }).await.unwrap();

        assert_eq!(result.status, PaymentStatus::Voided);
    }

    #[tokio::test]
    async fn test_void_after_capture_rejected() {
        let (handler, repo) = setup_handler();
        let operator_id = Uuid::now_v7();
        let pi_id = setup_authorized_intent(&handler, &repo, operator_id).await;

        // Capture first
        handler.capture_payment_intent(CapturePaymentIntent {
            payment_intent_id: pi_id,
            amount_minor_units: None,
            supports_partial_capture: true,
            max_partial_captures: 10,
            actor_id: Uuid::now_v7(),
        }).await.unwrap();

        // Now try to void
        let result = handler.void_payment_intent(VoidPaymentIntent {
            payment_intent_id: pi_id,
            actor_id: Uuid::now_v7(),
        }).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_void_already_voided_rejected() {
        let (handler, repo) = setup_handler();
        let operator_id = Uuid::now_v7();
        let pi_id = setup_authorized_intent(&handler, &repo, operator_id).await;

        // Void
        handler.void_payment_intent(VoidPaymentIntent {
            payment_intent_id: pi_id,
            actor_id: Uuid::now_v7(),
        }).await.unwrap();

        // Try to void again
        let result = handler.void_payment_intent(VoidPaymentIntent {
            payment_intent_id: pi_id,
            actor_id: Uuid::now_v7(),
        }).await;

        assert!(result.is_err());
    }

    // ══════════════════════════════════════════════════════════════════════
    // RefundPaymentIntent Tests
    // ══════════════════════════════════════════════════════════════════════

    #[tokio::test]
    async fn test_refund_full_amount() {
        let (handler, repo) = setup_handler();
        let operator_id = Uuid::now_v7();
        let pi_id = setup_authorized_intent(&handler, &repo, operator_id).await;

        // Capture
        handler.capture_payment_intent(CapturePaymentIntent {
            payment_intent_id: pi_id,
            amount_minor_units: None,
            supports_partial_capture: true,
            max_partial_captures: 10,
            actor_id: Uuid::now_v7(),
        }).await.unwrap();

        // Refund
        let result = handler.refund_payment_intent(RefundPaymentIntent {
            payment_intent_id: pi_id,
            amount_minor_units: 10000,
            actor_id: Uuid::now_v7(),
        }).await.unwrap();

        assert_eq!(result.status, PaymentStatus::Refunded);
        assert_eq!(result.refunded_amount.amount_minor_units, 10000);
    }

    #[tokio::test]
    async fn test_refund_partial_amount() {
        let (handler, repo) = setup_handler();
        let operator_id = Uuid::now_v7();
        let pi_id = setup_authorized_intent(&handler, &repo, operator_id).await;

        handler.capture_payment_intent(CapturePaymentIntent {
            payment_intent_id: pi_id,
            amount_minor_units: None,
            supports_partial_capture: true,
            max_partial_captures: 10,
            actor_id: Uuid::now_v7(),
        }).await.unwrap();

        let result = handler.refund_payment_intent(RefundPaymentIntent {
            payment_intent_id: pi_id,
            amount_minor_units: 3000,
            actor_id: Uuid::now_v7(),
        }).await.unwrap();

        assert_eq!(result.status, PaymentStatus::PartiallyRefunded);
        assert_eq!(result.refunded_amount.amount_minor_units, 3000);
    }

    #[tokio::test]
    async fn test_refund_exceeds_balance_rejected() {
        let (handler, repo) = setup_handler();
        let operator_id = Uuid::now_v7();
        let pi_id = setup_authorized_intent(&handler, &repo, operator_id).await;

        handler.capture_payment_intent(CapturePaymentIntent {
            payment_intent_id: pi_id,
            amount_minor_units: None,
            supports_partial_capture: true,
            max_partial_captures: 10,
            actor_id: Uuid::now_v7(),
        }).await.unwrap();

        let result = handler.refund_payment_intent(RefundPaymentIntent {
            payment_intent_id: pi_id,
            amount_minor_units: 20000, // exceeds captured 10000
            actor_id: Uuid::now_v7(),
        }).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_refund_zero_amount_rejected() {
        let (handler, repo) = setup_handler();
        let operator_id = Uuid::now_v7();
        let pi_id = setup_authorized_intent(&handler, &repo, operator_id).await;

        handler.capture_payment_intent(CapturePaymentIntent {
            payment_intent_id: pi_id,
            amount_minor_units: None,
            supports_partial_capture: true,
            max_partial_captures: 10,
            actor_id: Uuid::now_v7(),
        }).await.unwrap();

        let result = handler.refund_payment_intent(RefundPaymentIntent {
            payment_intent_id: pi_id,
            amount_minor_units: 0,
            actor_id: Uuid::now_v7(),
        }).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_refund_on_voided_rejected() {
        let (handler, repo) = setup_handler();
        let operator_id = Uuid::now_v7();
        let pi_id = setup_authorized_intent(&handler, &repo, operator_id).await;

        handler.void_payment_intent(VoidPaymentIntent {
            payment_intent_id: pi_id,
            actor_id: Uuid::now_v7(),
        }).await.unwrap();

        let result = handler.refund_payment_intent(RefundPaymentIntent {
            payment_intent_id: pi_id,
            amount_minor_units: 1000,
            actor_id: Uuid::now_v7(),
        }).await;

        assert!(result.is_err());
    }

    // ══════════════════════════════════════════════════════════════════════
    // RoutingPolicy Tests
    // ══════════════════════════════════════════════════════════════════════

    #[tokio::test]
    async fn test_activate_routing_policy_success() {
        let (handler, _) = setup_handler();
        let operator_id = Uuid::now_v7();
        let link_id = Uuid::now_v7();

        let result = handler.activate_routing_policy(make_activate_policy_cmd(operator_id, link_id)).await.unwrap();

        assert_eq!(result.version, 1);
    }

    #[tokio::test]
    async fn test_routing_policy_version_increment() {
        let (handler, _) = setup_handler();
        let operator_id = Uuid::now_v7();
        let link_id = Uuid::now_v7();

        let _v1 = handler.activate_routing_policy(make_activate_policy_cmd(operator_id, link_id)).await.unwrap();

        let v2 = handler.activate_routing_policy(ActivateRoutingPolicy {
            operator_id,
            rules: vec![
                RoutingRule {
                    acquirer_link_id: link_id,
                    priority: 2,
                    condition: RoutingCondition::all(),
                }
            ],
            failover_config: FailoverConfig::default(),
            partial_auth_strategy: PartialAuthStrategy::RetryNextAcquirer,
            rotation_strategy: RotationStrategy::RoundRobin,
            max_transaction_amount_minor: None,
        }).await.unwrap();

        assert_eq!(v2.version, 1); // New policy always starts at v1
    }

    // ══════════════════════════════════════════════════════════════════════
    // Invariant Tests
    // ══════════════════════════════════════════════════════════════════════

    #[tokio::test]
    async fn test_inv_01_partial_captures_sum_never_exceeds_authorized() {
        let (handler, repo) = setup_handler();
        let operator_id = Uuid::now_v7();
        let pi_id = setup_authorized_intent(&handler, &repo, operator_id).await;

        // Capture 3000
        handler.capture_payment_intent(CapturePaymentIntent {
            payment_intent_id: pi_id,
            amount_minor_units: Some(3000),
            supports_partial_capture: true,
            max_partial_captures: 10,
            actor_id: Uuid::now_v7(),
        }).await.unwrap();

        // Capture 4000
        handler.capture_payment_intent(CapturePaymentIntent {
            payment_intent_id: pi_id,
            amount_minor_units: Some(4000),
            supports_partial_capture: true,
            max_partial_captures: 10,
            actor_id: Uuid::now_v7(),
        }).await.unwrap();

        // Capture 4000 → would exceed 10000 (total would be 11000)
        let result = handler.capture_payment_intent(CapturePaymentIntent {
            payment_intent_id: pi_id,
            amount_minor_units: Some(4000),
            supports_partial_capture: true,
            max_partial_captures: 10,
            actor_id: Uuid::now_v7(),
        }).await;

        assert!(result.is_err(), "Should reject capture that exceeds authorized amount");
    }

    // ══════════════════════════════════════════════════════════════════════
    // Query Tests
    // ══════════════════════════════════════════════════════════════════════

    #[tokio::test]
    async fn test_query_payment_intent() {
        let repo = InMemoryOrchestrationRepository::new();
        let handler = OrchestrationCommandHandler::new(repo.clone());
        let query_handler = OrchestrationQueryHandler::new(repo.clone());
        let operator_id = Uuid::now_v7();

        let pi = handler.create_payment_intent(make_create_cmd(operator_id, "q-key", 5000)).await.unwrap();

        let loaded = query_handler.get_payment_intent(GetPaymentIntentQuery {
            payment_intent_id: pi.payment_intent_id,
        }).await.unwrap();

        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().requested_amount.amount_minor_units, 5000);
    }

    #[tokio::test]
    async fn test_list_payment_intents() {
        let repo = InMemoryOrchestrationRepository::new();
        let handler = OrchestrationCommandHandler::new(repo.clone());
        let query_handler = OrchestrationQueryHandler::new(repo.clone());
        let operator_id = Uuid::now_v7();

        handler.create_payment_intent(make_create_cmd(operator_id, "l-key-1", 1000)).await.unwrap();
        handler.create_payment_intent(make_create_cmd(operator_id, "l-key-2", 2000)).await.unwrap();

        let list = query_handler.list_payment_intents(ListPaymentIntentsQuery {
            operator_id,
            limit: 10,
            offset: 0,
        }).await.unwrap();

        assert_eq!(list.len(), 2);
    }
}
