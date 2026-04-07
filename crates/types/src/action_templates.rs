use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::{ActionPlan, AutonomousAction, ActionType};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionTemplate {
    pub id: String,
    pub name: String,
    pub description: String,
    pub actions: Vec<AutonomousAction>,
    #[serde(default)]
    pub loop_template: bool,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub category: Option<String>,
}

impl ActionTemplate {
    pub fn instantiate(&self) -> ActionPlan {
        ActionPlan {
            actions: self.actions.clone(),
            loop_plan: self.loop_template,
            interrupt_on_chat: true,
        }
    }
}

pub struct ActionTemplateLibrary {
    templates: HashMap<String, ActionTemplate>,
}

impl ActionTemplateLibrary {
    pub fn new() -> Self {
        let mut library = Self {
            templates: HashMap::new(),
        };
        library.load_defaults();
        library
    }

    fn load_defaults(&mut self) {
        // Morning Routine Template
        self.add_template(ActionTemplate {
            id: "morning_routine".into(),
            name: "Morning Routine".into(),
            description: "Wake up, stretch, and start the day".into(),
            actions: vec![
                AutonomousAction {
                    action_type: ActionType::Animate,
                    animation: Some("stretch".into()),
                    duration_ms: Some(2000),
                    ..Default::default()
                },
                AutonomousAction {
                    action_type: ActionType::Speak,
                    target: Some("Good morning!".into()),
                    ..Default::default()
                },
            ],
            loop_template: false,
            tags: vec!["routine".into(), "daily".into()],
            category: Some("daily".into()),
        });

        // Patrol Route Template
        self.add_template(ActionTemplate {
            id: "patrol_route".into(),
            name: "Patrol Route".into(),
            description: "Patrol between multiple waypoints".into(),
            actions: vec![
                AutonomousAction {
                    action_type: ActionType::Navigate,
                    target: Some("waypoint_1".into()),
                    ..Default::default()
                },
                AutonomousAction {
                    action_type: ActionType::Wait,
                    duration_ms: Some(3000),
                    ..Default::default()
                },
                AutonomousAction {
                    action_type: ActionType::Navigate,
                    target: Some("waypoint_2".into()),
                    ..Default::default()
                },
                AutonomousAction {
                    action_type: ActionType::Wait,
                    duration_ms: Some(3000),
                    ..Default::default()
                },
            ],
            loop_template: true,
            tags: vec!["patrol".into(), "navigation".into()],
            category: Some("security".into()),
        });

        // Greeting Template
        self.add_template(ActionTemplate {
            id: "friendly_greeting".into(),
            name: "Friendly Greeting".into(),
            description: "Wave and greet someone warmly".into(),
            actions: vec![
                AutonomousAction {
                    action_type: ActionType::Emote,
                    animation: Some("wave".into()),
                    emotion: Some("happy".into()),
                    duration_ms: Some(1500),
                    ..Default::default()
                },
                AutonomousAction {
                    action_type: ActionType::Speak,
                    target: Some("Hello! Nice to see you!".into()),
                    ..Default::default()
                },
            ],
            loop_template: false,
            tags: vec!["social".into(), "greeting".into()],
            category: Some("social".into()),
        });

        // Idle Animation Loop Template
        self.add_template(ActionTemplate {
            id: "idle_animations".into(),
            name: "Idle Animations".into(),
            description: "Cycle through idle animations".into(),
            actions: vec![
                AutonomousAction {
                    action_type: ActionType::Wait,
                    duration_ms: Some(5000),
                    ..Default::default()
                },
                AutonomousAction {
                    action_type: ActionType::Animate,
                    animation: Some("look_around".into()),
                    duration_ms: Some(2000),
                    ..Default::default()
                },
                AutonomousAction {
                    action_type: ActionType::Wait,
                    duration_ms: Some(3000),
                    ..Default::default()
                },
                AutonomousAction {
                    action_type: ActionType::Animate,
                    animation: Some("stretch".into()),
                    duration_ms: Some(1500),
                    ..Default::default()
                },
            ],
            loop_template: true,
            tags: vec!["idle".into(), "ambient".into()],
            category: Some("ambient".into()),
        });
    }

    pub fn add_template(&mut self, template: ActionTemplate) {
        self.templates.insert(template.id.clone(), template);
    }

    pub fn get_template(&self, id: &str) -> Option<&ActionTemplate> {
        self.templates.get(id)
    }

    pub fn list_templates(&self) -> Vec<&ActionTemplate> {
        self.templates.values().collect()
    }

    pub fn list_by_category(&self, category: &str) -> Vec<&ActionTemplate> {
        self.templates
            .values()
            .filter(|t| t.category.as_deref() == Some(category))
            .collect()
    }

    pub fn list_by_tag(&self, tag: &str) -> Vec<&ActionTemplate> {
        self.templates
            .values()
            .filter(|t| t.tags.contains(&tag.to_string()))
            .collect()
    }

    pub fn instantiate(&self, template_id: &str) -> Option<ActionPlan> {
        self.get_template(template_id).map(|t| t.instantiate())
    }
}

impl Default for ActionTemplateLibrary {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for AutonomousAction {
    fn default() -> Self {
        Self {
            action_type: ActionType::Wait,
            target: None,
            position: None,
            duration_ms: Some(1000),
            animation: None,
            emotion: None,
            delay_ms: None,
            priority: Default::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_library_creation() {
        let library = ActionTemplateLibrary::new();
        assert!(!library.templates.is_empty());
        assert!(library.get_template("morning_routine").is_some());
    }

    #[test]
    fn test_template_instantiation() {
        let library = ActionTemplateLibrary::new();
        let plan = library.instantiate("patrol_route");
        assert!(plan.is_some());

        let plan = plan.unwrap();
        assert!(plan.loop_plan);
        assert_eq!(plan.actions.len(), 4);
    }

    #[test]
    fn test_list_by_category() {
        let library = ActionTemplateLibrary::new();
        let social = library.list_by_category("social");
        assert!(!social.is_empty());
    }

    #[test]
    fn test_list_by_tag() {
        let library = ActionTemplateLibrary::new();
        let routines = library.list_by_tag("routine");
        assert!(!routines.is_empty());
    }
}
