use crate::tools::Tool;
use async_trait::async_trait;
use jules_api::{types::CreateSessionRequest, JulesClient};
use serde::Deserialize;
use serde_json::Value;

// --- CreateSession Tool ---

#[derive(Debug, Deserialize)]
struct CreateSessionParams {
    source_id: String,
}

pub struct CreateSession;

#[async_trait]
impl Tool for CreateSession {
    async fn execute(&self, client: &JulesClient, params: Value) -> Result<Value, (i32, String)> {
        let params: CreateSessionParams =
            serde_json::from_value(params).map_err(|e| (-32602, e.to_string()))?;

        let request = CreateSessionRequest {
            source_id: params.source_id,
        };

        let session = client
            .create_session(request)
            .await
            .map_err(|e| (-32000, e.to_string()))?;

        serde_json::to_value(session).map_err(|e| (-32603, e.to_string()))
    }
}

// --- ListSessions Tool ---

#[derive(Debug, Deserialize)]
struct ListSessionsParams {
    page_size: Option<u32>,
    page_token: Option<String>,
}

pub struct ListSessions;

#[async_trait]
impl Tool for ListSessions {
    async fn execute(&self, client: &JulesClient, params: Value) -> Result<Value, (i32, String)> {
        let params: ListSessionsParams =
            serde_json::from_value(params).map_err(|e| (-32602, e.to_string()))?;

        let response = client
            .list_sessions(params.page_size, params.page_token.as_deref())
            .await
            .map_err(|e| (-32000, e.to_string()))?;

        serde_json::to_value(response).map_err(|e| (-32603, e.to_string()))
    }
}

// --- GetSession Tool ---

#[derive(Debug, Deserialize)]
struct GetSessionParams {
    session_id: String,
}

pub struct GetSession;

#[async_trait]
impl Tool for GetSession {
    async fn execute(&self, client: &JulesClient, params: Value) -> Result<Value, (i32, String)> {
        let params: GetSessionParams =
            serde_json::from_value(params).map_err(|e| (-32602, e.to_string()))?;

        let session = client
            .get_session(&params.session_id)
            .await
            .map_err(|e| (-32000, e.to_string()))?;

        serde_json::to_value(session).map_err(|e| (-32603, e.to_string()))
    }
}

// --- DeleteSession Tool ---

#[derive(Debug, Deserialize)]
struct DeleteSessionParams {
    session_id: String,
}

pub struct DeleteSession;

#[async_trait]
impl Tool for DeleteSession {
    async fn execute(&self, client: &JulesClient, params: Value) -> Result<Value, (i32, String)> {
        let params: DeleteSessionParams =
            serde_json::from_value(params).map_err(|e| (-32602, e.to_string()))?;

        client
            .delete_session(&params.session_id)
            .await
            .map_err(|e| (-32000, e.to_string()))?;

        Ok(Value::Null)
    }
}