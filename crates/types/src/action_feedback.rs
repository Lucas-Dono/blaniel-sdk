use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionFeedback {
    pub action_id: String,
    pub action_name: String,
    pub agent_id: String,
    pub success: bool,
    pub execution_time_ms: u64,
    pub result_data: serde_json::Value,
    #[serde(default)]
    pub error_message: Option<String>,
    pub context_snapshot: HashMap<String, String>,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitActionFeedbackRequest {
    pub feedback: ActionFeedback,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitActionFeedbackResponse {
    pub status: String,
    pub feedback_id: String,
    pub will_adjust_model: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionFeedbackStats {
    pub agent_id: String,
    pub total_feedback: u64,
    pub success_rate: f64,
    pub average_execution_time_ms: u64,
    pub most_successful_action: Option<String>,
    pub most_failed_action: Option<String>,
}

impl ActionFeedback {
    pub fn new(
        action_id: String,
        action_name: String,
        agent_id: String,
        success: bool,
        execution_time_ms: u64,
    ) -> Self {
        Self {
            action_id,
            action_name,
            agent_id,
            success,
            execution_time_ms,
            result_data: serde_json::Value::Null,
            error_message: None,
            context_snapshot: HashMap::new(),
            timestamp: chrono::Utc::now().timestamp_millis(),
        }
    }

    pub fn with_error(mut self, error: String) -> Self {
        self.success = false;
        self.error_message = Some(error);
        self
    }

    pub fn with_result_data(mut self, data: serde_json::Value) -> Self {
        self.result_data = data;
        self
    }

    pub fn with_context(mut self, context: HashMap<String, String>) -> Self {
        self.context_snapshot = context;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feedback_creation() {
        let feedback = ActionFeedback::new(
            "action-1".into(),
            "test_action".into(),
            "agent-1".into(),
            true,
            1500,
        );

        assert_eq!(feedback.action_id, "action-1");
        assert!(feedback.success);
        assert_eq!(feedback.execution_time_ms, 1500);
        assert!(feedback.error_message.is_none());
    }

    #[test]
    fn test_feedback_with_error() {
        let feedback = ActionFeedback::new(
            "action-1".into(),
            "test_action".into(),
            "agent-1".into(),
            true,
            1500,
        )
        .with_error("Test error".into());

        assert!(!feedback.success);
        assert_eq!(feedback.error_message, Some("Test error".into()));
    }

    #[test]
    fn test_feedback_with_context() {
        let mut context = HashMap::new();
        context.insert("player".into(), "John".into());

        let feedback = ActionFeedback::new(
            "action-1".into(),
            "test_action".into(),
            "agent-1".into(),
            true,
            1500,
        )
        .with_context(context);

        assert_eq!(feedback.context_snapshot.get("player"), Some(&"John".to_string()));
    }
}
