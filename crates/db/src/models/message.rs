use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct MessageRecord {
    pub id: String,
    #[sqlx(rename = "agentId")]
    pub agent_id: Option<String>,
    #[sqlx(rename = "userId")]
    pub user_id: Option<String>,
    pub role: String,
    pub content: String,
    pub metadata: Option<serde_json::Value>,
    #[sqlx(rename = "createdAt")]
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl MessageRecord {
    pub fn is_from_user(&self) -> bool {
        self.role == "user"
    }

    pub fn is_from_agent(&self) -> bool {
        self.role == "assistant" || self.role == "agent"
    }
}
