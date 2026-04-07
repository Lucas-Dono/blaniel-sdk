use serde::{Deserialize, Serialize};

use crate::{ActionContext, AutonomousAction};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositeAction {
    pub id: String,
    pub name: String,
    pub description: String,
    pub steps: Vec<ActionStep>,
    #[serde(default)]
    pub rollback_on_failure: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionStep {
    pub step_id: String,
    pub action: AutonomousAction,
    #[serde(default)]
    pub success_condition: Option<SuccessCondition>,
    #[serde(default)]
    pub on_failure: FailureStrategy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum SuccessCondition {
    HasItem { item_id: String },
    AtLocation { location: String },
    StatGreaterThan { stat: String, value: f64 },
    StatLessThan { stat: String, value: f64 },
    TimeElapsed { duration_ms: u64 },
    Custom { check_fn: String },
}

impl SuccessCondition {
    pub fn evaluate(&self, context: &ActionContext) -> bool {
        match self {
            Self::HasItem { item_id } => context.inventory.contains(item_id),
            Self::AtLocation { location } => &context.location == location,
            Self::StatGreaterThan { stat, value } => {
                context.stats.get(stat).map_or(false, |v| v > value)
            }
            Self::StatLessThan { stat, value } => {
                context.stats.get(stat).map_or(false, |v| v < value)
            }
            Self::TimeElapsed { duration_ms: _ } => {
                // This would need actual time tracking
                true
            }
            Self::Custom { .. } => {
                // Would need script execution
                true
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureStrategy {
    Abort,
    Retry { max_attempts: u32 },
    Skip,
    Rollback,
    ContinueAnyway,
}

impl Default for FailureStrategy {
    fn default() -> Self {
        Self::Abort
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionReport {
    pub composite_action_id: String,
    pub success: bool,
    pub completed_steps: Vec<String>,
    pub failed_step: Option<String>,
    pub error_message: Option<String>,
    pub total_duration_ms: u64,
    pub rollback_performed: bool,
}

pub struct CompositeActionExecutor;

impl CompositeActionExecutor {
    pub fn new() -> Self {
        Self
    }

    pub fn validate_composite(&self, composite: &CompositeAction) -> Result<(), String> {
        if composite.steps.is_empty() {
            return Err("Composite action must have at least one step".into());
        }

        // Validate step IDs are unique
        let mut step_ids = std::collections::HashSet::new();
        for step in &composite.steps {
            if !step_ids.insert(&step.step_id) {
                return Err(format!("Duplicate step ID: {}", step.step_id));
            }
        }

        Ok(())
    }

    pub fn estimate_duration(&self, composite: &CompositeAction) -> u64 {
        composite
            .steps
            .iter()
            .map(|step| step.action.duration_ms.unwrap_or(1000))
            .sum()
    }

    pub fn get_step_count(&self, composite: &CompositeAction) -> usize {
        composite.steps.len()
    }

    pub fn get_rollback_steps(&self, completed_steps: &[String]) -> Vec<String> {
        let mut rollback = completed_steps.to_vec();
        rollback.reverse();
        rollback
    }
}

impl Default for CompositeActionExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ActionType;

    #[test]
    fn test_composite_creation() {
        let composite = CompositeAction {
            id: "craft_sword".into(),
            name: "Craft Sword".into(),
            description: "Gather materials and craft a sword".into(),
            steps: vec![
                ActionStep {
                    step_id: "get_iron".into(),
                    action: AutonomousAction {
                        action_type: ActionType::Navigate,
                        target: Some("mine".into()),
                        ..Default::default()
                    },
                    success_condition: Some(SuccessCondition::HasItem {
                        item_id: "iron".into(),
                    }),
                    on_failure: FailureStrategy::Retry { max_attempts: 3 },
                },
                ActionStep {
                    step_id: "go_to_forge".into(),
                    action: AutonomousAction {
                        action_type: ActionType::Navigate,
                        target: Some("forge".into()),
                        ..Default::default()
                    },
                    success_condition: Some(SuccessCondition::AtLocation {
                        location: "forge".into(),
                    }),
                    on_failure: FailureStrategy::Abort,
                },
            ],
            rollback_on_failure: true,
        };

        let executor = CompositeActionExecutor::new();
        assert!(executor.validate_composite(&composite).is_ok());
        assert_eq!(executor.get_step_count(&composite), 2);
    }

    #[test]
    fn test_validate_empty_steps() {
        let composite = CompositeAction {
            id: "empty".into(),
            name: "Empty".into(),
            description: "No steps".into(),
            steps: vec![],
            rollback_on_failure: false,
        };

        let executor = CompositeActionExecutor::new();
        assert!(executor.validate_composite(&composite).is_err());
    }

    #[test]
    fn test_validate_duplicate_step_ids() {
        let composite = CompositeAction {
            id: "duplicate".into(),
            name: "Duplicate".into(),
            description: "Duplicate step IDs".into(),
            steps: vec![
                ActionStep {
                    step_id: "step1".into(),
                    action: AutonomousAction::default(),
                    success_condition: None,
                    on_failure: FailureStrategy::Abort,
                },
                ActionStep {
                    step_id: "step1".into(),
                    action: AutonomousAction::default(),
                    success_condition: None,
                    on_failure: FailureStrategy::Abort,
                },
            ],
            rollback_on_failure: false,
        };

        let executor = CompositeActionExecutor::new();
        let result = executor.validate_composite(&composite);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Duplicate"));
    }

    #[test]
    fn test_estimate_duration() {
        let composite = CompositeAction {
            id: "test".into(),
            name: "Test".into(),
            description: "Test".into(),
            steps: vec![
                ActionStep {
                    step_id: "1".into(),
                    action: AutonomousAction {
                        duration_ms: Some(1000),
                        ..Default::default()
                    },
                    success_condition: None,
                    on_failure: FailureStrategy::Abort,
                },
                ActionStep {
                    step_id: "2".into(),
                    action: AutonomousAction {
                        duration_ms: Some(2000),
                        ..Default::default()
                    },
                    success_condition: None,
                    on_failure: FailureStrategy::Abort,
                },
            ],
            rollback_on_failure: false,
        };

        let executor = CompositeActionExecutor::new();
        assert_eq!(executor.estimate_duration(&composite), 3000);
    }

    #[test]
    fn test_rollback_steps() {
        let executor = CompositeActionExecutor::new();
        let completed = vec!["step1".into(), "step2".into(), "step3".into()];
        let rollback = executor.get_rollback_steps(&completed);

        assert_eq!(rollback, vec!["step3", "step2", "step1"]);
    }
}
