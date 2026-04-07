use std::collections::HashSet;

use npc_types::PathfindOptions;

#[derive(Debug, Clone)]
pub struct NavigationMesh {
    blocked: HashSet<(i64, i64, i64)>,
    water_blocks: HashSet<(i64, i64, i64)>,
    lava_blocks: HashSet<(i64, i64, i64)>,
    ground_height: i64,
    size: i64,
}

impl NavigationMesh {
    pub fn new_flat(size: i64, ground_height: i64) -> Self {
        Self {
            blocked: HashSet::new(),
            water_blocks: HashSet::new(),
            lava_blocks: HashSet::new(),
            ground_height,
            size,
        }
    }

    pub fn with_blocked(mut self, blocks: Vec<(i64, i64, i64)>) -> Self {
        self.blocked = blocks.into_iter().collect();
        self
    }

    pub fn with_water(mut self, blocks: Vec<(i64, i64, i64)>) -> Self {
        self.water_blocks = blocks.into_iter().collect();
        self
    }

    pub fn with_lava(mut self, blocks: Vec<(i64, i64, i64)>) -> Self {
        self.lava_blocks = blocks.into_iter().collect();
        self
    }

    pub fn block(&mut self, x: i64, y: i64, z: i64) {
        self.blocked.insert((x, y, z));
    }

    pub fn unblock(&mut self, x: i64, y: i64, z: i64) {
        self.blocked.remove(&(x, y, z));
    }

    pub fn add_water(&mut self, x: i64, y: i64, z: i64) {
        self.water_blocks.insert((x, y, z));
    }

    pub fn add_lava(&mut self, x: i64, y: i64, z: i64) {
        self.lava_blocks.insert((x, y, z));
    }

    pub fn is_walkable(&self, x: i64, y: i64, z: i64, options: &PathfindOptions) -> bool {
        if self.blocked.contains(&(x, y, z)) {
            return false;
        }

        if x.abs() > self.size || z.abs() > self.size {
            return false;
        }

        if y < self.ground_height - 20 || y > self.ground_height + 10 {
            return false;
        }

        if options.avoid_water && self.water_blocks.contains(&(x, y, z)) {
            return false;
        }

        if options.avoid_lava && self.lava_blocks.contains(&(x, y, z)) {
            return false;
        }

        if y < self.ground_height {
            let fall_distance = self.ground_height - y;
            if fall_distance > options.max_fall_distance as i64 {
                return false;
            }
        }

        true
    }

    pub fn is_blocked(&self, x: i64, y: i64, z: i64) -> bool {
        self.blocked.contains(&(x, y, z))
    }

    pub fn is_water(&self, x: i64, y: i64, z: i64) -> bool {
        self.water_blocks.contains(&(x, y, z))
    }

    pub fn is_lava(&self, x: i64, y: i64, z: i64) -> bool {
        self.lava_blocks.contains(&(x, y, z))
    }

    pub fn blocked_count(&self) -> usize {
        self.blocked.len()
    }

    pub fn water_count(&self) -> usize {
        self.water_blocks.len()
    }

    pub fn lava_count(&self) -> usize {
        self.lava_blocks.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flat_terrain_walkable() {
        let mesh = NavigationMesh::new_flat(100, 64);
        let options = PathfindOptions::default();

        assert!(mesh.is_walkable(0, 64, 0, &options));
        assert!(mesh.is_walkable(10, 64, 10, &options));
    }

    #[test]
    fn test_blocked_terrain() {
        let mut mesh = NavigationMesh::new_flat(100, 64);
        mesh.block(5, 64, 5);

        let options = PathfindOptions::default();

        assert!(!mesh.is_walkable(5, 64, 5, &options));
        assert!(mesh.is_walkable(5, 65, 5, &options));
    }

    #[test]
    fn test_water_avoidance() {
        let mut mesh = NavigationMesh::new_flat(100, 64);
        mesh.add_water(3, 64, 3);

        let options_with_water = PathfindOptions {
            avoid_water: true,
            ..Default::default()
        };

        let options_without_water = PathfindOptions {
            avoid_water: false,
            ..Default::default()
        };

        assert!(!mesh.is_walkable(3, 64, 3, &options_with_water));
        assert!(mesh.is_walkable(3, 64, 3, &options_without_water));
    }

    #[test]
    fn test_lava_avoidance() {
        let mut mesh = NavigationMesh::new_flat(100, 64);
        mesh.add_lava(7, 64, 7);

        let options = PathfindOptions {
            avoid_lava: true,
            ..Default::default()
        };

        assert!(!mesh.is_walkable(7, 64, 7, &options));
    }

    #[test]
    fn test_fall_distance_limit() {
        let mesh = NavigationMesh::new_flat(100, 64);

        let options = PathfindOptions {
            max_fall_distance: 3,
            ..Default::default()
        };

        assert!(mesh.is_walkable(0, 61, 0, &options));
        assert!(!mesh.is_walkable(0, 60, 0, &options));
    }

    #[test]
    fn test_boundary_limits() {
        let mesh = NavigationMesh::new_flat(50, 64);
        let options = PathfindOptions::default();

        assert!(!mesh.is_walkable(51, 64, 0, &options));
        assert!(!mesh.is_walkable(0, 64, 51, &options));
        assert!(mesh.is_walkable(50, 64, 50, &options));
    }
}
