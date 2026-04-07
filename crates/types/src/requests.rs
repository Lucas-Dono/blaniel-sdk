use crate::{NpcAction, Position3D, Weather};
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct ChatRequest {
    #[validate(length(min = 1, max = 1000))]
    pub message: String,
    pub context: Option<ChatContext>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatContext {
    pub position: Option<Position3D>,
    pub activity: Option<NpcAction>,
    pub nearby_players: Vec<String>,
    pub time_of_day: Option<u32>, // 0-24000 (game time in ticks)
    pub weather: Option<Weather>,
    pub nearby_npcs: Vec<String>,
}

impl Default for ChatContext {
    fn default() -> Self {
        Self {
            position: None,
            activity: None,
            nearby_players: Vec::new(),
            time_of_day: None,
            weather: None,
            nearby_npcs: Vec::new(),
        }
    }
}

impl ChatContext {
    pub fn is_daytime(&self) -> bool {
        if let Some(time) = self.time_of_day {
            time >= 0 && time < 12000
        } else {
            true
        }
    }

    pub fn player_count(&self) -> usize {
        self.nearby_players.len()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathfindRequest {
    pub start: Position3D,
    pub goal: Position3D,
    pub options: PathfindOptions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathfindOptions {
    #[serde(default = "default_true")]
    pub avoid_water: bool,

    #[serde(default = "default_true")]
    pub avoid_lava: bool,

    #[serde(default = "default_max_fall")]
    pub max_fall_distance: u32,

    #[serde(default = "default_true")]
    pub allow_diagonal: bool,

    #[serde(default)]
    pub max_iterations: Option<usize>,
}

impl Default for PathfindOptions {
    fn default() -> Self {
        Self {
            avoid_water: true,
            avoid_lava: true,
            max_fall_distance: 3,
            allow_diagonal: true,
            max_iterations: None,
        }
    }
}

impl PathfindOptions {
    pub fn hash(&self) -> String {
        format!(
            "{}:{}:{}:{}",
            self.avoid_water as u8,
            self.avoid_lava as u8,
            self.max_fall_distance,
            self.allow_diagonal as u8
        )
    }
}

fn default_true() -> bool {
    true
}

fn default_max_fall() -> u32 {
    3
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoveRequest {
    pub position: Position3D,
    pub action: Option<NpcAction>,
    pub facing_direction: Option<f64>, // Yaw angle
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmbientDialogueQuery {
    pub participant_ids: Vec<String>,
    pub context: String, // e.g., "tavern", "marketplace", "forest"
    pub max_exchanges: Option<usize>,
}

impl AmbientDialogueQuery {
    pub fn hash(&self) -> String {
        let mut ids = self.participant_ids.clone();
        ids.sort();
        format!("{}:{}", ids.join(","), self.context)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvalidationRequest {
    pub event_type: String, // "emotional_update", "position_update", "personality_change"
    pub agent_id: String,
    pub timestamp: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_context_defaults() {
        let ctx = ChatContext::default();
        assert_eq!(ctx.player_count(), 0);
        assert!(ctx.is_daytime()); // Default is daytime
    }

    #[test]
    fn test_daytime_detection() {
        let mut ctx = ChatContext::default();
        ctx.time_of_day = Some(6000); // Noon
        assert!(ctx.is_daytime());

        ctx.time_of_day = Some(18000); // Midnight
        assert!(!ctx.is_daytime());
    }

    #[test]
    fn test_pathfind_options_hash() {
        let opts = PathfindOptions::default();
        let hash1 = opts.hash();

        let mut opts2 = PathfindOptions::default();
        opts2.max_fall_distance = 5;
        let hash2 = opts2.hash();

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_ambient_dialogue_hash() {
        let query1 = AmbientDialogueQuery {
            participant_ids: vec!["agent1".to_string(), "agent2".to_string()],
            context: "tavern".to_string(),
            max_exchanges: None,
        };

        let query2 = AmbientDialogueQuery {
            participant_ids: vec!["agent2".to_string(), "agent1".to_string()],
            context: "tavern".to_string(),
            max_exchanges: None,
        };

        // Should be the same hash regardless of order
        assert_eq!(query1.hash(), query2.hash());
    }
}
