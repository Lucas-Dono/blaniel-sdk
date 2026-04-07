use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct AgentRecord {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub description: Option<String>,
    pub personality: Option<String>,
    pub system_prompt: String,
    pub metadata: Option<serde_json::Value>,
    #[sqlx(default)]
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct AgentWithPosition {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub position_x: Option<f64>,
    pub position_y: Option<f64>,
    pub position_z: Option<f64>,
    pub world: Option<String>,
    pub current_activity: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

impl AgentWithPosition {
    pub fn has_position(&self) -> bool {
        self.position_x.is_some() && self.position_y.is_some() && self.position_z.is_some()
    }

    pub fn distance_to(&self, x: f64, y: f64, z: f64) -> Option<f64> {
        if let (Some(px), Some(py), Some(pz)) = (self.position_x, self.position_y, self.position_z) {
            let dx = px - x;
            let dy = py - y;
            let dz = pz - z;
            Some((dx * dx + dy * dy + dz * dz).sqrt())
        } else {
            None
        }
    }
}
