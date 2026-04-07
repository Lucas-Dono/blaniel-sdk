use std::collections::HashMap;

use npc_cache::RedisCache;
use npc_types::{NavigationMode, Position3D, ScenePosition};
use tracing::debug;

use crate::navigation_engine::movement_engine::MovementEngine;

pub struct SceneRegistry {
    redis: RedisCache,
    local_cache: HashMap<String, Vec<ScenePosition>>,
}

impl SceneRegistry {
    pub fn new(redis: RedisCache) -> Self {
        Self {
            redis,
            local_cache: HashMap::new(),
        }
    }

    pub async fn register_scene(
        &mut self,
        scene_id: &str,
        positions: Vec<ScenePosition>,
    ) -> Result<usize, String> {
        let count = positions.len();

        self.local_cache.insert(scene_id.to_string(), positions.clone());

        let mut pipeline_items: Vec<(String, String, u64)> =
            Vec::with_capacity(1 + positions.len() * 3);

        let cache_key = format!("scene:{}:positions", scene_id);
        let json = serde_json::to_string(&positions)
            .map_err(|e| format!("Failed to serialize: {}", e))?;
        pipeline_items.push((cache_key, json, 86400));

        for pos in &positions {
            let name_key = format!("scene:{}:name:{}", scene_id, pos.name.to_lowercase());
            let pos_json = serde_json::to_string(pos)
                .map_err(|e| format!("Failed to serialize position: {}", e))?;
            pipeline_items.push((name_key, pos_json.clone(), 86400));

            for alias in &pos.aliases {
                let alias_key = format!("scene:{}:name:{}", scene_id, alias.to_lowercase());
                pipeline_items.push((alias_key, pos_json.clone(), 86400));
            }
        }

        self.redis
            .set_many_with_ttl(&pipeline_items)
            .await
            .map_err(|e| format!("Redis pipeline error: {}", e))?;

        debug!(
            "Registered scene {} with {} positions ({} Redis keys pipelined)",
            scene_id,
            count,
            pipeline_items.len()
        );
        Ok(count)
    }

    pub async fn resolve_word(
        &self,
        scene_id: &str,
        word: &str,
    ) -> Option<ScenePosition> {
        if let Some(positions) = self.local_cache.get(scene_id) {
            for pos in positions {
                if pos.matches(word) {
                    debug!("Local cache hit for word '{}' in scene '{}'", word, scene_id);
                    return Some(pos.clone());
                }
            }
        }

        let name_key = format!("scene:{}:name:{}", scene_id, word.to_lowercase());
        if let Ok(Some(json)) = self.redis.get::<String>(&name_key).await {
            if let Ok(pos) = serde_json::from_str::<ScenePosition>(&json) {
                debug!("Redis cache hit for word '{}' in scene '{}'", word, scene_id);
                return Some(pos);
            }
        }

        if let Ok(Some(json)) = self
            .redis
            .get::<String>(&format!("scene:{}:positions", scene_id))
            .await
        {
            if let Ok(positions) = serde_json::from_str::<Vec<ScenePosition>>(&json) {
                for pos in &positions {
                    if pos.matches(word) {
                        debug!("Fallback scan match for '{}' in scene '{}'", word, scene_id);
                        return Some(pos.clone());
                    }
                }
            }
        }

        debug!("Word '{}' not found in scene '{}'", word, scene_id);
        None
    }

    pub async fn get_scene_positions(
        &self,
        scene_id: &str,
    ) -> Option<Vec<ScenePosition>> {
        if let Some(positions) = self.local_cache.get(scene_id) {
            return Some(positions.clone());
        }

        let cache_key = format!("scene:{}:positions", scene_id);
        if let Ok(Some(json)) = self.redis.get::<String>(&cache_key).await {
            if let Ok(positions) = serde_json::from_str::<Vec<ScenePosition>>(&json) {
                return Some(positions);
            }
        }

        None
    }

    pub async fn resolve_movement(
        &self,
        engine: &MovementEngine,
        target_word: &str,
        scene_id: &str,
        current_position: &Position3D,
    ) -> Option<npc_types::NavigationResponse> {
        let scene_pos = self.resolve_word(scene_id, target_word).await?;

        let target = &scene_pos.position;

        let distance = current_position.distance_to(target);
        let speed = engine.default_speed();
        let estimated_ms = if speed > 0.0 {
            ((distance / speed) * 1000.0) as u64
        } else {
            0
        };

        Some(npc_types::NavigationResponse {
            success: true,
            from: Some(current_position.clone()),
            to: Some(target.clone()),
            target_name: Some(scene_pos.name.clone()),
            distance: Some(distance),
            estimated_time_ms: Some(estimated_ms),
            navigation_mode: NavigationMode::Word.as_str().to_string(),
        })
    }

    pub async fn delete_scene(&mut self, scene_id: &str) -> Result<(), String> {
        self.local_cache.remove(scene_id);

        let pattern = format!("scene:{}:*", scene_id);
        self.redis
            .del_pattern(&pattern)
            .await
            .map_err(|e| format!("Redis error: {}", e))?;

        debug!("Deleted scene {}", scene_id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scene_position_matching() {
        let pos = ScenePosition {
            name: "bed".to_string(),
            position: Position3D::new(10.0, 0.0, 5.0, "house".to_string()),
            aliases: vec!["cama".to_string()],
            tags: vec!["furniture".to_string()],
        };

        assert!(pos.matches("bed"));
        assert!(pos.matches("cama"));
        assert!(!pos.matches("chair"));
    }
}
