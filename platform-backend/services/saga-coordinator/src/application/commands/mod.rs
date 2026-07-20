use uuid::Uuid;
#[derive(Debug, Clone)]
pub struct StartSagaCommand { pub saga_type: String, pub steps: Vec<SagaStepDef>, pub payload: serde_json::Value }
#[derive(Debug, Clone)]
pub struct SagaStepDef { pub name: String, pub service: String, pub action: String, pub compensation_action: Option<String> }
#[derive(Debug, Clone)]
pub struct AdvanceSagaCommand { pub saga_id: Uuid }
#[derive(Debug, Clone)]
pub struct FailSagaCommand { pub saga_id: Uuid, pub error: String }
