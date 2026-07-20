#![allow(clippy::should_implement_trait)]
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DocumentType {
    TradeLicense,
    CertificateOfIncorporation,
    BoardResolution,
    BankStatement,
    ProofOfAddress,
    UboDeclaration,
    Other,
}

impl DocumentType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::TradeLicense => "trade_license",
            Self::CertificateOfIncorporation => "certificate_of_incorporation",
            Self::BoardResolution => "board_resolution",
            Self::BankStatement => "bank_statement",
            Self::ProofOfAddress => "proof_of_address",
            Self::UboDeclaration => "ubo_declaration",
            Self::Other => "other",
        }
    }

    pub fn from_str(s: &str) -> Result<Self, &'static str> {
        match s {
            "trade_license" => Ok(Self::TradeLicense),
            "certificate_of_incorporation" => Ok(Self::CertificateOfIncorporation),
            "board_resolution" => Ok(Self::BoardResolution),
            "bank_statement" => Ok(Self::BankStatement),
            "proof_of_address" => Ok(Self::ProofOfAddress),
            "ubo_declaration" => Ok(Self::UboDeclaration),
            "other" => Ok(Self::Other),
            _ => Err("unknown document type"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DocumentStatus {
    Uploaded,
    Processing,
    Completed,
    Failed,
    Deleted,
}

impl DocumentStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Uploaded => "uploaded",
            Self::Processing => "processing",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Deleted => "deleted",
        }
    }

    pub fn from_str(s: &str) -> Result<Self, &'static str> {
        match s {
            "uploaded" => Ok(Self::Uploaded),
            "processing" => Ok(Self::Processing),
            "completed" => Ok(Self::Completed),
            "failed" => Ok(Self::Failed),
            "deleted" => Ok(Self::Deleted),
            _ => Err("unknown document status"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_document_type_values() {
        assert_eq!(DocumentType::TradeLicense.as_str(), "trade_license");
        assert_eq!(DocumentType::BankStatement.as_str(), "bank_statement");
    }

    #[test]
    fn test_document_type_from_str() {
        assert_eq!(DocumentType::from_str("trade_license").unwrap(), DocumentType::TradeLicense);
        assert!(DocumentType::from_str("unknown").is_err());
    }

    #[test]
    fn test_document_status_values() {
        assert_eq!(DocumentStatus::Uploaded.as_str(), "uploaded");
        assert_eq!(DocumentStatus::Completed.as_str(), "completed");
    }

    #[test]
    fn test_document_status_from_str() {
        assert_eq!(DocumentStatus::from_str("uploaded").unwrap(), DocumentStatus::Uploaded);
        assert!(DocumentStatus::from_str("unknown").is_err());
    }
}
