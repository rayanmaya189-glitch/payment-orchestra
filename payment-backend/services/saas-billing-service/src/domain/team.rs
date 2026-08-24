//! Tenant Team Member domain model.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::error::SaaSbillingError;

/// A team member invitation for a tenant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantTeamMember {
    pub membership_id: Uuid,
    pub operator_id: Uuid,
    pub principal_id: Uuid,
    pub role: String,
    pub invited_by: Uuid,
    pub invited_at: DateTime<Utc>,
    pub accepted_at: Option<DateTime<Utc>>,
    pub status: TeamMemberStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Team member status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TeamMemberStatus {
    Pending,
    Active,
    Suspended,
}

impl TeamMemberStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Active => "active",
            Self::Suspended => "suspended",
        }
    }

    pub fn parse_str(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(Self::Pending),
            "active" => Some(Self::Active),
            "suspended" => Some(Self::Suspended),
            _ => None,
        }
    }
}

impl std::fmt::Display for TeamMemberStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Available team roles.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TeamRole {
    Owner,
    Admin,
    Manager,
    Developer,
    Viewer,
}

impl TeamRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Owner => "owner",
            Self::Admin => "admin",
            Self::Manager => "manager",
            Self::Developer => "developer",
            Self::Viewer => "viewer",
        }
    }

    pub fn parse_str(s: &str) -> Option<Self> {
        match s {
            "owner" => Some(Self::Owner),
            "admin" => Some(Self::Admin),
            "manager" => Some(Self::Manager),
            "developer" => Some(Self::Developer),
            "viewer" => Some(Self::Viewer),
            _ => None,
        }
    }

    /// Check if this role can manage team members.
    pub fn can_manage_team(&self) -> bool {
        matches!(self, Self::Owner | Self::Admin)
    }

    /// Check if this role can manage billing.
    pub fn can_manage_billing(&self) -> bool {
        matches!(self, Self::Owner)
    }

    /// Check if this role can manage settings.
    pub fn can_manage_settings(&self) -> bool {
        matches!(self, Self::Owner | Self::Admin)
    }
}

impl std::fmt::Display for TeamRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl TenantTeamMember {
    /// Create a new team member invitation.
    pub fn new(
        operator_id: Uuid,
        principal_id: Uuid,
        role: &str,
        invited_by: Uuid,
    ) -> Self {
        let now = Utc::now();
        Self {
            membership_id: Uuid::now_v7(),
            operator_id,
            principal_id,
            role: role.into(),
            invited_by,
            invited_at: now,
            accepted_at: None,
            status: TeamMemberStatus::Pending,
            created_at: now,
            updated_at: now,
        }
    }

    /// Accept the invitation.
    pub fn accept(&mut self) -> Result<(), SaaSbillingError> {
        if self.status != TeamMemberStatus::Pending {
            return Err(SaaSbillingError::InvalidStatusTransition {
                from: self.status.to_string(),
                to: "active".into(),
            });
        }
        self.status = TeamMemberStatus::Active;
        self.accepted_at = Some(Utc::now());
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Suspend the team member.
    pub fn suspend(&mut self) -> Result<(), SaaSbillingError> {
        if self.status != TeamMemberStatus::Active {
            return Err(SaaSbillingError::InvalidStatusTransition {
                from: self.status.to_string(),
                to: "suspended".into(),
            });
        }
        self.status = TeamMemberStatus::Suspended;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Reactivate the team member.
    pub fn reactivate(&mut self) -> Result<(), SaaSbillingError> {
        if self.status != TeamMemberStatus::Suspended {
            return Err(SaaSbillingError::InvalidStatusTransition {
                from: self.status.to_string(),
                to: "active".into(),
            });
        }
        self.status = TeamMemberStatus::Active;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Change the team member's role.
    pub fn change_role(&mut self, new_role: &str) -> Result<(), SaaSbillingError> {
        TeamRole::parse_str(new_role).ok_or_else(|| SaaSbillingError::ValidationError(
            format!("Invalid role: {}", new_role),
        ))?;
        self.role = new_role.into();
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Check if the team member has a specific role.
    pub fn has_role(&self, role: &str) -> bool {
        self.role == role
    }

    /// Check if the team member is active.
    pub fn is_active(&self) -> bool {
        self.status == TeamMemberStatus::Active
    }
}

impl std::fmt::Display for TenantTeamMember {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "TeamMember({} role={} status={})",
            self.principal_id, self.role, self.status
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_team_member() -> TenantTeamMember {
        TenantTeamMember::new(
            Uuid::now_v7(),
            Uuid::now_v7(),
            "developer",
            Uuid::now_v7(),
        )
    }

    #[test]
    fn test_team_member_new() {
        let member = test_team_member();
        assert_eq!(member.status, TeamMemberStatus::Pending);
        assert_eq!(member.role, "developer");
        assert!(member.accepted_at.is_none());
    }

    #[test]
    fn test_team_member_accept() {
        let mut member = test_team_member();
        assert!(member.accept().is_ok());
        assert_eq!(member.status, TeamMemberStatus::Active);
        assert!(member.accepted_at.is_some());
    }

    #[test]
    fn test_team_member_accept_non_pending_fails() {
        let mut member = test_team_member();
        member.status = TeamMemberStatus::Active;
        assert!(member.accept().is_err());
    }

    #[test]
    fn test_team_member_suspend() {
        let mut member = test_team_member();
        member.status = TeamMemberStatus::Active;
        assert!(member.suspend().is_ok());
        assert_eq!(member.status, TeamMemberStatus::Suspended);
    }

    #[test]
    fn test_team_member_suspend_non_active_fails() {
        let mut member = test_team_member();
        assert!(member.suspend().is_err());
    }

    #[test]
    fn test_team_member_reactivate() {
        let mut member = test_team_member();
        member.status = TeamMemberStatus::Suspended;
        assert!(member.reactivate().is_ok());
        assert_eq!(member.status, TeamMemberStatus::Active);
    }

    #[test]
    fn test_team_member_reactivate_non_suspended_fails() {
        let mut member = test_team_member();
        assert!(member.reactivate().is_err());
    }

    #[test]
    fn test_team_member_change_role() {
        let mut member = test_team_member();
        assert!(member.change_role("admin").is_ok());
        assert_eq!(member.role, "admin");
    }

    #[test]
    fn test_team_member_change_role_invalid_fails() {
        let mut member = test_team_member();
        assert!(member.change_role("invalid_role").is_err());
    }

    #[test]
    fn test_team_role_can_manage_team() {
        assert!(TeamRole::Owner.can_manage_team());
        assert!(TeamRole::Admin.can_manage_team());
        assert!(!TeamRole::Manager.can_manage_team());
        assert!(!TeamRole::Developer.can_manage_team());
        assert!(!TeamRole::Viewer.can_manage_team());
    }

    #[test]
    fn test_team_role_can_manage_billing() {
        assert!(TeamRole::Owner.can_manage_billing());
        assert!(!TeamRole::Admin.can_manage_billing());
        assert!(!TeamRole::Manager.can_manage_billing());
    }

    #[test]
    fn test_team_member_display() {
        let member = test_team_member();
        assert!(format!("{}", member).contains("developer"));
        assert!(format!("{}", member).contains("pending"));
    }
}
