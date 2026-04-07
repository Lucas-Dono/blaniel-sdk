use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionDependency {
    pub action_id: String,
    #[serde(default)]
    pub required_state: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependentAction {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub dependencies: Vec<ActionDependency>,
    #[serde(default)]
    pub provides_state: Option<String>,
}

pub struct ActionDependencyResolver;

impl ActionDependencyResolver {
    pub fn resolve_dependencies(
        actions: &[DependentAction],
        target_action: &str,
    ) -> Result<Vec<String>, String> {
        let action_map: HashMap<_, _> = actions
            .iter()
            .map(|a| (a.name.as_str(), a))
            .collect();

        let _target = action_map
            .get(target_action)
            .ok_or_else(|| format!("Action '{}' not found", target_action))?;

        let mut resolved = Vec::new();
        let mut visiting = HashSet::new();
        let mut visited = HashSet::new();

        Self::resolve_recursive(
            target_action,
            &action_map,
            &mut resolved,
            &mut visiting,
            &mut visited,
        )?;

        resolved.reverse();
        Ok(resolved)
    }

    fn resolve_recursive<'a>(
        action_name: &'a str,
        action_map: &HashMap<&'a str, &'a DependentAction>,
        resolved: &mut Vec<String>,
        visiting: &mut HashSet<&'a str>,
        visited: &mut HashSet<&'a str>,
    ) -> Result<(), String> {
        if visited.contains(action_name) {
            return Ok(());
        }

        if visiting.contains(action_name) {
            return Err(format!("Circular dependency detected involving '{}'", action_name));
        }

        visiting.insert(action_name);

        if let Some(action) = action_map.get(action_name) {
            for dep in &action.dependencies {
                Self::resolve_recursive(
                    &dep.action_id,
                    action_map,
                    resolved,
                    visiting,
                    visited,
                )?;
            }
        }

        visiting.remove(action_name);
        visited.insert(action_name);
        resolved.push(action_name.to_string());

        Ok(())
    }

    pub fn validate_dependencies(actions: &[DependentAction]) -> Result<(), String> {
        let action_map: HashMap<_, _> = actions
            .iter()
            .map(|a| (a.name.as_str(), a))
            .collect();

        // Check that all dependencies exist
        for action in actions {
            for dep in &action.dependencies {
                if !action_map.contains_key(dep.action_id.as_str()) {
                    return Err(format!(
                        "Action '{}' has dependency on non-existent action '{}'",
                        action.name, dep.action_id
                    ));
                }
            }
        }

        // Check for circular dependencies
        for action in actions {
            let mut visiting = HashSet::new();
            let mut visited = HashSet::new();
            let mut resolved = Vec::new();

            Self::resolve_recursive(
                &action.name,
                &action_map,
                &mut resolved,
                &mut visiting,
                &mut visited,
            )?;
        }

        Ok(())
    }

    pub fn build_dependency_graph(
        actions: &[DependentAction],
    ) -> HashMap<String, Vec<String>> {
        let mut graph = HashMap::new();

        for action in actions {
            let deps: Vec<String> = action
                .dependencies
                .iter()
                .map(|d| d.action_id.clone())
                .collect();
            graph.insert(action.name.clone(), deps);
        }

        graph
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_dependency_resolution() {
        let actions = vec![
            DependentAction {
                name: "get_ingredients".into(),
                description: "Get cooking ingredients".into(),
                dependencies: vec![],
                provides_state: Some("has_ingredients".into()),
            },
            DependentAction {
                name: "cook".into(),
                description: "Cook the meal".into(),
                dependencies: vec![ActionDependency {
                    action_id: "get_ingredients".into(),
                    required_state: Some("has_ingredients".into()),
                }],
                provides_state: Some("meal_cooked".into()),
            },
        ];

        let resolved = ActionDependencyResolver::resolve_dependencies(&actions, "cook").unwrap();

        assert_eq!(resolved.len(), 2);
        assert_eq!(resolved[0], "get_ingredients");
        assert_eq!(resolved[1], "cook");
    }

    #[test]
    fn test_chain_dependencies() {
        let actions = vec![
            DependentAction {
                name: "a".into(),
                description: "First".into(),
                dependencies: vec![],
                provides_state: None,
            },
            DependentAction {
                name: "b".into(),
                description: "Second".into(),
                dependencies: vec![ActionDependency {
                    action_id: "a".into(),
                    required_state: None,
                }],
                provides_state: None,
            },
            DependentAction {
                name: "c".into(),
                description: "Third".into(),
                dependencies: vec![ActionDependency {
                    action_id: "b".into(),
                    required_state: None,
                }],
                provides_state: None,
            },
        ];

        let resolved = ActionDependencyResolver::resolve_dependencies(&actions, "c").unwrap();

        assert_eq!(resolved, vec!["a", "b", "c"]);
    }

    #[test]
    fn test_circular_dependency_detection() {
        let actions = vec![
            DependentAction {
                name: "a".into(),
                description: "First".into(),
                dependencies: vec![ActionDependency {
                    action_id: "b".into(),
                    required_state: None,
                }],
                provides_state: None,
            },
            DependentAction {
                name: "b".into(),
                description: "Second".into(),
                dependencies: vec![ActionDependency {
                    action_id: "a".into(),
                    required_state: None,
                }],
                provides_state: None,
            },
        ];

        let result = ActionDependencyResolver::validate_dependencies(&actions);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Circular dependency"));
    }

    #[test]
    fn test_missing_dependency() {
        let actions = vec![DependentAction {
            name: "cook".into(),
            description: "Cook".into(),
            dependencies: vec![ActionDependency {
                action_id: "nonexistent".into(),
                required_state: None,
            }],
            provides_state: None,
        }];

        let result = ActionDependencyResolver::validate_dependencies(&actions);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("non-existent"));
    }
}
