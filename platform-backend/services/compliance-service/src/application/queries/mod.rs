use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct GetKybCaseQuery {
    pub kyb_case_id: Uuid,
}

#[derive(Debug, Clone)]
pub struct ListKybCasesQuery {
    pub status: Option<String>,
    pub operator_id: Option<Uuid>,
    pub cursor: Option<String>,
    pub limit: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct GetKybDocumentsQuery {
    pub kyb_case_id: Uuid,
}
