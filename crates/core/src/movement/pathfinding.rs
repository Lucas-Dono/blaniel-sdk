use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

use priority_queue::DoublePriorityQueue;
use tracing::{debug, instrument};

use npc_types::{PathResult, PathfindOptions, Position3D};

use crate::movement::navigation::NavigationMesh;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Node {
    x: i64,
    y: i64,
    z: i64,
}

impl Node {
    fn from_position(pos: &Position3D) -> Self {
        Self {
            x: pos.x.round() as i64,
            y: pos.y.round() as i64,
            z: pos.z.round() as i64,
        }
    }

    fn to_position(&self, world: &str) -> Position3D {
        Position3D::new(
            self.x as f64,
            self.y as f64,
            self.z as f64,
            world.to_string(),
        )
    }

    fn heuristic(&self, goal: &Node) -> f64 {
        let dx = (self.x - goal.x).abs() as f64;
        let dy = (self.y - goal.y).abs() as f64;
        let dz = (self.z - goal.z).abs() as f64;
        (dx + dy + dz) - 0.586 * dx.min(dy).min(dz)
    }

    fn neighbors(&self, allow_diagonal: bool) -> Vec<(Node, f64)> {
        let mut neighbors = Vec::with_capacity(26);
        let directions: &[(&[i64], f64)] = if allow_diagonal {
            &[
                (&[1, 0, 0], 1.0),
                (&[-1, 0, 0], 1.0),
                (&[0, 1, 0], 1.0),
                (&[0, -1, 0], 1.0),
                (&[0, 0, 1], 1.0),
                (&[0, 0, -1], 1.0),
                (&[1, 1, 0], 1.414),
                (&[1, -1, 0], 1.414),
                (&[-1, 1, 0], 1.414),
                (&[-1, -1, 0], 1.414),
                (&[1, 0, 1], 1.414),
                (&[1, 0, -1], 1.414),
                (&[-1, 0, 1], 1.414),
                (&[-1, 0, -1], 1.414),
                (&[0, 1, 1], 1.414),
                (&[0, 1, -1], 1.414),
                (&[0, -1, 1], 1.414),
                (&[0, -1, -1], 1.414),
                (&[1, 1, 1], 1.732),
                (&[1, 1, -1], 1.732),
                (&[1, -1, 1], 1.732),
                (&[1, -1, -1], 1.732),
                (&[-1, 1, 1], 1.732),
                (&[-1, 1, -1], 1.732),
                (&[-1, -1, 1], 1.732),
                (&[-1, -1, -1], 1.732),
            ]
        } else {
            &[
                (&[1, 0, 0], 1.0),
                (&[-1, 0, 0], 1.0),
                (&[0, 1, 0], 1.0),
                (&[0, -1, 0], 1.0),
                (&[0, 0, 1], 1.0),
                (&[0, 0, -1], 1.0),
            ]
        };

        for &(offset, cost) in directions {
            neighbors.push((
                Node {
                    x: self.x + offset[0],
                    y: self.y + offset[1],
                    z: self.z + offset[2],
                },
                cost,
            ));
        }

        neighbors
    }
}

#[derive(Debug)]
pub struct PathfindingEngine {
    nav_mesh: NavigationMesh,
    max_iterations: usize,
}

impl PathfindingEngine {
    pub fn new(nav_mesh: NavigationMesh) -> Self {
        Self {
            nav_mesh,
            max_iterations: 10_000,
        }
    }

    pub fn with_max_iterations(mut self, max: usize) -> Self {
        self.max_iterations = max;
        self
    }

    #[instrument(skip(self, start, goal, options), fields(start = %start.hash_key(), goal = %goal.hash_key()))]
    pub fn find_path(
        &self,
        start: &Position3D,
        goal: &Position3D,
        options: &PathfindOptions,
    ) -> PathResult {
        let timer = std::time::Instant::now();

        if start.world != goal.world {
            return PathResult {
                path: vec![],
                cost: f64::INFINITY,
                computed_in_ms: timer.elapsed().as_millis(),
                cached: None,
            };
        }

        let start_node = Node::from_position(start);
        let goal_node = Node::from_position(goal);

        if start_node == goal_node {
            return PathResult {
                path: vec![start.clone()],
                cost: 0.0,
                computed_in_ms: timer.elapsed().as_millis(),
                cached: None,
            };
        }

        let max_iterations = options.max_iterations.unwrap_or(self.max_iterations);
        let result = self.astar(
            &start_node,
            &goal_node,
            options,
            max_iterations,
            &start.world,
        );

        PathResult {
            path: result.path,
            cost: result.cost,
            computed_in_ms: timer.elapsed().as_millis(),
            cached: None,
        }
    }

    fn astar(
        &self,
        start: &Node,
        goal: &Node,
        options: &PathfindOptions,
        max_iterations: usize,
        world: &str,
    ) -> AStarResult {
        let mut open_set: DoublePriorityQueue<Node, ReverseOrderedF64> = DoublePriorityQueue::new();
        let mut came_from: HashMap<Node, Node> = HashMap::new();
        let mut g_score: HashMap<Node, f64> = HashMap::new();
        let mut closed_set: HashSet<Node> = HashSet::new();

        let h = start.heuristic(goal);
        g_score.insert(start.clone(), 0.0);
        open_set.push(start.clone(), ReverseOrderedF64(h));

        let mut iterations = 0;

        while let Some((current, _)) = open_set.pop_min() {
            iterations += 1;

            if iterations > max_iterations {
                debug!("Pathfinding exceeded max iterations ({})", max_iterations);
                return self.reconstruct_partial(&came_from, &current, start, world);
            }

            if current == *goal {
                return self.reconstruct_path(&came_from, &current, start, world);
            }

            closed_set.insert(current.clone());

            for (neighbor, move_cost) in current.neighbors(options.allow_diagonal) {
                if closed_set.contains(&neighbor) {
                    continue;
                }

                if !self
                    .nav_mesh
                    .is_walkable(neighbor.x, neighbor.y, neighbor.z, options)
                {
                    continue;
                }

                let tentative_g =
                    g_score.get(&current).copied().unwrap_or(f64::INFINITY) + move_cost;

                let current_g = g_score.get(&neighbor).copied().unwrap_or(f64::INFINITY);

                if tentative_g < current_g {
                    came_from.insert(neighbor.clone(), current.clone());
                    g_score.insert(neighbor.clone(), tentative_g);

                    let f = tentative_g + neighbor.heuristic(goal);
                    open_set.push(neighbor, ReverseOrderedF64(f));
                }
            }
        }

        debug!("No path found after {} iterations", iterations);

        AStarResult {
            path: vec![start.to_position(world)],
            cost: f64::INFINITY,
        }
    }

    fn reconstruct_path(
        &self,
        came_from: &HashMap<Node, Node>,
        current: &Node,
        start: &Node,
        world: &str,
    ) -> AStarResult {
        let mut path = vec![current.to_position(world)];
        let mut node = current.clone();

        while let Some(prev) = came_from.get(&node) {
            if prev == start {
                break;
            }
            path.push(prev.to_position(world));
            node = prev.clone();
        }

        path.push(start.to_position(world));
        path.reverse();

        let cost = path.windows(2).map(|w| w[0].distance_to(&w[1])).sum();

        AStarResult { path, cost }
    }

    fn reconstruct_partial(
        &self,
        came_from: &HashMap<Node, Node>,
        closest: &Node,
        start: &Node,
        world: &str,
    ) -> AStarResult {
        let mut path = vec![closest.to_position(world)];
        let mut node = closest.clone();

        while let Some(prev) = came_from.get(&node) {
            if prev == start {
                break;
            }
            path.push(prev.to_position(world));
            node = prev.clone();
        }

        path.push(start.to_position(world));
        path.reverse();

        AStarResult {
            path,
            cost: f64::INFINITY,
        }
    }
}

struct AStarResult {
    path: Vec<Position3D>,
    cost: f64,
}

#[derive(Debug, Clone, PartialEq)]
struct ReverseOrderedF64(f64);

impl Eq for ReverseOrderedF64 {}

impl PartialOrd for ReverseOrderedF64 {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        other.0.partial_cmp(&self.0)
    }
}

impl Ord for ReverseOrderedF64 {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap_or(Ordering::Equal)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use npc_types::Position3D;

    fn default_nav_mesh() -> NavigationMesh {
        NavigationMesh::new_flat(100, 64)
    }

    #[test]
    fn test_straight_line_path() {
        let engine = PathfindingEngine::new(default_nav_mesh());
        let start = Position3D::new(0.0, 64.0, 0.0, "overworld".to_string());
        let goal = Position3D::new(5.0, 64.0, 0.0, "overworld".to_string());
        let options = PathfindOptions::default();

        let result = engine.find_path(&start, &goal, &options);

        assert!(!result.is_empty());
        assert!(result.cost > 0.0);
        assert!(result.computed_in_ms < 10000);
    }

    #[test]
    fn test_long_distance_path() {
        let engine = PathfindingEngine::new(NavigationMesh::new_flat(200, 64));
        let start = Position3D::new(0.0, 64.0, 0.0, "overworld".to_string());
        let goal = Position3D::new(20.0, 64.0, 20.0, "overworld".to_string());
        let options = PathfindOptions::default();

        let result = engine.find_path(&start, &goal, &options);

        assert!(!result.is_empty());
    }

    #[test]
    fn test_different_worlds() {
        let engine = PathfindingEngine::new(default_nav_mesh());
        let start = Position3D::new(0.0, 64.0, 0.0, "overworld".to_string());
        let goal = Position3D::new(5.0, 64.0, 0.0, "nether".to_string());
        let options = PathfindOptions::default();

        let result = engine.find_path(&start, &goal, &options);

        assert!(result.is_empty());
        assert!(result.cost.is_infinite());
    }

    #[test]
    fn test_same_position() {
        let engine = PathfindingEngine::new(default_nav_mesh());
        let pos = Position3D::new(5.0, 64.0, 5.0, "overworld".to_string());
        let options = PathfindOptions::default();

        let result = engine.find_path(&pos, &pos, &options);

        assert!(!result.is_empty());
        assert_eq!(result.path.len(), 1);
    }
}
