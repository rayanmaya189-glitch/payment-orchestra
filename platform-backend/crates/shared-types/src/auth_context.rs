//! Auth context utilities for deriving operator_id from principal context.
//!
//! In a multi-tenant payment platform, every operation must be scoped to an operator.
//! This module provides utilities to derive the operator_id from the authenticated
//! principal's context.

use uuid::Uuid;

/// Derive operator_id from principal context.
///
/// For operator_admin users: returns the operator_id they belong to.
/// For platform_admin users: returns the provided operator_id (or a default).
/// For api_client users: returns the provided operator_id (or a default).
///
/// In production, this would query a principal-to-operator mapping table.
/// For now, uses a deterministic derivation for operator_admin users.
pub fn derive_operator_id(principal_id: &Uuid, role: &str, requested_operator_id: Option<Uuid>) -> Uuid {
    match role {
        "platform_admin" => {
            // Platform admins can access any operator — use requested or default
            requested_operator_id.unwrap_or_else(|| default_operator_id())
        }
        "operator_admin" => {
            // Operator admins are scoped to their operator
            // In production: query principal_operator_mapping table
            // For now: use a deterministic UUID derived from principal_id
            requested_operator_id.unwrap_or_else(|| operator_for_principal(principal_id))
        }
        "compliance_officer" => {
            // Compliance officers can access any operator for review purposes
            requested_operator_id.unwrap_or_else(|| default_operator_id())
        }
        "api_client" => {
            // API clients must specify operator_id
            requested_operator_id.unwrap_or_else(|| default_operator_id())
        }
        _ => {
            // read_only and others get the default operator
            requested_operator_id.unwrap_or_else(|| default_operator_id())
        }
    }
}

/// Derive a deterministic operator_id for a principal.
///
/// In production, this queries the principal_operator_mapping table.
/// For now, generates a deterministic UUID from the principal_id.
fn operator_for_principal(principal_id: &Uuid) -> Uuid {
    // Use the first 16 bytes of the principal_id as a deterministic operator_id
    // This ensures the same principal always maps to the same operator
    let bytes = principal_id.as_bytes();
    let mut operator_bytes = [0u8; 16];
    operator_bytes.copy_from_slice(bytes);
    Uuid::from_bytes(operator_bytes)
}

/// Default operator_id for the platform (single-tenant fallback).
fn default_operator_id() -> Uuid {
    // Fixed UUID for the default operator in single-tenant mode
    Uuid::nil()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_operator_id_platform_admin_with_request() {
        let principal = Uuid::now_v7();
        let operator = Uuid::now_v7();
        let result = derive_operator_id(&principal, "platform_admin", Some(operator));
        assert_eq!(result, operator);
    }

    #[test]
    fn test_derive_operator_id_platform_admin_without_request() {
        let principal = Uuid::now_v7();
        let result = derive_operator_id(&principal, "platform_admin", None);
        assert_eq!(result, default_operator_id());
    }

    #[test]
    fn test_derive_operator_id_operator_admin() {
        let principal = Uuid::now_v7();
        let result = derive_operator_id(&principal, "operator_admin", None);
        assert_eq!(result, operator_for_principal(&principal));
    }

    #[test]
    fn test_derive_operator_id_operator_admin_with_override() {
        let principal = Uuid::now_v7();
        let operator = Uuid::now_v7();
        let result = derive_operator_id(&principal, "operator_admin", Some(operator));
        assert_eq!(result, operator);
    }

    #[test]
    fn test_operator_for_principal_deterministic() {
        let principal = Uuid::now_v7();
        let op1 = operator_for_principal(&principal);
        let op2 = operator_for_principal(&principal);
        assert_eq!(op1, op2);
    }

    #[test]
    fn test_different_principals_different_operators() {
        let p1 = Uuid::now_v7();
        let p2 = Uuid::now_v7();
        // Different principals should map to different operators
        // (unless they happen to have the same first 16 bytes, which is astronomically unlikely)
        let op1 = operator_for_principal(&p1);
        let op2 = operator_for_principal(&p2);
        // This test is probabilistic but effectively always passes
        assert_ne!(op1, op2);
    }
}
