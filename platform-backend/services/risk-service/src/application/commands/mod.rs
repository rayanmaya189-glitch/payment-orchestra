use uuid::Uuid;

/// Command to trigger a risk assessment for a payment intent.
#[derive(Debug, Clone)]
pub struct AssessPaymentRiskCommand {
    pub payment_intent_id: Uuid,
    pub operator_id: Uuid,
    pub amount_minor_units: i64,
    pub currency: String,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    /// ISO 3166-1 alpha-2 country code resolved from IP geolocation.
    pub country_code: Option<String>,
    /// Merchant's home country for geo-mismatch checks.
    pub merchant_country: Option<String>,
    /// Whether the entity is whitelisted (pre-resolved by caller).
    pub is_whitelisted: bool,
    /// Whether the entity is blacklisted (pre-resolved by caller).
    pub is_blacklisted: bool,
    /// Recent transaction count from the same IP within the velocity window.
    pub recent_tx_count_from_ip: u32,
    /// Recent transaction count from the same card fingerprint.
    pub recent_tx_count_from_card: u32,
    /// The authenticated principal's role for ABAC checks.
    pub principal_role: String,
    /// The operator scope the principal is authorized to access.
    pub principal_operator_id: Uuid,
}

impl AssessPaymentRiskCommand {
    /// Returns true when the principal is allowed to assess risk for the
    /// requested operator.
    pub fn is_authorized(&self) -> bool {
        match self.principal_role.as_str() {
            "platform_admin" | "compliance_officer" => true,
            "operator_admin" => self.principal_operator_id == self.operator_id,
            "api_client" => self.principal_operator_id == self.operator_id,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_admin_can_assess_any_operator() {
        let cmd = AssessPaymentRiskCommand {
            payment_intent_id: Uuid::now_v7(),
            operator_id: Uuid::now_v7(),
            amount_minor_units: 1000,
            currency: "AED".into(),
            ip_address: None,
            user_agent: None,
            country_code: None,
            merchant_country: None,
            is_whitelisted: false,
            is_blacklisted: false,
            recent_tx_count_from_ip: 0,
            recent_tx_count_from_card: 0,
            principal_role: "platform_admin".into(),
            principal_operator_id: Uuid::now_v7(),
        };
        assert!(cmd.is_authorized());
    }

    #[test]
    fn operator_admin_scoped() {
        let op = Uuid::now_v7();
        let mut cmd = AssessPaymentRiskCommand {
            payment_intent_id: Uuid::now_v7(),
            operator_id: op,
            amount_minor_units: 1000,
            currency: "AED".into(),
            ip_address: None,
            user_agent: None,
            country_code: None,
            merchant_country: None,
            is_whitelisted: false,
            is_blacklisted: false,
            recent_tx_count_from_ip: 0,
            recent_tx_count_from_card: 0,
            principal_role: "operator_admin".into(),
            principal_operator_id: op,
        };
        assert!(cmd.is_authorized());
        // Different operator_id → denied.
        cmd.operator_id = Uuid::now_v7();
        assert!(!cmd.is_authorized());
    }

    #[test]
    fn read_only_denied() {
        let cmd = AssessPaymentRiskCommand {
            payment_intent_id: Uuid::now_v7(),
            operator_id: Uuid::now_v7(),
            amount_minor_units: 1000,
            currency: "AED".into(),
            ip_address: None,
            user_agent: None,
            country_code: None,
            merchant_country: None,
            is_whitelisted: false,
            is_blacklisted: false,
            recent_tx_count_from_ip: 0,
            recent_tx_count_from_card: 0,
            principal_role: "read_only".into(),
            principal_operator_id: Uuid::now_v7(),
        };
        assert!(!cmd.is_authorized());
    }
}
