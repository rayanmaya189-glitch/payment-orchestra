use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct SubmitKybCommand {
    pub operator_id: Uuid,
    pub principal_id: Uuid,
    pub role: String,
}

#[derive(Debug, Clone)]
pub struct AssignOfficerCommand {
    pub case_id: Uuid,
    pub officer_id: Uuid,
    pub principal_id: Uuid,
    pub role: String,
}

#[derive(Debug, Clone)]
pub struct DecideKybCommand {
    pub case_id: Uuid,
    pub decision: String,
    pub reason: String,
    pub principal_id: Uuid,
    pub role: String,
}

#[derive(Debug, Clone)]
pub struct GetKybCaseCommand {
    pub case_id: Uuid,
    pub principal_id: Uuid,
    pub role: String,
}
