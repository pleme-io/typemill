use crate::{config::Config, Result};
use reqwest::Client;
use std::time::Duration;

/// The main client for interacting with the Jules API.
#[derive(Debug, Clone)]
pub struct JulesClient {
    http_client: Client,
    base_url: String,
    api_key: String,
}

impl JulesClient {
    /// Creates a new `JulesClient` from a given configuration.
    pub fn new(config: Config) -> Self {
        let http_client = Client::builder()
            .timeout(Duration::from_secs(config.request_timeout))
            .build()
            .expect("Failed to build HTTP client");

        Self {
            http_client,
            base_url: config.api_base_url,
            api_key: config.api_key,
        }
    }

    /// Creates a new `JulesClient` with a custom `reqwest::Client`.
    pub fn with_client(client: Client, base_url: &str, api_key: &str) -> Self {
        Self {
            http_client: client,
            base_url: base_url.to_string(),
            api_key: api_key.to_string(),
        }
    }

    fn add_auth(&self, builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        builder.bearer_auth(&self.api_key)
    }

    pub async fn list_sources(
        &self,
        page_size: Option<u32>,
        page_token: Option<&str>,
    ) -> Result<crate::types::SourcesResponse> {
        let mut url = format!("{}/sources", self.base_url);
        let mut query_params = Vec::new();

        if let Some(size) = page_size {
            query_params.push(("pageSize", size.to_string()));
        }
        if let Some(token) = page_token {
            query_params.push(("pageToken", token.to_string()));
        }

        if !query_params.is_empty() {
            let query_string = query_params
                .into_iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect::<Vec<_>>()
                .join("&");
            url.push('?');
            url.push_str(&query_string);
        }

        let builder = self.http_client.get(&url);
        let response = self.add_auth(builder).send().await?;
        let sources = response.json().await?;
        Ok(sources)
    }

    pub async fn get_source(&self, source_id: &str) -> Result<crate::types::Source> {
        let url = format!("{}/sources/{}", self.base_url, source_id);
        let builder = self.http_client.get(&url);
        let response = self.add_auth(builder).send().await?;
        let source = response.json().await?;
        Ok(source)
    }

    pub async fn create_session(
        &self,
        req: crate::types::CreateSessionRequest,
    ) -> Result<crate::types::Session> {
        let url = format!("{}/sessions", self.base_url);
        let builder = self.http_client.post(&url).json(&req);
        let response = self.add_auth(builder).send().await?;
        let session = response.json().await?;
        Ok(session)
    }

    pub async fn list_sessions(
        &self,
        page_size: Option<u32>,
        page_token: Option<&str>,
    ) -> Result<crate::types::SessionsResponse> {
        let mut url = format!("{}/sessions", self.base_url);
        let mut query_params = Vec::new();

        if let Some(size) = page_size {
            query_params.push(("pageSize", size.to_string()));
        }
        if let Some(token) = page_token {
            query_params.push(("pageToken", token.to_string()));
        }

        if !query_params.is_empty() {
            let query_string = query_params
                .into_iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect::<Vec<_>>()
                .join("&");
            url.push('?');
            url.push_str(&query_string);
        }

        let builder = self.http_client.get(&url);
        let response = self.add_auth(builder).send().await?;
        let sessions = response.json().await?;
        Ok(sessions)
    }

    pub async fn get_session(&self, session_id: &str) -> Result<crate::types::Session> {
        let url = format!("{}/sessions/{}", self.base_url, session_id);
        let builder = self.http_client.get(&url);
        let response = self.add_auth(builder).send().await?;
        let session = response.json().await?;
        Ok(session)
    }

    pub async fn delete_session(&self, session_id: &str) -> Result<()> {
        let url = format!("{}/sessions/{}", self.base_url, session_id);
        let builder = self.http_client.delete(&url);
        self.add_auth(builder).send().await?;
        Ok(())
    }

    pub async fn list_activities(
        &self,
        session_id: &str,
        page_size: Option<u32>,
        page_token: Option<&str>,
    ) -> Result<crate::types::ActivitiesResponse> {
        let mut url = format!("{}/sessions/{}/activities", self.base_url, session_id);
        let mut query_params = Vec::new();

        if let Some(size) = page_size {
            query_params.push(("pageSize", size.to_string()));
        }
        if let Some(token) = page_token {
            query_params.push(("pageToken", token.to_string()));
        }

        if !query_params.is_empty() {
            let query_string = query_params
                .into_iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect::<Vec<_>>()
                .join("&");
            url.push('?');
            url.push_str(&query_string);
        }

        let builder = self.http_client.get(&url);
        let response = self.add_auth(builder).send().await?;
        let activities = response.json().await?;
        Ok(activities)
    }

    pub async fn send_message(
        &self,
        session_id: &str,
        content: &str,
    ) -> Result<crate::types::Activity> {
        let url = format!("{}/sessions/{}/activities", self.base_url, session_id);
        let request_body = serde_json::json!({ "content": content });
        let builder = self.http_client.post(&url).json(&request_body);
        let response = self.add_auth(builder).send().await?;
        let activity = response.json().await?;
        Ok(activity)
    }

    pub async fn approve_plan(&self, session_id: &str, plan_id: &str) -> Result<()> {
        let url = format!(
            "{}/sessions/{}/plans/{}:approve",
            self.base_url, session_id, plan_id
        );
        let builder = self.http_client.post(&url);
        self.add_auth(builder).send().await?;
        Ok(())
    }
}