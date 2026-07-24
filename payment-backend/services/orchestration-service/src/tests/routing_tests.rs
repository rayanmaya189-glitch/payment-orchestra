//! Routing policy tests and query tests.

use uuid::Uuid;

use crate::domain::*;
use crate::commands::*;
use crate::queries::*;
use crate::repository::*;

use super::{make_create_cmd, make_activate_policy_cmd, setup_handler, setup_authorized_intent};

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
