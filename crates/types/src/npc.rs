use crate::{Animation, FacingDirection, NpcAction, Position3D};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcState {
    pub id: String,
    pub name: String,
    pub position: Option<Position3D>,
    pub current_action: NpcAction,
    pub animation: Animation,
    pub facing_direction: FacingDirection,
    pub cached_emotion: String,
    pub cached_at: i64, // Unix timestamp
    pub metadata: Option<serde_json::Value>,
}

impl NpcState {
    pub fn is_cache_stale(&self, max_age_seconds: i64) -> bool {
        let now = chrono::Utc::now().timestamp();
        now - self.cached_at > max_age_seconds
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcMetadata {
    pub trust_level: Option<f64>,
    pub affinity: Option<f64>,
    pub last_interaction: Option<i64>,
    pub interaction_count: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProximityQuery {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub radius: f64,
    pub world: String,
    pub limit: Option<usize>,
}

impl ProximityQuery {
    pub fn validate(&self) -> Result<(), String> {
        // Validate coordinates are finite numbers
        if !self.x.is_finite() {
            return Err("X coordinate must be a finite number".to_string());
        }
        if !self.y.is_finite() {
            return Err("Y coordinate must be a finite number".to_string());
        }
        if !self.z.is_finite() {
            return Err("Z coordinate must be a finite number".to_string());
        }

        // Validate radius
        if !self.radius.is_finite() {
            return Err("Radius must be a finite number".to_string());
        }
        if self.radius <= 0.0 {
            return Err("Radius must be positive".to_string());
        }
        if self.radius > 1000.0 {
            return Err("Radius too large (max 1000)".to_string());
        }

        // Validate world name
        if self.world.is_empty() {
            return Err("World name cannot be empty".to_string());
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchStateRequest {
    pub agent_ids: Vec<String>,
}

impl BatchStateRequest {
    pub fn validate(&self) -> Result<(), String> {
        if self.agent_ids.is_empty() {
            return Err("Agent IDs list cannot be empty".to_string());
        }
        if self.agent_ids.len() > 100 {
            return Err("Too many agent IDs (max 100)".to_string());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proximity_query_validation() {
        let valid = ProximityQuery {
            x: 0.0,
            y: 64.0,
            z: 0.0,
            radius: 50.0,
            world: "overworld".to_string(),
            limit: None,
        };
        assert!(valid.validate().is_ok());

        let invalid_radius = ProximityQuery {
            x: 0.0,
            y: 64.0,
            z: 0.0,
            radius: -5.0,
            world: "overworld".to_string(),
            limit: None,
        };
        assert!(invalid_radius.validate().is_err());

        let too_large = ProximityQuery {
            x: 0.0,
            y: 64.0,
            z: 0.0,
            radius: 2000.0,
            world: "overworld".to_string(),
            limit: None,
        };
        assert!(too_large.validate().is_err());
    }

    #[test]
    fn test_cache_staleness() {
        let now = chrono::Utc::now().timestamp();

        let fresh_state = NpcState {
            id: "test".to_string(),
            name: "Test NPC".to_string(),
            position: None,
            current_action: NpcAction::Idle,
            animation: Animation::Idle,
            facing_direction: FacingDirection::North,
            cached_emotion: "neutral".to_string(),
            cached_at: now,
            metadata: None,
        };

        assert!(!fresh_state.is_cache_stale(60));

        let stale_state = NpcState {
            cached_at: now - 120,
            ..fresh_state
        };

        assert!(stale_state.is_cache_stale(60));
    }
}
