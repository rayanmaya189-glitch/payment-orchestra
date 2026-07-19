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
}
