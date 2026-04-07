use npc_types::{
    MovementRestrictions, NavigationMode, NavigationResponse, NavigationTarget, Position3D,
};
use tracing::debug;

pub struct MovementEngine {
    default_speed: f64,
}

impl MovementEngine {
    pub fn new(default_speed: f64) -> Self {
        Self { default_speed }
    }

    pub fn default_speed(&self) -> f64 {
        self.default_speed
    }

    pub fn resolve_target(
        &self,
        target: &NavigationTarget,
        scene_positions: &[npc_types::ScenePosition],
    ) -> Option<(Position3D, Option<String>)> {
        match target {
            NavigationTarget::Word { name } => {
                for pos in scene_positions {
                    if pos.matches(name) {
                        return Some((pos.position.clone(), Some(pos.name.clone())));
                    }
                }
                debug!(
                    "Word '{}' not found in {} scene positions",
                    name,
                    scene_positions.len()
                );
                None
            }
            NavigationTarget::Coordinate { position } => Some((position.clone(), None)),
            NavigationTarget::Relative { .. } => None,
        }
    }

    pub fn resolve_relative(&self, current: &Position3D, dx: f64, dy: f64, dz: f64) -> Position3D {
        Position3D::new(
            current.x + dx,
            current.y + dy,
            current.z + dz,
            current.world.clone(),
        )
    }

    pub fn validate_movement(
        &self,
        from: &Position3D,
        to: &Position3D,
        restrictions: Option<&MovementRestrictions>,
    ) -> Result<(), String> {
        let Some(r) = restrictions else {
            return Ok(());
        };

        if !r.allowed_worlds.is_empty() && !r.allowed_worlds.contains(&to.world) {
            return Err(format!("World '{}' is not allowed", to.world));
        }

        if let Some(max_dist) = r.max_distance {
            let dist = from.distance_to(to);
            if dist > max_dist {
                return Err(format!(
                    "Distance {:.1} exceeds maximum {:.1}",
                    dist, max_dist
                ));
            }
        }

        if let Some(ref bounds) = r.bounds {
            if !bounds.contains(to) {
                return Err("Target position is out of bounds".to_string());
            }
        }

        for blocked in &r.blocked_positions {
            if to.x == blocked.x
                && to.y == blocked.y
                && to.z == blocked.z
                && to.world == blocked.world
            {
                return Err("Target position is blocked".to_string());
            }
        }

        Ok(())
    }

    pub fn calculate_response(
        &self,
        from: &Position3D,
        to: &Position3D,
        target_name: Option<String>,
        mode: NavigationMode,
        speed_override: Option<f64>,
    ) -> NavigationResponse {
        let distance = from.distance_to(to);
        let speed = speed_override.unwrap_or(self.default_speed);
        let estimated_ms = if speed > 0.0 {
            ((distance / speed) * 1000.0) as u64
        } else {
            0
        };

        NavigationResponse {
            success: true,
            from: Some(from.clone()),
            to: Some(to.clone()),
            target_name,
            distance: Some(distance),
            estimated_time_ms: Some(estimated_ms),
            navigation_mode: mode.as_str().to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use npc_types::WorldBounds;

    #[test]
    fn test_validate_within_bounds() {
        let engine = MovementEngine::new(4.0);
        let from = Position3D::new(0.0, 0.0, 0.0, "house".to_string());
        let to = Position3D::new(5.0, 0.0, 5.0, "house".to_string());
        let restrictions = MovementRestrictions {
            max_distance: Some(10.0),
            bounds: Some(WorldBounds {
                min_x: -10.0,
                min_y: -5.0,
                min_z: -10.0,
                max_x: 10.0,
                max_y: 5.0,
                max_z: 10.0,
                world: "house".to_string(),
            }),
            blocked_positions: vec![],
            allowed_worlds: vec!["house".to_string()],
        };

        assert!(engine
            .validate_movement(&from, &to, Some(&restrictions))
            .is_ok());
    }

    #[test]
    fn test_validate_exceeds_distance() {
        let engine = MovementEngine::new(4.0);
        let from = Position3D::new(0.0, 0.0, 0.0, "house".to_string());
        let to = Position3D::new(50.0, 0.0, 0.0, "house".to_string());
        let restrictions = MovementRestrictions {
            max_distance: Some(10.0),
            bounds: None,
            blocked_positions: vec![],
            allowed_worlds: vec![],
        };

        let result = engine.validate_movement(&from, &to, Some(&restrictions));
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_blocked_position() {
        let engine = MovementEngine::new(4.0);
        let from = Position3D::new(0.0, 0.0, 0.0, "house".to_string());
        let to = Position3D::new(5.0, 0.0, 5.0, "house".to_string());
        let restrictions = MovementRestrictions {
            max_distance: None,
            bounds: None,
            blocked_positions: vec![Position3D::new(5.0, 0.0, 5.0, "house".to_string())],
            allowed_worlds: vec![],
        };

        let result = engine.validate_movement(&from, &to, Some(&restrictions));
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_no_restrictions() {
        let engine = MovementEngine::new(4.0);
        let from = Position3D::new(0.0, 0.0, 0.0, "world".to_string());
        let to = Position3D::new(1000.0, 500.0, 1000.0, "world".to_string());

        assert!(engine.validate_movement(&from, &to, None).is_ok());
    }

    #[test]
    fn test_calculate_response() {
        let engine = MovementEngine::new(4.0);
        let from = Position3D::new(0.0, 0.0, 0.0, "house".to_string());
        let to = Position3D::new(4.0, 0.0, 0.0, "house".to_string());

        let response = engine.calculate_response(
            &from,
            &to,
            Some("bed".to_string()),
            NavigationMode::Word,
            None,
        );

        assert!(response.success);
        assert_eq!(response.distance.unwrap(), 4.0);
        assert_eq!(response.estimated_time_ms.unwrap(), 1000);
        assert_eq!(response.navigation_mode, "word");
    }

    #[test]
    fn test_resolve_relative() {
        let engine = MovementEngine::new(4.0);
        let current = Position3D::new(10.0, 0.0, 20.0, "house".to_string());

        let result = engine.resolve_relative(&current, 5.0, 1.0, -3.0);

        assert_eq!(result.x, 15.0);
        assert_eq!(result.y, 1.0);
        assert_eq!(result.z, 17.0);
        assert_eq!(result.world, "house");
    }
}
