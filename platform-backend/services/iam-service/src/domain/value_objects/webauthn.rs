//! WebAuthn MFA support (SRS AUTH-011).
//!
//! WebAuthn provides hardware security key authentication (FIDO2/WebAuthn).
//! This module defines the domain types and stubs for WebAuthn integration.
//!
//! In production, this would integrate with a WebAuthn library like `webauthn-rs`
//! or `passkey` for credential management.

use chrono::{DateTime, Utc};
use uuid::Uuid;

/// WebAuthn credential stored for a principal.
#[derive(Debug, Clone)]
pub struct WebAuthnCredential {
    pub credential_id: Vec<u8>,
    pub public_key: Vec<u8>,
    pub sign_count: u32,
    pub authenticator_type: AuthenticatorType,
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
}

/// Type of authenticator used for WebAuthn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthenticatorType {
    /// Platform authenticator (fingerprint, face ID)
    Platform,
    /// Cross-platform authenticator (USB security key)
    CrossPlatform,
    /// Hybrid authenticator (Bluetooth, NFC)
    Hybrid,
}

impl AuthenticatorType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Platform => "platform",
            Self::CrossPlatform => "cross_platform",
            Self::Hybrid => "hybrid",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "platform" => Self::Platform,
            "cross_platform" => Self::CrossPlatform,
            "hybrid" => Self::Hybrid,
            _ => Self::CrossPlatform,
        }
    }
}

/// WebAuthn registration request (sent to client).
#[derive(Debug, Clone, Serialize)]
pub struct PublicKeyCredentialCreationOptions {
    pub challenge: Vec<u8>,
    pub rp: RelyingParty,
    pub user: PublicKeyCredentialUserEntity,
    pub pub_key_cred_params: Vec<PublicKeyCredentialParameters>,
    pub timeout_ms: u32,
    pub attestation: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RelyingParty {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PublicKeyCredentialUserEntity {
    pub id: Vec<u8>,
    pub name: String,
    pub display_name: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PublicKeyCredentialParameters {
    pub ty: String,
    pub alg: i32,
}

/// WebAuthn authentication request (sent to client).
#[derive(Debug, Clone, Serialize)]
pub struct PublicKeyCredentialRequestOptions {
    pub challenge: Vec<u8>,
    pub timeout_ms: u32,
    pub rp_id: String,
    pub allow_credentials: Vec<AllowCredential>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AllowCredential {
    pub id: Vec<u8>,
    pub ty: String,
    pub transports: Vec<String>,
}

/// WebAuthn registration response (received from client).
#[derive(Debug, Clone, Deserialize)]
pub struct PublicKeyCredential {
    pub id: String,
    pub raw_id: Vec<u8>,
    pub response: AuthenticatorResponse,
    pub authenticator_attachment: Option<String>,
    pub client_extension_results: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuthenticatorResponse {
    pub attestation_object: Option<String>,
    pub client_data_json: String,
    pub authenticator_data: Option<String>,
    pub signature: Option<String>,
    pub user_handle: Option<String>,
}

use serde::{Serialize, Deserialize};

/// Create a WebAuthn registration challenge for a principal.
pub fn create_registration_challenge(
    principal_id: Uuid,
    email: &str,
    rp_id: &str,
) -> PublicKeyCredentialCreationOptions {
    let challenge = generate_challenge();

    PublicKeyCredentialCreationOptions {
        challenge,
        rp: RelyingParty {
            id: rp_id.to_string(),
            name: "Payment Platform".to_string(),
        },
        user: PublicKeyCredentialUserEntity {
            id: principal_id.as_bytes().to_vec(),
            name: email.to_string(),
            display_name: email.to_string(),
        },
        pub_key_cred_params: vec![
            PublicKeyCredentialParameters { ty: "public-key".to_string(), alg: -7 }, // ES256
            PublicKeyCredentialParameters { ty: "public-key".to_string(), alg: -257 }, // RS256
        ],
        timeout_ms: 60000,
        attestation: "none".to_string(),
    }
}

/// Create a WebAuthn authentication challenge for a principal.
pub fn create_authentication_challenge(
    rp_id: &str,
    allow_credentials: Vec<Vec<u8>>,
) -> PublicKeyCredentialRequestOptions {
    let challenge = generate_challenge();

    PublicKeyCredentialRequestOptions {
        challenge,
        timeout_ms: 60000,
        rp_id: rp_id.to_string(),
        allow_credentials: allow_credentials.into_iter().map(|id| AllowCredential {
            id,
            ty: "public-key".to_string(),
            transports: vec!["usb".to_string(), "ble".to_string(), "nfc".to_string()],
        }).collect(),
    }
}

/// Generate a random challenge for WebAuthn.
fn generate_challenge() -> Vec<u8> {
    let mut challenge = vec![0u8; 32];
    use rand::Rng;
    rand::thread_rng().fill(&mut challenge[..]);
    challenge
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_authenticator_type_roundtrip() {
        for at in [AuthenticatorType::Platform, AuthenticatorType::CrossPlatform, AuthenticatorType::Hybrid] {
            let s = at.as_str();
            let parsed = AuthenticatorType::from_str(s);
            assert_eq!(at, parsed);
        }
    }

    #[test]
    fn test_create_registration_challenge() {
        let principal_id = Uuid::now_v7();
        let challenge = create_registration_challenge(principal_id, "user@example.com", "platform.example.com");
        assert_eq!(challenge.rp.id, "platform.example.com");
        assert_eq!(challenge.user.name, "user@example.com");
        assert_eq!(challenge.user.id, principal_id.as_bytes().to_vec());
        assert_eq!(challenge.pub_key_cred_params.len(), 2);
    }

    #[test]
    fn test_create_authentication_challenge() {
        let challenge = create_authentication_challenge("platform.example.com", vec![vec![1, 2, 3]]);
        assert_eq!(challenge.rp_id, "platform.example.com");
        assert_eq!(challenge.allow_credentials.len(), 1);
        assert_eq!(challenge.allow_credentials[0].id, vec![1, 2, 3]);
    }
}
