use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DocumentStatus {
    Uploaded,
    Verified,
    Rejected,
}

impl DocumentStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Uploaded => "uploaded",
            Self::Verified => "verified",
            Self::Rejected => "rejected",
        }
    }

    pub fn can_transition_to(&self, target: &DocumentStatus) -> bool {
        matches!(
            (self, target),
            (Self::Uploaded, Self::Verified) | (Self::Uploaded, Self::Rejected)
        )
    }
}

impl fmt::Display for DocumentStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for DocumentStatus {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "uploaded" => Ok(Self::Uploaded),
            "verified" => Ok(Self::Verified),
            "rejected" => Ok(Self::Rejected),
            other => Err(format!("unknown document status: {other}")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum VerificationStatus {
    Pending,
    Verified,
    Rejected,
}

impl VerificationStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Verified => "verified",
            Self::Rejected => "rejected",
        }
    }
}

impl fmt::Display for VerificationStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for VerificationStatus {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pending" => Ok(Self::Pending),
            "verified" => Ok(Self::Verified),
            "rejected" => Ok(Self::Rejected),
            other => Err(format!("unknown verification status: {other}")),
        }
    }
}

/// Allowed document types for KYB compliance.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DocumentType {
    TradeLicense,
    CertificateOfIncorporation,
    MemorandumOfAssociation,
    BankStatement,
    ProofOfAddress,
    Passport,
    NationalId,
    Other(String),
}

impl DocumentType {
    pub fn as_str(&self) -> &str {
        match self {
            Self::TradeLicense => "trade_license",
            Self::CertificateOfIncorporation => "certificate_of_incorporation",
            Self::MemorandumOfAssociation => "memorandum_of_association",
            Self::BankStatement => "bank_statement",
            Self::ProofOfAddress => "proof_of_address",
            Self::Passport => "passport",
            Self::NationalId => "national_id",
            Self::Other(s) => s,
        }
    }

    /// Maximum allowed file size in bytes for this document type.
    pub fn max_file_size(&self) -> i64 {
        match self {
            Self::BankStatement => 10 * 1024 * 1024, // 10MB
            _ => 5 * 1024 * 1024,                    // 5MB
        }
    }

    /// Allowed MIME types for this document type.
    pub fn allowed_content_types(&self) -> &[&str] {
        &[
            "application/pdf",
            "image/jpeg",
            "image/png",
        ]
    }
}

impl fmt::Display for DocumentType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for DocumentType {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "trade_license" => Ok(Self::TradeLicense),
            "certificate_of_incorporation" => Ok(Self::CertificateOfIncorporation),
            "memorandum_of_association" => Ok(Self::MemorandumOfAssociation),
            "bank_statement" => Ok(Self::BankStatement),
            "proof_of_address" => Ok(Self::ProofOfAddress),
            "passport" => Ok(Self::Passport),
            "national_id" => Ok(Self::NationalId),
            other => Ok(Self::Other(other.to_string())),
        }
    }
}

/// Errors that can occur during document state transitions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocumentError {
    InvalidStateTransition { from: String, to: String },
    InvalidFileType { content_type: String, document_type: String },
    FileTooLarge { size: i64, max_size: i64 },
    InvalidFileHash,
    MissingRequiredField(String),
}

impl fmt::Display for DocumentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidStateTransition { from, to } => {
                write!(f, "Cannot transition from {from} to {to}")
            }
            Self::InvalidFileType { content_type, document_type } => {
                write!(f, "Content type {content_type} not allowed for {document_type}")
            }
            Self::FileTooLarge { size, max_size } => {
                write!(f, "File size {size} exceeds maximum {max_size}")
            }
            Self::InvalidFileHash => write!(f, "File hash verification failed"),
            Self::MissingRequiredField(field) => write!(f, "Missing required field: {field}"),
        }
    }
}

impl std::error::Error for DocumentError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_document_status_roundtrip() {
        for status in [
            DocumentStatus::Uploaded,
            DocumentStatus::Verified,
            DocumentStatus::Rejected,
        ] {
            let s = status.as_str();
            let parsed: DocumentStatus = s.parse().unwrap();
            assert_eq!(status, parsed);
        }
    }

    #[test]
    fn test_document_status_transitions() {
        assert!(DocumentStatus::Uploaded.can_transition_to(&DocumentStatus::Verified));
        assert!(DocumentStatus::Uploaded.can_transition_to(&DocumentStatus::Rejected));
        assert!(!DocumentStatus::Verified.can_transition_to(&DocumentStatus::Uploaded));
        assert!(!DocumentStatus::Rejected.can_transition_to(&DocumentStatus::Verified));
    }

    #[test]
    fn test_verification_status_roundtrip() {
        for status in [
            VerificationStatus::Pending,
            VerificationStatus::Verified,
            VerificationStatus::Rejected,
        ] {
            let s = status.as_str();
            let parsed: VerificationStatus = s.parse().unwrap();
            assert_eq!(status, parsed);
        }
    }

    #[test]
    fn test_document_type_display() {
        assert_eq!(DocumentType::TradeLicense.to_string(), "trade_license");
        assert_eq!(DocumentType::BankStatement.to_string(), "bank_statement");
    }

    #[test]
    fn test_document_type_from_str() {
        let dt: DocumentType = "trade_license".parse().unwrap();
        assert_eq!(dt, DocumentType::TradeLicense);

        let dt: DocumentType = "custom_type".parse().unwrap();
        assert_eq!(dt, DocumentType::Other("custom_type".into()));
    }

    #[test]
    fn test_document_type_max_size() {
        assert_eq!(DocumentType::BankStatement.max_file_size(), 10 * 1024 * 1024);
        assert_eq!(DocumentType::TradeLicense.max_file_size(), 5 * 1024 * 1024);
    }

    #[test]
    fn test_document_type_allowed_content_types() {
        let types = DocumentType::TradeLicense.allowed_content_types();
        assert!(types.contains(&"application/pdf"));
        assert!(types.contains(&"image/jpeg"));
        assert!(types.contains(&"image/png"));
    }

    #[test]
    fn test_document_error_display() {
        let e = DocumentError::InvalidStateTransition {
            from: "verified".into(),
            to: "uploaded".into(),
        };
        assert_eq!(e.to_string(), "Cannot transition from verified to uploaded");

        let e = DocumentError::FileTooLarge {
            size: 20_000_000,
            max_size: 5_000_000,
        };
        assert!(e.to_string().contains("20000000"));
    }
}
