use serde::{Deserialize, Serialize};

use crate::{ActionPriority, AutonomousAction};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InterruptionLevel {
    NonInterruptible,
    HighPriority,
    Normal,
    Interruptible,
}

impl Default for InterruptionLevel {
    fn default() -> Self {
        Self::Interruptible
    }
}

impl InterruptionLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::NonInterruptible => "non_interruptible",
            Self::HighPriority => "high_priority",
            Self::Normal => "normal",
            Self::Interruptible => "interruptible",
        }
    }

    pub fn can_be_interrupted_by(&self, priority: ActionPriority) -> bool {
        match self {
            Self::NonInterruptible => false,
            Self::HighPriority => priority >= ActionPriority::Critical,
            Self::Normal => priority >= ActionPriority::High,
            Self::Interruptible => true,
        }
    }
}

pub trait Interruptible {
    fn can_be_interrupted_by(&self, new_action: &AutonomousAction) -> bool;
    fn get_interruption_level(&self) -> InterruptionLevel;
}

impl Interruptible for AutonomousAction {
    fn can_be_interrupted_by(&self, new_action: &AutonomousAction) -> bool {
        self.get_interruption_level()
            .can_be_interrupted_by(new_action.get_priority())
    }

    fn get_interruption_level(&self) -> InterruptionLevel {
        // Default mapping based on action type and priority
        match self.priority {
            ActionPriority::Critical => InterruptionLevel::NonInterruptible,
            ActionPriority::High => InterruptionLevel::HighPriority,
            ActionPriority::Normal => InterruptionLevel::Normal,
            ActionPriority::Low | ActionPriority::Background => InterruptionLevel::Interruptible,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterruptionRequest {
    pub current_action_id: String,
    pub new_action: AutonomousAction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterruptionResponse {
    pub allowed: bool,
    pub reason: String,
    pub current_interruption_level: String,
    pub new_action_priority: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ActionType;

    #[test]
    fn test_interruption_level_ordering() {
        assert!(
            InterruptionLevel::NonInterruptible
                .can_be_interrupted_by(ActionPriority::Critical)
                == false
        );
        assert!(
            InterruptionLevel::HighPriority.can_be_interrupted_by(ActionPriority::Critical) == true
        );
        assert!(InterruptionLevel::Normal.can_be_interrupted_by(ActionPriority::High) == true);
        assert!(
            InterruptionLevel::Interruptible.can_be_interrupted_by(ActionPriority::Low) == true
        );
    }

    #[test]
    fn test_action_interruption() {
        let current = AutonomousAction {
            action_type: ActionType::Wait,
            priority: ActionPriority::Normal,
            ..Default::default()
        };

        let high_priority_new = AutonomousAction {
            action_type: ActionType::Navigate,
            priority: ActionPriority::High,
            ..Default::default()
        };

        let low_priority_new = AutonomousAction {
            action_type: ActionType::Emote,
            priority: ActionPriority::Low,
            ..Default::default()
        };

        assert!(current.can_be_interrupted_by(&high_priority_new));
        assert!(!current.can_be_interrupted_by(&low_priority_new));
    }

    #[test]
    fn test_critical_action_non_interruptible() {
        let critical = AutonomousAction {
            action_type: ActionType::Navigate,
            priority: ActionPriority::Critical,
            ..Default::default()
        };

        let another_critical = AutonomousAction {
            action_type: ActionType::Wait,
            priority: ActionPriority::Critical,
            ..Default::default()
        };

        assert!(!critical.can_be_interrupted_by(&another_critical));
    }
}
