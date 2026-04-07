use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::ActionType;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionExecution {
    pub action_id: String,
    pub action_name: String,
    pub action_type: ActionType,
    pub agent_id: String,
    pub timestamp: i64,
    pub duration_ms: u64,
    pub success: bool,
    pub context: HashMap<String, String>,
    #[serde(default)]
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionAnalytics {
    pub agent_id: String,
    pub total_actions: u64,
    pub most_used_action: Option<String>,
    pub average_duration_ms: u64,
    pub success_rate: f64,
    pub action_distribution: HashMap<String, u64>,
    pub action_success_counts: HashMap<String, u64>,
    pub action_failure_counts: HashMap<String, u64>,
}

impl ActionAnalytics {
    pub fn new(agent_id: String) -> Self {
        Self {
            agent_id,
            total_actions: 0,
            most_used_action: None,
            average_duration_ms: 0,
            success_rate: 0.0,
            action_distribution: HashMap::new(),
            action_success_counts: HashMap::new(),
            action_failure_counts: HashMap::new(),
        }
    }

    pub fn add_execution(&mut self, execution: &ActionExecution) {
        self.total_actions += 1;

        *self
            .action_distribution
            .entry(execution.action_name.clone())
            .or_insert(0) += 1;

        if execution.success {
            *self
                .action_success_counts
                .entry(execution.action_name.clone())
                .or_insert(0) += 1;
        } else {
            *self
                .action_failure_counts
                .entry(execution.action_name.clone())
                .or_insert(0) += 1;
        }

        self.recalculate_stats();
    }

    fn recalculate_stats(&mut self) {
        // Find most used action
        self.most_used_action = self
            .action_distribution
            .iter()
            .max_by_key(|(_, count)| *count)
            .map(|(name, _)| name.clone());

        // Calculate success rate
        let total_successes: u64 = self.action_success_counts.values().sum();
        self.success_rate = if self.total_actions > 0 {
            (total_successes as f64 / self.total_actions as f64) * 100.0
        } else {
            0.0
        };
    }

    pub fn get_action_success_rate(&self, action_name: &str) -> f64 {
        let successes = self
            .action_success_counts
            .get(action_name)
            .copied()
            .unwrap_or(0);
        let failures = self
            .action_failure_counts
            .get(action_name)
            .copied()
            .unwrap_or(0);
        let total = successes + failures;

        if total > 0 {
            (successes as f64 / total as f64) * 100.0
        } else {
            0.0
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordActionExecutionRequest {
    pub execution: ActionExecution,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordActionExecutionResponse {
    pub status: String,
    pub execution_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetActionAnalyticsRequest {
    #[serde(default)]
    pub time_range_hours: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetActionAnalyticsResponse {
    pub analytics: ActionAnalytics,
    pub time_range_hours: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analytics_add_execution() {
        let mut analytics = ActionAnalytics::new("test-agent".into());

        let execution = ActionExecution {
            action_id: "exec-1".into(),
            action_name: "combat".into(),
            action_type: ActionType::Navigate,
            agent_id: "test-agent".into(),
            timestamp: 1234567890,
            duration_ms: 1000,
            success: true,
            context: HashMap::new(),
            error_message: None,
        };

        analytics.add_execution(&execution);

        assert_eq!(analytics.total_actions, 1);
        assert_eq!(analytics.most_used_action, Some("combat".into()));
        assert_eq!(analytics.success_rate, 100.0);
    }

    #[test]
    fn test_analytics_success_rate() {
        let mut analytics = ActionAnalytics::new("test-agent".into());

        for i in 0..10 {
            let execution = ActionExecution {
                action_id: format!("exec-{}", i),
                action_name: "test_action".into(),
                action_type: ActionType::Wait,
                agent_id: "test-agent".into(),
                timestamp: 1234567890 + i,
                duration_ms: 1000,
                success: i < 7, // 7 successes, 3 failures
                context: HashMap::new(),
                error_message: if i >= 7 {
                    Some("failed".into())
                } else {
                    None
                },
            };
            analytics.add_execution(&execution);
        }

        assert_eq!(analytics.total_actions, 10);
        assert_eq!(analytics.success_rate, 70.0);
        assert_eq!(analytics.get_action_success_rate("test_action"), 70.0);
    }
}
