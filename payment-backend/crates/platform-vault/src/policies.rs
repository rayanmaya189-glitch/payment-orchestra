//! Vault policy management for role-based access control.
//!
//! Provides policy definitions and management for controlling access to
//! secrets in HashiCorp Vault.

use serde::{Deserialize, Serialize};


/// Vault policy path capability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathCapability {
    /// Path pattern (e.g., "secret/data/database/*")
    pub path: String,
    /// Allowed capabilities (create, read, update, delete, list, sudo)
    pub capabilities: Vec<String>,
}

/// Vault policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultPolicy {
    /// Policy name
    pub name: String,
    /// Human-readable description
    pub description: Option<String>,
    /// Path capabilities
    pub paths: Vec<PathCapability>,
}

impl VaultPolicy {
    /// Create a new policy
    pub fn new(name: &str, description: Option<&str>) -> Self {
        Self {
            name: name.to_string(),
            description: description.map(String::from),
            paths: Vec::new(),
        }
    }

    /// Add a path capability
    pub fn add_path(mut self, path: &str, capabilities: Vec<&str>) -> Self {
        self.paths.push(PathCapability {
            path: path.to_string(),
            capabilities: capabilities.into_iter().map(String::from).collect(),
        });
        self
    }

    /// Convert to HCL format for Vault
    pub fn to_hcl(&self) -> String {
        let path_names: Vec<&str> = self.paths.iter().map(|p| p.path.as_str()).collect();
        let joined_paths = path_names.join("\" \"");
        let mut hcl = format!("path \"{}\" {{\n", joined_paths);
        for path in &self.paths {
            hcl.push_str(&format!("  capabilities = [{}]\n", 
                path.capabilities.iter()
                    .map(|c| format!("\"{}\"", c))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        hcl.push_str("}\n");
        hcl
    }
}

/// Predefined policies for the payment platform
pub struct PlatformPolicies;

impl PlatformPolicies {
    /// Database read-only policy
    pub fn database_readonly() -> VaultPolicy {
        VaultPolicy::new("database-readonly", Some("Read-only access to database secrets"))
            .add_path("secret/data/database/*", vec!["read", "list"])
    }

    /// Payment connector policy
    pub fn payment_connector() -> VaultPolicy {
        VaultPolicy::new("payment-connector", Some("Access to payment connector secrets"))
            .add_path("secret/data/connectors/*", vec!["read"])
            .add_path("secret/data/payment-gateway/*", vec!["read"])
    }

    /// AI service policy
    pub fn ai_service() -> VaultPolicy {
        VaultPolicy::new("ai-service", Some("Access to AI service secrets"))
            .add_path("secret/data/ai/*", vec!["read"])
            .add_path("secret/data/openai/*", vec!["read"])
    }

    /// Admin policy (full access)
    pub fn admin() -> VaultPolicy {
        VaultPolicy::new("admin", Some("Full admin access to all secrets"))
            .add_path("secret/*", vec!["create", "read", "update", "delete", "list", "sudo"])
            .add_path("sys/*", vec!["read", "list"])
            .add_path("auth/*", vec!["read", "list"])
    }

    /// Operator policy
    pub fn operator() -> VaultPolicy {
        VaultPolicy::new("operator", Some("Operator access to connector and config secrets"))
            .add_path("secret/data/connectors/*", vec!["read"])
            .add_path("secret/data/config/*", vec!["read"])
            .add_path("secret/data/operators/*", vec!["read", "update"])
    }

    /// Get all platform policies
    pub fn all() -> Vec<VaultPolicy> {
        vec![
            Self::database_readonly(),
            Self::payment_connector(),
            Self::ai_service(),
            Self::admin(),
            Self::operator(),
        ]
    }
}

/// Vault policy document in JSON format
#[derive(Debug, Serialize, Deserialize)]
pub struct PolicyDocument {
    #[serde(rename = "policy")]
    pub name: String,
    pub description: Option<String>,
    pub rules: String,
}

impl PolicyDocument {
    /// Create from a VaultPolicy
    pub fn from_policy(policy: &VaultPolicy) -> Self {
        let rules = serde_json::json!({
            "path": policy.paths.iter().map(|p| {
                serde_json::json!({
                    "capabilities": p.capabilities
                })
            }).collect::<Vec<_>>()
        });

        Self {
            name: policy.name.clone(),
            description: policy.description.clone(),
            rules: serde_json::to_string_pretty(&rules).unwrap_or_default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_policy_creation() {
        let policy = VaultPolicy::new("test", Some("Test policy"))
            .add_path("secret/data/test/*", vec!["read", "list"]);
        
        assert_eq!(policy.name, "test");
        assert_eq!(policy.paths.len(), 1);
        assert_eq!(policy.paths[0].capabilities.len(), 2);
    }

    #[test]
    fn test_platform_policies() {
        let policies = PlatformPolicies::all();
        assert!(!policies.is_empty());
        assert!(policies.iter().any(|p| p.name == "admin"));
        assert!(policies.iter().any(|p| p.name == "payment-connector"));
    }
}
