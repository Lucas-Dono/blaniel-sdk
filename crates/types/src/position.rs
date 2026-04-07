use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Position3D {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub world: String, // "overworld", "nether", "end"
}

impl Position3D {
    pub fn new(x: f64, y: f64, z: f64, world: String) -> Self {
        Self { x, y, z, world }
    }

    pub fn distance_to(&self, other: &Position3D) -> f64 {
        if self.world != other.world {
            return f64::INFINITY;
        }

        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;

        (dx * dx + dy * dy + dz * dz).sqrt()
    }

    pub fn coords(&self) -> String {
        format!("{}:{}:{}", self.x as i64, self.y as i64, self.z as i64)
    }

    pub fn hash_key(&self) -> String {
        format!(
            "{}:{}:{}:{}",
            self.world, self.x as i64, self.y as i64, self.z as i64
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FacingDirection {
    North,
    South,
    East,
    West,
}

impl FacingDirection {
    pub fn from_yaw(yaw: f64) -> Self {
        let normalized = (yaw % 360.0 + 360.0) % 360.0;

        match normalized {
            y if y >= 315.0 || y < 45.0 => FacingDirection::South,
            y if y >= 45.0 && y < 135.0 => FacingDirection::West,
            y if y >= 135.0 && y < 225.0 => FacingDirection::North,
            _ => FacingDirection::East,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Weather {
    Clear,
    Rain,
    Thunder,
    Snow,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_distance_calculation() {
        let pos1 = Position3D::new(0.0, 0.0, 0.0, "overworld".to_string());
        let pos2 = Position3D::new(3.0, 4.0, 0.0, "overworld".to_string());

        assert_eq!(pos1.distance_to(&pos2), 5.0);
    }

    #[test]
    fn test_different_worlds() {
        let pos1 = Position3D::new(0.0, 0.0, 0.0, "overworld".to_string());
        let pos2 = Position3D::new(0.0, 0.0, 0.0, "nether".to_string());

        assert_eq!(pos1.distance_to(&pos2), f64::INFINITY);
    }

    #[test]
    fn test_facing_direction() {
        assert_eq!(FacingDirection::from_yaw(0.0), FacingDirection::South);
        assert_eq!(FacingDirection::from_yaw(90.0), FacingDirection::West);
        assert_eq!(FacingDirection::from_yaw(180.0), FacingDirection::North);
        assert_eq!(FacingDirection::from_yaw(270.0), FacingDirection::East);
    }
}
