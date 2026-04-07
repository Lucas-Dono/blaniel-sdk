use reqwest::{header, Client};
use serde_json::json;
use thiserror::Error;
use tracing::{debug, error};

use npc_types::{ChatContext, ChatResponse};

#[derive(Debug, Error)]
pub enum ClientError {
    #[error("HTTP request failed: {0}")]
    Request(#[from] reqwest::Error),

    #[error("Request failed with status {0}")]
    RequestFailed(reqwest::StatusCode),

    #[error("Invalid response: {0}")]
    InvalidResponse(String),
}

pub type ClientResult<T> = Result<T, ClientError>;

#[derive(Clone)]
pub struct NextJsClient {
    client: Client,
    base_url: String,
    auth_header: String,
}

impl NextJsClient {
    pub fn new(base_url: String, api_key: String) -> Result<Self, ClientError> {
        let auth_header = format!("Bearer {}", api_key);

        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .tcp_keepalive(Some(std::time::Duration::from_secs(60)))
            .pool_max_idle_per_host(10)
            .pool_idle_timeout(Some(std::time::Duration::from_secs(90)))
            .build()
            .map_err(|e| ClientError::Request(e))?;

        Ok(Self {
            client,
            base_url,
            auth_header,
        })
    }

    pub async fn process_complex_chat(
        &self,
        agent_id: &str,
        user_id: &str,
        message: &str,
        context: ChatContext,
        ai_context: Option<&str>,
    ) -> ClientResult<ChatResponse> {
        let url = format!("{}/api/v1/agents/{}/chat", self.base_url, agent_id);

        debug!("Calling Next.js AI backend: {}", url);

        let mut request_body = json!({
            "message": message,
            "context": context,
        });

        if let Some(ctx) = ai_context {
            request_body
                .as_object_mut()
                .unwrap()
                .insert("action_system_context".into(), json!(ctx));
        }

        let start = std::time::Instant::now();

        let response = self
            .client
            .post(&url)
            .header(header::AUTHORIZATION, &self.auth_header)
            .header("X-Service-Auth", "rust-npc-api")
            .header("X-User-Id", user_id)
            .json(&request_body)
            .send()
            .await?;

        if !response.status().is_success() {
            error!("Next.js API request failed with status: {}", response.status());
            return Err(ClientError::RequestFailed(response.status()));
        }

        let mut chat_response: ChatResponse = response.json().await?;

        chat_response.source = "nextjs_ai".to_string();
        chat_response.latency_ms = start.elapsed().as_millis() as u64;

        debug!(
            "Next.js AI response received in {}ms",
            chat_response.latency_ms
        );

        Ok(chat_response)
    }

    pub async fn update_npc_position(
        &self,
        agent_id: &str,
        position: &npc_types::Position3D,
    ) -> ClientResult<()> {
        let url = format!("{}/api/v1/npc/{}/position", self.base_url, agent_id);

        let request_body = json!({
            "position": position,
        });

        let response = self
            .client
            .put(&url)
            .header(header::AUTHORIZATION, &self.auth_header)
            .header("X-Service-Auth", "rust-npc-api")
            .json(&request_body)
            .send()
            .await?;

        if !response.status().is_success() {
            error!(
                "Failed to update NPC position, status: {}",
                response.status()
            );
            return Err(ClientError::RequestFailed(response.status()));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = NextJsClient::new(
            "http://localhost:3000".to_string(),
            "test-key".to_string(),
        ).expect("Failed to create test client");

        assert_eq!(client.base_url, "http://localhost:3000");
    }
}
