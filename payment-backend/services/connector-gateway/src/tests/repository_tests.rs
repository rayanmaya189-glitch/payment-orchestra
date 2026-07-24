//! Repository tests.

use crate::domain;
use crate::repository::{GatewayProfileRepository, InMemoryGatewayProfileRepository};

use super::{sample_gateway_profile, test_profile_id, test_operator_id, test_link_id};

#[tokio::test]
async fn test_repository_save_and_load_profile() {
    let repo = InMemoryGatewayProfileRepository::new();
    let profile = sample_gateway_profile(test_profile_id(), test_operator_id(), test_link_id(), "network_international");

    repo.save(&profile).await.unwrap();
    let loaded = repo.load(profile.profile_id).await.unwrap();

    assert!(loaded.is_some());
    assert_eq!(loaded.unwrap().connector_id, "network_international");
}

#[tokio::test]
async fn test_repository_find_active_by_operator() {
    let repo = InMemoryGatewayProfileRepository::new();
    let operator_id = test_operator_id();
    let profile = sample_gateway_profile(test_profile_id(), operator_id, test_link_id(), "network_international");

    repo.save(&profile).await.unwrap();
    let active = repo.find_active_for_operator(operator_id).await.unwrap();

    assert_eq!(active.len(), 1);
}

#[tokio::test]
async fn test_repository_daily_volume() {
    let repo = InMemoryGatewayProfileRepository::new();
    let profile_id = test_profile_id();
    let amount = domain::Money { amount_minor_units: 10000, currency: "AED".into() };

    let vol_before = repo.check_daily_volume(profile_id).await.unwrap();
    assert_eq!(vol_before.amount_minor_units, 0);

    repo.increment_daily_volume(profile_id, amount).await.unwrap();
    let vol_after = repo.check_daily_volume(profile_id).await.unwrap();
    assert_eq!(vol_after.amount_minor_units, 10000);
}
