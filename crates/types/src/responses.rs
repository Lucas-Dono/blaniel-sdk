use crate::{NpcState, Position3D};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub timestamp: i64,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub response: String,
    pub emotion: String,
    pub animation: String,
    pub source: String, // "rust_cache", "nextjs_ai", "template"
    pub latency_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cached: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NearbyNpcsResponse {
    pub npcs: Vec<NpcState>,
    pub total: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchStateResponse {
    pub states: Vec<Result<NpcState, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathResult {
    pub path: Vec<Position3D>,
    pub cost: f64,
    pub computed_in_ms: u128,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cached: Option<bool>,
}

impl PathResult {
    pub fn is_empty(&self) -> bool {
        self.path.is_empty()
    }

    pub fn length(&self) -> usize {
        self.path.len()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmbientDialogue {
    pub speaker_id: String,
    pub message: String,
    pub emotion: String,
    pub animation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmbientDialoguesResponse {
    pub dialogues: Vec<AmbientDialogue>,
    pub cached: bool,
    pub group_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    pub error: String,
    pub code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

impl ApiError {
    pub fn new(error: impl Into<String>, code: impl Into<String>) -> Self {
        Self {
            error: error.into(),
            code: code.into(),
            details: None,
        }
    }

    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChunk {
    pub delta: String,
    pub done: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emotion: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_result_empty() {
        let empty_path = PathResult {
            path: vec![],
            cost: 0.0,
            computed_in_ms: 0,
            cached: None,
        };

        assert!(empty_path.is_empty());
        assert_eq!(empty_path.length(), 0);

        let non_empty = PathResult {
            path: vec![
                Position3D::new(0.0, 0.0, 0.0, "overworld".to_string()),
                Position3D::new(1.0, 0.0, 0.0, "overworld".to_string()),
            ],
            cost: 1.0,
            computed_in_ms: 10,
            cached: None,
        };

        assert!(!non_empty.is_empty());
        assert_eq!(non_empty.length(), 2);
    }

    #[test]
    fn test_api_error_creation() {
        let error = ApiError::new("Not found", "NOT_FOUND");
        assert_eq!(error.error, "Not found");
        assert_eq!(error.code, "NOT_FOUND");
        assert!(error.details.is_none());

        let error_with_details = error.with_details(serde_json::json!({
            "agent_id": "test123"
        }));
        assert!(error_with_details.details.is_some());
    }
}
