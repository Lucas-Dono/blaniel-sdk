use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NpcAction {
    Idle,
    FollowingPlayer,
    Working,
    Sleeping,
    Walking,
    Running,
    Trading,
    Talking,
    Mining,
    Building,
    Farming,
    Fishing,
    Exploring,
}

impl Default for NpcAction {
    fn default() -> Self {
        NpcAction::Idle
    }
}

impl NpcAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            NpcAction::Idle => "idle",
            NpcAction::FollowingPlayer => "following_player",
            NpcAction::Working => "working",
            NpcAction::Sleeping => "sleeping",
            NpcAction::Walking => "walking",
            NpcAction::Running => "running",
            NpcAction::Trading => "trading",
            NpcAction::Talking => "talking",
            NpcAction::Mining => "mining",
            NpcAction::Building => "building",
            NpcAction::Farming => "farming",
            NpcAction::Fishing => "fishing",
            NpcAction::Exploring => "exploring",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "idle" => Some(NpcAction::Idle),
            "following_player" => Some(NpcAction::FollowingPlayer),
            "working" => Some(NpcAction::Working),
            "sleeping" => Some(NpcAction::Sleeping),
            "walking" => Some(NpcAction::Walking),
            "running" => Some(NpcAction::Running),
            "trading" => Some(NpcAction::Trading),
            "talking" => Some(NpcAction::Talking),
            "mining" => Some(NpcAction::Mining),
            "building" => Some(NpcAction::Building),
            "farming" => Some(NpcAction::Farming),
            "fishing" => Some(NpcAction::Fishing),
            "exploring" => Some(NpcAction::Exploring),
            _ => None,
        }
    }

    pub fn is_moving(&self) -> bool {
        matches!(
            self,
            NpcAction::Walking
                | NpcAction::Running
                | NpcAction::FollowingPlayer
                | NpcAction::Exploring
        )
    }

    pub fn is_interruptible(&self) -> bool {
        !matches!(self, NpcAction::Sleeping | NpcAction::Trading)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Animation {
    Idle,
    Walk,
    Run,
    Wave,
    Nod,
    Shake,
    Point,
    Work,
    Sleep,
    Trade,
    Talk,
    Emote,
}

impl Default for Animation {
    fn default() -> Self {
        Animation::Idle
    }
}

impl Animation {
    pub fn as_str(&self) -> &'static str {
        match self {
            Animation::Idle => "idle",
            Animation::Walk => "walk",
            Animation::Run => "run",
            Animation::Wave => "wave",
            Animation::Nod => "nod",
            Animation::Shake => "shake",
            Animation::Point => "point",
            Animation::Work => "work",
            Animation::Sleep => "sleep",
            Animation::Trade => "trade",
            Animation::Talk => "talk",
            Animation::Emote => "emote",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_action_serialization() {
        let action = NpcAction::Walking;
        let json = serde_json::to_string(&action).unwrap();
        assert_eq!(json, r#""walking""#);
    }

    #[test]
    fn test_action_from_str() {
        assert_eq!(NpcAction::from_str("walking"), Some(NpcAction::Walking));
        assert_eq!(NpcAction::from_str("WALKING"), Some(NpcAction::Walking));
        assert_eq!(NpcAction::from_str("invalid"), None);
    }

    #[test]
    fn test_is_moving() {
        assert!(NpcAction::Walking.is_moving());
        assert!(NpcAction::Running.is_moving());
        assert!(!NpcAction::Idle.is_moving());
        assert!(!NpcAction::Sleeping.is_moving());
    }
}
