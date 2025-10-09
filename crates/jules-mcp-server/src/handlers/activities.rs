use crate::tools::Tool;
use async_trait::async_trait;
use jules_api::JulesClient;
use serde::Deserialize;
use serde_json::Value;

// --- ListActivities Tool ---

#[derive(Debug, Deserialize)]
struct ListActivitiesParams {
    session_id: String,
    page_size: Option<u32>,
    page_token: Option<String>,
}

pub struct ListActivities;

#[async_trait]
impl Tool for ListActivities {
    async fn execute(&self, client: &JulesClient, params: Value) -> Result<Value, (i32, String)> {
        let params: ListActivitiesParams =
            serde_json::from_value(params).map_err(|e| (-32602, e.to_string()))?;

        let response = client
            .list_activities(
                &params.session_id,
                params.page_size,
                params.page_token.as_deref(),
            )
            .await
            .map_err(|e| (-32000, e.to_string()))?;

        serde_json::to_value(response).map_err(|e| (-32603, e.to_string()))
    }
}

// --- SendMessage Tool ---

#[derive(Debug, Deserialize)]
struct SendMessageParams {
    session_id: String,
    content: String,
}

pub struct SendMessage;

#[async_trait]
impl Tool for SendMessage {
    async fn execute(&self, client: &JulesClient, params: Value) -> Result<Value, (i32, String)> {
        let params: SendMessageParams =
            serde_json::from_value(params).map_err(|e| (-32602, e.to_string()))?;

        let activity = client
            .send_message(&params.session_id, &params.content)
            .await
            .map_err(|e| (-32000, e.to_string()))?;

        serde_json::to_value(activity).map_err(|e| (-32603, e.to_string()))
    }
}