use super::*;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct VariableListParams {
    #[schemars(description = "Maximum number of variables to return, between 1 and 100")]
    pub limit: Option<u64>,
    #[schemars(description = "Optional exact scope: Global or Script")]
    pub scope: Option<String>,
    #[schemars(description = "Optional task id for script-scoped variables")]
    pub task_id: Option<i32>,
    #[schemars(description = "Optional key search text")]
    pub query: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct VariableIdParams {
    pub variable_id: i32,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct VariableCreateParams {
    pub key: String,
    pub value: String,
    #[schemars(description = "Variable scope: Global or Script")]
    pub scope: String,
    pub task_ids: Option<Vec<i32>>,
    pub remarks: Option<String>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct VariableUpdateParams {
    pub variable_id: i32,
    pub key: String,
    pub value: String,
    #[schemars(description = "Variable scope: Global or Script")]
    pub scope: String,
    pub task_ids: Option<Vec<i32>>,
    pub remarks: Option<String>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct VariableOutput {
    pub id: i32,
    pub key: String,
    pub value: Option<String>,
    pub remarks: Option<String>,
    pub enabled: bool,
    pub scope: String,
    pub task_ids: Vec<i32>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct VariableListOutput {
    pub total: u64,
    pub items: Vec<VariableOutput>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct VariableActionOutput {
    pub variable_id: i32,
    pub accepted: bool,
    pub message: String,
}
