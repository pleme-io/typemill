use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Activity {
    pub id: String,
    pub session_id: String,
    pub r#type: String,
    pub content: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivitiesResponse {
    pub activities: Vec<Activity>,
    pub next_page_token: Option<String>,
}