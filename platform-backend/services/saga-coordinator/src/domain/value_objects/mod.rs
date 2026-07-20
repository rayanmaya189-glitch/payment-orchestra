#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SagaStatus { Running, Completed, Failed, Compensating, Compensated }
impl SagaStatus { pub fn as_str(&self) -> &'static str { match self { Self::Running => "running", Self::Completed => "completed", Self::Failed => "failed", Self::Compensating => "compensating", Self::Compensated => "compensated" } } }

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SagaStepStatus { Pending, Running, Completed, Failed, Compensated }
impl SagaStepStatus { pub fn as_str(&self) -> &'static str { match self { Self::Pending => "pending", Self::Running => "running", Self::Completed => "completed", Self::Failed => "failed", Self::Compensated => "compensated" } } }
