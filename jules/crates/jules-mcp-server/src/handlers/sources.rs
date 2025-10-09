use crate::tools::Tool;
use async_trait::async_trait;
use jules_api::JulesClient;
use serde::Deserialize;
use serde_json::Value;

// --- ListSources Tool ---

#[derive(Debug, Deserialize)]
struct ListSourcesParams {
    page_size: Option<u32>,
    page_token: Option<String>,
}

pub struct ListSources;

#[async_trait]
impl Tool for ListSources {
    async fn execute(&self, client: &JulesClient, params: Value) -> Result<Value, (i32, String)> {
        let params: ListSourcesParams =
            serde_json::from_value(params).map_err(|e| (-32602, e.to_string()))?;

        let response = client
            .list_sources(params.page_size, params.page_token.as_deref())
            .await
            .map_err(|e| (-32000, e.to_string()))?;

        serde_json::to_value(response).map_err(|e| (-32603, e.to_string()))
    }
}

// --- GetSource Tool ---

#[derive(Debug, Deserialize)]
struct GetSourceParams {
    source_id: String,
}

pub struct GetSource;

#[async_trait]
impl Tool for GetSource {
    async fn execute(&self, client: &JulesClient, params: Value) -> Result<Value, (i32, String)> {
        let params: GetSourceParams =
            serde_json::from_value(params).map_err(|e| (-32602, e.to_string()))?;

        let source = client
            .get_source(&params.source_id)
            .await
            .map_err(|e| (-32000, e.to_string()))?;

        serde_json::to_value(source).map_err(|e| (-32603, e.to_string()))
    }
}