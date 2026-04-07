use crate::{ActionCategory, ActionSystemConfig, CustomAction};

pub trait AiContextBuilder {
    fn build_context(&mut self) -> String;
}

pub struct AiContextTextBuilder {
    sections: Vec<String>,
}

impl AiContextTextBuilder {
    pub fn new() -> Self {
        Self {
            sections: Vec::new(),
        }
    }

    pub fn add_section(&mut self, title: &str, content: &str) -> &mut Self {
        if !content.is_empty() {
            self.sections.push(format!("[{}]\n{}", title, content));
        }
        self
    }

    pub fn add_action_categories(&mut self, actions: &[ActionCategory]) -> &mut Self {
        if actions.is_empty() {
            return self;
        }

        let actions_str: Vec<String> = actions
            .iter()
            .map(|a| format!("- {}: {}", a.as_str(), a.description()))
            .collect();

        self.sections.push(format!(
            "[ENABLED ACTION CATEGORIES]\n{}",
            actions_str.join("\n")
        ));
        self
    }

    pub fn add_custom_actions(&mut self, actions: &[CustomAction]) -> &mut Self {
        if actions.is_empty() {
            return self;
        }

        let customs: Vec<String> = actions
            .iter()
            .map(|a| {
                let mut s = format!("- {}: {}", a.name, a.description);
                if let Some(ref anim) = a.animation {
                    s.push_str(&format!(" (animation: {})", anim));
                }
                if let Some(cd) = a.cooldown_ms {
                    s.push_str(&format!(" (cooldown: {}ms)", cd));
                }
                if a.target_required {
                    s.push_str(" (requires target)");
                }
                s
            })
            .collect();

        self.sections
            .push(format!("[CUSTOM ACTIONS]\n{}", customs.join("\n")));
        self
    }

    pub fn build(self) -> String {
        self.sections.join("\n\n")
    }
}

impl Default for AiContextTextBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl AiContextBuilder for ActionSystemConfig {
    fn build_context(&mut self) -> String {
        if let Some(ref context) = self.cached_ai_context {
            return context.clone();
        }

        let mut builder = AiContextTextBuilder::new();

        builder
            .add_section("INTRODUCTION", &self.introduction)
            .add_section("ACTIONS", &self.action_explain)
            .add_section("MOVEMENT", &self.movement_explain)
            .add_action_categories(&self.enabled_actions)
            .add_custom_actions(&self.custom_actions);

        let context = builder.build();
        self.cached_ai_context = Some(context.clone());
        context
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::GameGenre;
    use std::collections::HashMap;

    #[test]
    fn test_text_builder_basic() {
        let mut builder = AiContextTextBuilder::new();
        builder
            .add_section("TEST", "This is a test")
            .add_section("ANOTHER", "Another section");

        let result = builder.build();
        assert!(result.contains("[TEST]"));
        assert!(result.contains("[ANOTHER]"));
    }

    #[test]
    fn test_text_builder_actions() {
        let mut builder = AiContextTextBuilder::new();
        builder.add_action_categories(&[ActionCategory::Combat, ActionCategory::Magic]);

        let result = builder.build();
        assert!(result.contains("[ENABLED ACTION CATEGORIES]"));
        assert!(result.contains("combat"));
        assert!(result.contains("magic"));
    }

    #[test]
    fn test_text_builder_custom_actions() {
        let custom = CustomAction {
            name: "fireball".into(),
            description: "Cast a fireball".into(),
            animation: Some("cast_spell".into()),
            cooldown_ms: Some(3000),
            target_required: true,
            parameters: HashMap::new(),
        };

        let mut builder = AiContextTextBuilder::new();
        builder.add_custom_actions(&[custom]);

        let result = builder.build();
        assert!(result.contains("[CUSTOM ACTIONS]"));
        assert!(result.contains("fireball"));
        assert!(result.contains("animation: cast_spell"));
        assert!(result.contains("cooldown: 3000ms"));
        assert!(result.contains("requires target"));
    }

    #[test]
    fn test_action_system_config_builder_trait() {
        let mut config = ActionSystemConfig::new(
            GameGenre::Rpg,
            vec![ActionCategory::Combat],
            vec![],
            "I am a warrior".into(),
            "I fight".into(),
            "I walk".into(),
        );

        let context = config.build_context();
        assert!(context.contains("[INTRODUCTION]"));
        assert!(context.contains("[ACTIONS]"));
        assert!(context.contains("[MOVEMENT]"));

        // Test caching
        let context2 = config.build_context();
        assert_eq!(context, context2);
    }
}
