//! Authorize PaymentIntent tests: authorization, routing, no-route.

use uuid::Uuid;

use crate::domain::*;
use crate::commands::*;
use crate::repository::*;

use super::{make_authorize_cmd, make_create_cmd, make_activate_policy_cmd, setup_with_links};

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
