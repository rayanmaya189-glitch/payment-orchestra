#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocumentStatus { Uploaded, Verified, Rejected }
impl DocumentStatus { pub fn as_str(&self) -> &'static str { match self { Self::Uploaded => "uploaded", Self::Verified => "verified", Self::Rejected => "rejected" } } }

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerificationStatus { Pending, Verified, Rejected }
impl VerificationStatus { pub fn as_str(&self) -> &'static str { match self { Self::Pending => "pending", Self::Verified => "verified", Self::Rejected => "rejected" } } }
