//! Integration tests for middleware — ABAC, rate limiting, SSRF, headers.

#[cfg(test)]
mod middleware_tests {
    use platform_middleware::abac::{AbacContext, evaluate_policy};
    use uuid::Uuid;

    #[test]
    fn test_abac_platform_admin_can_create_operator() {
        let ctx = AbacContext {
            principal_id: Uuid::now_v7(),
            role: "platform_admin".to_string(),
            action: "create".to_string(),
            resource: "operator".to_string(),
            resource_id: None,
            amount: None,
            ip_address: None,
            operator_id: None,
        };
        assert!(evaluate_policy(&ctx).is_ok());
    }

    #[test]
    fn test_abac_read_only_cannot_create_operator() {
        let ctx = AbacContext {
            principal_id: Uuid::now_v7(),
            role: "read_only".to_string(),
            action: "create".to_string(),
            resource: "operator".to_string(),
            resource_id: None,
            amount: None,
            ip_address: None,
            operator_id: None,
        };
        assert!(evaluate_policy(&ctx).is_err());
    }

    #[test]
    fn test_abac_api_client_can_create_payment() {
        let ctx = AbacContext {
            principal_id: Uuid::now_v7(),
            role: "api_client".to_string(),
            action: "create".to_string(),
            resource: "payment".to_string(),
            resource_id: None,
            amount: Some(5000),
            ip_address: None,
            operator_id: Some(Uuid::now_v7()),
        };
        assert!(evaluate_policy(&ctx).is_ok());
    }

    #[test]
    fn test_abac_compliance_officer_can_review_kyb() {
        let ctx = AbacContext {
            principal_id: Uuid::now_v7(),
            role: "compliance_officer".to_string(),
            action: "review".to_string(),
            resource: "kyb".to_string(),
            resource_id: None,
            amount: None,
            ip_address: None,
            operator_id: None,
        };
        assert!(evaluate_policy(&ctx).is_ok());
    }

    #[test]
    fn test_abac_operator_admin_can_read_own_operator() {
        let operator_id = Uuid::now_v7();
        let ctx = AbacContext {
            principal_id: Uuid::now_v7(),
            role: "operator_admin".to_string(),
            action: "read".to_string(),
            resource: "operator".to_string(),
            resource_id: None,
            amount: None,
            ip_address: None,
            operator_id: Some(operator_id),
        };
        assert!(evaluate_policy(&ctx).is_ok());
    }
}
