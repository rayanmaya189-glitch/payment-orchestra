//! Comprehensive TDD tests for orchestration-service.
//! Tests follow the spec: failing test → implementation → passing test.

mod authorize_tests;
mod capture_tests;
mod create_tests;
mod routing_tests;
mod void_refund_tests;

pub(crate) mod helpers {
    use uuid::Uuid;

    use crate::domain::*;
    use crate::commands::*;
    use crate::repository::*;

    pub fn setup_handler() -> (OrchestrationCommandHandler<InMemoryOrchestrationRepository>, InMemoryOrchestrationRepository) {
        let repo = InMemoryOrchestrationRepository::new();
        let handler = OrchestrationCommandHandler::new(repo.clone());
        (handler, repo)
    }

    /// Setup handler with active acquirer links for a specific operator.
    pub async fn setup_with_links(link_ids: Vec<Uuid>) -> (OrchestrationCommandHandler<InMemoryOrchestrationRepository>, InMemoryOrchestrationRepository, Uuid) {
        let repo = InMemoryOrchestrationRepository::new();
        let operator_id = Uuid::now_v7();
        repo.set_active_links(operator_id, link_ids).await;
        let handler = OrchestrationCommandHandler::new(repo.clone());
        (handler, repo, operator_id)
    }

    pub fn make_create_cmd(operator_id: Uuid, key: &str, amount: i64) -> CreatePaymentIntent {
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

    pub fn make_authorize_cmd(pi_id: Uuid, token_id: Uuid) -> AuthorizePaymentIntent {
        AuthorizePaymentIntent {
            payment_intent_id: pi_id,
            payment_method_token_id: token_id,
            card_scheme: "Visa".into(),
            actor_id: Uuid::now_v7(),
        }
    }

    pub fn make_activate_policy_cmd(operator_id: Uuid, link_id: Uuid) -> ActivateRoutingPolicy {
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

    pub async fn setup_authorized_intent(handler: &dyn CommandHandler, repo: &InMemoryOrchestrationRepository, operator_id: Uuid) -> Uuid {
        let link_id = Uuid::now_v7();
        repo.set_active_links(operator_id, vec![link_id]).await;

        let pi = handler.create_payment_intent(make_create_cmd(operator_id, "cap-key", 10000)).await.unwrap();

        handler.activate_routing_policy(make_activate_policy_cmd(operator_id, link_id)).await.unwrap();

        let token_id = Uuid::now_v7();
        let auth = handler.authorize_payment_intent(make_authorize_cmd(pi.payment_intent_id, token_id)).await.unwrap();
        auth.payment_intent_id
    }
}

pub(crate) use helpers::*;
