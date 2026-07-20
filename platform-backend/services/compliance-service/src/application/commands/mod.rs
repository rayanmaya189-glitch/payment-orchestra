use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct SubmitKybCommand { pub operator_id: Uuid }

#[derive(Debug, Clone)]
pub struct AssignOfficerCommand { pub case_id: Uuid, pub officer_id: Uuid }

#[derive(Debug, Clone)]
pub struct DecideKybCommand { pub case_id: Uuid, pub decision: String, pub reason: String }

#[derive(Debug, Clone)]
pub struct GetKybCaseCommand { pub case_id: Uuid }
