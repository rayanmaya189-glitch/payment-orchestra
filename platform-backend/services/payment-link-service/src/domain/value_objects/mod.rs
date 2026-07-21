#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaymentLinkStatus {
    Active,
    Expired,
    UsedUp,
    Deactivated,
}

impl PaymentLinkStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Expired => "expired",
            Self::UsedUp => "used_up",
            Self::Deactivated => "deactivated",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "expired" => Self::Expired,
            "used_up" => Self::UsedUp,
            "deactivated" => Self::Deactivated,
            _ => Self::Active,
        }
    }

    pub fn can_transition_to(&self, target: &PaymentLinkStatus) -> bool {
        matches!(
            (self, target),
            (Self::Active, Self::Expired)
                | (Self::Active, Self::UsedUp)
                | (Self::Active, Self::Deactivated)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_transitions() {
        assert!(PaymentLinkStatus::Active.can_transition_to(&PaymentLinkStatus::Expired));
        assert!(PaymentLinkStatus::Active.can_transition_to(&PaymentLinkStatus::UsedUp));
        assert!(PaymentLinkStatus::Active.can_transition_to(&PaymentLinkStatus::Deactivated));
        assert!(!PaymentLinkStatus::Expired.can_transition_to(&PaymentLinkStatus::Active));
        assert!(!PaymentLinkStatus::UsedUp.can_transition_to(&PaymentLinkStatus::Active));
        assert!(!PaymentLinkStatus::Deactivated.can_transition_to(&PaymentLinkStatus::Active));
    }

    #[test]
    fn test_status_from_str() {
        assert_eq!(PaymentLinkStatus::from_str("active"), PaymentLinkStatus::Active);
        assert_eq!(PaymentLinkStatus::from_str("expired"), PaymentLinkStatus::Expired);
        assert_eq!(PaymentLinkStatus::from_str("used_up"), PaymentLinkStatus::UsedUp);
        assert_eq!(PaymentLinkStatus::from_str("deactivated"), PaymentLinkStatus::Deactivated);
        assert_eq!(PaymentLinkStatus::from_str("unknown"), PaymentLinkStatus::Active);
    }
}
