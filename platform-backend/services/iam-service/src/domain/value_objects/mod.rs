use platform_error::ValidationError;

pub mod totp;
pub mod webauthn;

/// Email value object with validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Email(String);

impl Email {
    pub fn new(email: &str) -> Result<Self, ValidationError> {
        if email.is_empty() || email.len() > 254 {
            return Err(ValidationError::MissingField("email".to_string()));
        }
        if !email.contains('@') || !email.contains('.') {
            return Err(ValidationError::MissingField("email".to_string()));
        }
        Ok(Self(email.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for Email {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Password value object — enforces minimum length.
#[derive(Debug, Clone)]
pub struct Password(String);

impl Password {
    pub fn new(password: &str, min_length: usize) -> Result<Self, ValidationError> {
        if password.len() < min_length {
            return Err(ValidationError::MissingField(format!(
                "password must be at least {} characters",
                min_length
            )));
        }
        Ok(Self(password.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Principal role.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrincipalRole {
    PlatformAdmin,
    OperatorAdmin,
    ComplianceOfficer,
    ApiClient,
    ReadOnly,
}

impl PrincipalRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::PlatformAdmin => "platform_admin",
            Self::OperatorAdmin => "operator_admin",
            Self::ComplianceOfficer => "compliance_officer",
            Self::ApiClient => "api_client",
            Self::ReadOnly => "read_only",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "platform_admin" => Self::PlatformAdmin,
            "compliance_officer" => Self::ComplianceOfficer,
            "api_client" => Self::ApiClient,
            "read_only" => Self::ReadOnly,
            _ => Self::OperatorAdmin,
        }
    }

    /// Check if this role can perform the given action on the resource.
    pub fn can_perform(&self, action: &str, resource: &str) -> bool {
        match self {
            Self::PlatformAdmin => true, // Platform admins can do everything
            Self::OperatorAdmin => match action {
                "read" => true,
                "create" | "update" => !matches!(resource, "operator" | "compliance"),
                "delete" => false,
                _ => false,
            },
            Self::ComplianceOfficer => matches!(
                (action, resource),
                ("read", _) | ("update", "kyb_case")
            ),
            Self::ApiClient => matches!(action, "read" | "create"),
            Self::ReadOnly => action == "read",
        }
    }
}

/// Principal status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrincipalStatus {
    Active,
    Suspended,
    Locked,
    Deleted,
}

impl PrincipalStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Suspended => "suspended",
            Self::Locked => "locked",
            Self::Deleted => "deleted",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "suspended" => Self::Suspended,
            "locked" => Self::Locked,
            "deleted" => Self::Deleted,
            _ => Self::Active,
        }
    }
}

/// MFA method type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MfaMethod {
    Totp,
    WebAuthn,
    Sms,
}

impl MfaMethod {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Totp => "totp",
            Self::WebAuthn => "webauthn",
            Self::Sms => "sms",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "webauthn" => Self::WebAuthn,
            "sms" => Self::Sms,
            _ => Self::Totp,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_email_valid() {
        assert!(Email::new("user@example.com").is_ok());
    }

    #[test]
    fn test_email_invalid() {
        assert!(Email::new("").is_err());
        assert!(Email::new("notanemail").is_err());
    }

    #[test]
    fn test_password_valid() {
        assert!(Password::new("longpassword123", 12).is_ok());
    }

    #[test]
    fn test_password_too_short() {
        assert!(Password::new("short", 12).is_err());
    }

    #[test]
    fn test_role_permissions() {
        assert!(PrincipalRole::PlatformAdmin.can_perform("delete", "operator"));
        assert!(!PrincipalRole::OperatorAdmin.can_perform("delete", "operator"));
        assert!(PrincipalRole::ComplianceOfficer.can_perform("update", "kyb_case"));
        assert!(!PrincipalRole::ReadOnly.can_perform("create", "payment"));
    }
}
