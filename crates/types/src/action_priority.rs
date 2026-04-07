use serde::{Deserialize, Serialize};

use crate::ActionCategory;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionPriority {
    Background = 0,
    Low = 1,
    Normal = 2,
    High = 3,
    Critical = 4,
}

impl Default for ActionPriority {
    fn default() -> Self {
        Self::Normal
    }
}

impl ActionPriority {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Background => "background",
            Self::Low => "low",
            Self::Normal => "normal",
            Self::High => "high",
            Self::Critical => "critical",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "background" => Some(Self::Background),
            "low" => Some(Self::Low),
            "normal" => Some(Self::Normal),
            "high" => Some(Self::High),
            "critical" => Some(Self::Critical),
            _ => None,
        }
    }
}

impl ActionCategory {
    pub fn default_priority(&self) -> ActionPriority {
        match self {
            Self::Combat | Self::Healing => ActionPriority::High,
            Self::Magic => ActionPriority::High,
            Self::Stealth => ActionPriority::High,
            Self::Crafting | Self::Building => ActionPriority::Normal,
            Self::Trading => ActionPriority::Normal,
            Self::Exploration | Self::Quest => ActionPriority::Normal,
            Self::Navigation | Self::Interaction => ActionPriority::Normal,
            Self::Social | Self::Dialogue => ActionPriority::Low,
            Self::Emotes | Self::Music => ActionPriority::Low,
            Self::Farming | Self::Fishing | Self::Mining | Self::Cooking => ActionPriority::Background,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_priority_ordering() {
        assert!(ActionPriority::Critical > ActionPriority::High);
        assert!(ActionPriority::High > ActionPriority::Normal);
        assert!(ActionPriority::Normal > ActionPriority::Low);
        assert!(ActionPriority::Low > ActionPriority::Background);
    }

    #[test]
    fn test_priority_from_str() {
        assert_eq!(
            ActionPriority::from_str("critical"),
            Some(ActionPriority::Critical)
        );
        assert_eq!(
            ActionPriority::from_str("HIGH"),
            Some(ActionPriority::High)
        );
        assert_eq!(ActionPriority::from_str("invalid"), None);
    }

    #[test]
    fn test_category_default_priorities() {
        assert_eq!(ActionCategory::Combat.default_priority(), ActionPriority::High);
        assert_eq!(
            ActionCategory::Social.default_priority(),
            ActionPriority::Low
        );
        assert_eq!(
            ActionCategory::Farming.default_priority(),
            ActionPriority::Background
        );
    }
}
