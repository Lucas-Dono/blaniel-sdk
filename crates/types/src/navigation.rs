use serde::{Deserialize, Serialize};

use crate::{ActionPriority, Position3D};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NavigationMode {
    Word,
    Coordinate,
    Hybrid,
    Free,
}

impl NavigationMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Word => "word",
            Self::Coordinate => "coordinate",
            Self::Hybrid => "hybrid",
            Self::Free => "free",
        }
    }
}

impl Default for NavigationMode {
    fn default() -> Self {
        Self::Hybrid
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NavigationConfig {
    pub mode: NavigationMode,
    #[serde(default)]
    pub restrictions: Option<MovementRestrictions>,
    #[serde(default)]
    pub scene_id: Option<String>,
    #[serde(default)]
    pub default_speed: f64,
    #[serde(default)]
    pub introduction: String,
    #[serde(default)]
    pub explain: String,
}

impl Default for NavigationConfig {
    fn default() -> Self {
        Self {
            mode: NavigationMode::Hybrid,
            restrictions: None,
            scene_id: None,
            default_speed: 4.0,
            introduction: String::new(),
            explain: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MovementRestrictions {
    #[serde(default)]
    pub max_distance: Option<f64>,
    #[serde(default)]
    pub bounds: Option<WorldBounds>,
    #[serde(default)]
    pub blocked_positions: Vec<Position3D>,
    #[serde(default)]
    pub allowed_worlds: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldBounds {
    pub min_x: f64,
    pub min_y: f64,
    pub min_z: f64,
    pub max_x: f64,
    pub max_y: f64,
    pub max_z: f64,
    pub world: String,
}

impl WorldBounds {
    pub fn contains(&self, pos: &Position3D) -> bool {
        pos.world == self.world
            && pos.x >= self.min_x
            && pos.x <= self.max_x
            && pos.y >= self.min_y
            && pos.y <= self.max_y
            && pos.z >= self.min_z
            && pos.z <= self.max_z
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenePosition {
    pub name: String,
    pub position: Position3D,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

impl ScenePosition {
    pub fn matches(&self, query: &str) -> bool {
        let q = query.to_lowercase();
        self.name.to_lowercase() == q
            || self.aliases.iter().any(|a| a.to_lowercase() == q)
            || self.tags.iter().any(|t| t.to_lowercase() == q)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterSceneRequest {
    pub scene_id: String,
    pub world: String,
    pub positions: Vec<ScenePosition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NavigationRequest {
    pub target: NavigationTarget,
    #[serde(default)]
    pub speed: Option<f64>,
    #[serde(default)]
    pub callback_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum NavigationTarget {
    Word { name: String },
    Coordinate { position: Position3D },
    Relative { dx: f64, dy: f64, dz: f64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NavigationResponse {
    pub success: bool,
    pub from: Option<Position3D>,
    pub to: Option<Position3D>,
    pub target_name: Option<String>,
    pub distance: Option<f64>,
    pub estimated_time_ms: Option<u64>,
    pub navigation_mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AutonomousAction {
    #[serde(rename = "type")]
    pub action_type: ActionType,
    #[serde(default)]
    pub target: Option<String>,
    #[serde(default)]
    pub position: Option<Position3D>,
    #[serde(default)]
    pub duration_ms: Option<u64>,
    #[serde(default)]
    pub animation: Option<String>,
    #[serde(default)]
    pub emotion: Option<String>,
    #[serde(default)]
    pub delay_ms: Option<u64>,
    #[serde(default)]
    pub priority: ActionPriority,
}

impl AutonomousAction {
    pub fn get_priority(&self) -> ActionPriority {
        self.priority
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ActionType {
    Navigate,
    Wait,
    Animate,
    Emote,
    Speak,
}

impl ActionType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Navigate => "navigate",
            Self::Wait => "wait",
            Self::Animate => "animate",
            Self::Emote => "emote",
            Self::Speak => "speak",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionPlan {
    pub actions: Vec<AutonomousAction>,
    #[serde(default)]
    pub loop_plan: bool,
    #[serde(default)]
    pub interrupt_on_chat: bool,
}

impl ActionPlan {
    pub fn sort_by_priority(&mut self) {
        self.actions.sort_by(|a, b| b.priority.cmp(&a.priority));
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.actions.is_empty() {
            return Err("Action plan must have at least one action".into());
        }

        for (idx, action) in self.actions.iter().enumerate() {
            match action.action_type {
                ActionType::Navigate => {
                    if action.target.is_none() && action.position.is_none() {
                        return Err(format!(
                            "Action {} (Navigate) requires either 'target' or 'position'",
                            idx
                        ));
                    }
                }
                ActionType::Wait => {
                    if action.duration_ms.is_none() {
                        return Err(format!("Action {} (Wait) requires 'duration_ms'", idx));
                    }
                }
                ActionType::Animate | ActionType::Emote => {
                    if action.animation.is_none() && action.emotion.is_none() {
                        return Err(format!(
                            "Action {} ({:?}) requires 'animation' or 'emotion'",
                            idx, action.action_type
                        ));
                    }
                }
                ActionType::Speak => {
                    if action.target.is_none() {
                        return Err(format!(
                            "Action {} (Speak) requires 'target' with dialogue text",
                            idx
                        ));
                    }
                }
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionPlanRequest {
    pub plan: ActionPlan,
    #[serde(default)]
    pub navigation_config: Option<NavigationConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionPlanResponse {
    pub plan_id: String,
    pub action_count: usize,
    pub total_estimated_ms: u64,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenePositionsResponse {
    pub scene_id: String,
    pub positions: Vec<ScenePosition>,
    pub total: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_navigation_mode_default() {
        assert_eq!(NavigationMode::default(), NavigationMode::Hybrid);
    }

    #[test]
    fn test_scene_position_matching() {
        let pos = ScenePosition {
            name: "bed".to_string(),
            position: Position3D::new(10.0, 0.0, 5.0, "house".to_string()),
            aliases: vec!["cama".to_string(), "lecho".to_string()],
            tags: vec!["furniture".to_string(), "sleep".to_string()],
        };

        assert!(pos.matches("bed"));
        assert!(pos.matches("BED"));
        assert!(pos.matches("cama"));
        assert!(pos.matches("CAMA"));
        assert!(!pos.matches("chair"));
    }

    #[test]
    fn test_world_bounds() {
        let bounds = WorldBounds {
            min_x: -10.0,
            min_y: 0.0,
            min_z: -10.0,
            max_x: 10.0,
            max_y: 10.0,
            max_z: 10.0,
            world: "house".to_string(),
        };

        let inside = Position3D::new(5.0, 3.0, 5.0, "house".to_string());
        let outside = Position3D::new(15.0, 3.0, 5.0, "house".to_string());
        let wrong_world = Position3D::new(5.0, 3.0, 5.0, "garden".to_string());

        assert!(bounds.contains(&inside));
        assert!(!bounds.contains(&outside));
        assert!(!bounds.contains(&wrong_world));
    }

    #[test]
    fn test_movement_restrictions_validate() {
        let restrictions = MovementRestrictions {
            max_distance: Some(50.0),
            bounds: Some(WorldBounds {
                min_x: 0.0,
                min_y: 0.0,
                min_z: 0.0,
                max_x: 100.0,
                max_y: 10.0,
                max_z: 100.0,
                world: "house".to_string(),
            }),
            blocked_positions: vec![Position3D::new(50.0, 0.0, 50.0, "house".to_string())],
            allowed_worlds: vec!["house".to_string()],
        };

        assert!(restrictions.max_distance == Some(50.0));
        assert!(restrictions.blocked_positions.len() == 1);
    }

    #[test]
    fn test_action_plan_serialization() {
        let plan = ActionPlan {
            actions: vec![
                AutonomousAction {
                    action_type: ActionType::Navigate,
                    target: Some("bed".to_string()),
                    position: None,
                    duration_ms: None,
                    animation: Some("walking".to_string()),
                    emotion: None,
                    delay_ms: None,
                },
                AutonomousAction {
                    action_type: ActionType::Wait,
                    target: None,
                    position: None,
                    duration_ms: Some(3000),
                    animation: None,
                    emotion: None,
                    delay_ms: None,
                },
            ],
            loop_plan: false,
            interrupt_on_chat: true,
        };

        let json = serde_json::to_string(&plan).unwrap();
        assert!(json.contains("navigate"));
        assert!(json.contains("interrupt_on_chat"));
    }
}
