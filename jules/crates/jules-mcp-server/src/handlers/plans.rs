use crate::tools::Tool;
use async_trait::async_trait;
use jules_api::JulesClient;
use serde::Deserialize;
use serde_json::Value;

// --- ApprovePlan Tool ---

#[derive(Debug, Deserialize)]
struct ApprovePlanParams {
    session_id: String,
    plan_id: String,
}

pub struct ApprovePlan;

#[async_trait]
impl Tool for ApprovePlan {
    async fn execute(&self, client: &JulesClient, params: Value) -> Result<Value, (i32, String)> {
        let params: ApprovePlanParams =
            serde_json::from_value(params).map_err(|e| (-32602, e.to_string()))?;

        client
            .approve_plan(&params.session_id, &params.plan_id)
            .await
            .map_err(|e| (-32000, e.to_string()))?;

        Ok(Value::Null)
    }
}