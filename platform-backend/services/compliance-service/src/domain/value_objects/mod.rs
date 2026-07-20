#![allow(clippy::should_implement_trait)]
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum KybCaseStatus {
    Submitted,
    UnderReview,
    DocumentsRequested,
    DocumentsVerified,
    Approved,
    Rejected,
    Suspended,
}

impl KybCaseStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Submitted => "submitted",
            Self::UnderReview => "under_review",
            Self::DocumentsRequested => "documents_requested",
            Self::DocumentsVerified => "documents_verified",
            Self::Approved => "approved",
            Self::Rejected => "rejected",
            Self::Suspended => "suspended",
        }
    }

    pub fn from_str(s: &str) -> Result<Self, &'static str> {
        match s {
            "submitted" => Ok(Self::Submitted),
            "under_review" => Ok(Self::UnderReview),
            "documents_requested" => Ok(Self::DocumentsRequested),
            "documents_verified" => Ok(Self::DocumentsVerified),
            "approved" => Ok(Self::Approved),
            "rejected" => Ok(Self::Rejected),
            "suspended" => Ok(Self::Suspended),
            _ => Err("unknown KYB case status"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum KybDocumentType {
    TradeLicense,
    CertificateOfIncorporation,
    MemorandumOfAssociation,
    BoardResolution,
    ProofOfAddress,
    BankStatement,
    UboDeclaration,
}

impl KybDocumentType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::TradeLicense => "trade_license",
            Self::CertificateOfIncorporation => "certificate_of_incorporation",
            Self::MemorandumOfAssociation => "memorandum_of_association",
            Self::BoardResolution => "board_resolution",
            Self::ProofOfAddress => "proof_of_address",
            Self::BankStatement => "bank_statement",
            Self::UboDeclaration => "ubo_declaration",
        }
    }

    pub fn from_str(s: &str) -> Result<Self, &'static str> {
        match s {
            "trade_license" => Ok(Self::TradeLicense),
            "certificate_of_incorporation" => Ok(Self::CertificateOfIncorporation),
            "memorandum_of_association" => Ok(Self::MemorandumOfAssociation),
            "board_resolution" => Ok(Self::BoardResolution),
            "proof_of_address" => Ok(Self::ProofOfAddress),
            "bank_statement" => Ok(Self::BankStatement),
            "ubo_declaration" => Ok(Self::UboDeclaration),
            _ => Err("unknown KYB document type"),
        }
    }
}
