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

    pub fn from_str(s: &str) -> Self {
        match s {
            "submitted" => Self::Submitted,
            "under_review" => Self::UnderReview,
            "documents_requested" => Self::DocumentsRequested,
            "documents_verified" => Self::DocumentsVerified,
            "approved" => Self::Approved,
            "rejected" => Self::Rejected,
            "suspended" => Self::Suspended,
            _ => Self::Submitted,
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

    pub fn from_str(s: &str) -> Self {
        match s {
            "trade_license" => Self::TradeLicense,
            "certificate_of_incorporation" => Self::CertificateOfIncorporation,
            "memorandum_of_association" => Self::MemorandumOfAssociation,
            "board_resolution" => Self::BoardResolution,
            "proof_of_address" => Self::ProofOfAddress,
            "bank_statement" => Self::BankStatement,
            "ubo_declaration" => Self::UboDeclaration,
            _ => Self::TradeLicense,
        }
    }
}
