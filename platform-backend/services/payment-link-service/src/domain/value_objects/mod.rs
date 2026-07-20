#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaymentLinkStatus { Active, Expired, UsedUp }
impl PaymentLinkStatus {
    pub fn as_str(&self) -> &'static str { match self { Self::Active => "active", Self::Expired => "expired", Self::UsedUp => "used_up" } }
    pub fn from_str(s: &str) -> Self { match s { "expired" => Self::Expired, "used_up" => Self::UsedUp, _ => Self::Active } }
}
