use crate::{ActionCategory, ActionSystemConfig, CustomAction, GameGenre};

#[derive(Debug, Clone, Default)]
pub struct ActionSystemConfigBuilder {
    game_genre: Option<GameGenre>,
    enabled_actions: Vec<ActionCategory>,
    custom_actions: Vec<CustomAction>,
    introduction: String,
    action_explain: String,
    movement_explain: String,
}

impl ActionSystemConfigBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn game_genre(mut self, genre: GameGenre) -> Self {
        self.game_genre = Some(genre);
        self
    }

    pub fn enable_action(mut self, action: ActionCategory) -> Self {
        if !self.enabled_actions.contains(&action) {
            self.enabled_actions.push(action);
        }
        self
    }

    pub fn enable_actions(mut self, actions: Vec<ActionCategory>) -> Self {
        for action in actions {
            if !self.enabled_actions.contains(&action) {
                self.enabled_actions.push(action);
            }
        }
        self
    }

    pub fn enable_default_actions(mut self) -> Self {
        if let Some(ref genre) = self.game_genre {
            let defaults = genre.default_actions();
            for action in defaults {
                if !self.enabled_actions.contains(&action) {
                    self.enabled_actions.push(action);
                }
            }
        }
        self
    }

    pub fn add_custom_action(mut self, action: CustomAction) -> Self {
        self.custom_actions.push(action);
        self
    }

    pub fn add_custom_actions(mut self, actions: Vec<CustomAction>) -> Self {
        self.custom_actions.extend(actions);
        self
    }

    pub fn introduction(mut self, intro: impl Into<String>) -> Self {
        self.introduction = intro.into();
        self
    }

    pub fn action_explain(mut self, explain: impl Into<String>) -> Self {
        self.action_explain = explain.into();
        self
    }

    pub fn movement_explain(mut self, explain: impl Into<String>) -> Self {
        self.movement_explain = explain.into();
        self
    }

    pub fn build(self) -> Result<ActionSystemConfig, String> {
        let genre = self
            .game_genre
            .ok_or_else(|| "game_genre is required".to_string())?;

        if self.introduction.trim().is_empty() {
            return Err("introduction cannot be empty".to_string());
        }

        if self.action_explain.trim().is_empty() {
            return Err("action_explain cannot be empty".to_string());
        }

        if self.movement_explain.trim().is_empty() {
            return Err("movement_explain cannot be empty".to_string());
        }

        Ok(ActionSystemConfig::new(
            genre,
            self.enabled_actions,
            self.custom_actions,
            self.introduction,
            self.action_explain,
            self.movement_explain,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_builder_basic() {
        let config = ActionSystemConfigBuilder::new()
            .game_genre(GameGenre::Rpg)
            .enable_action(ActionCategory::Combat)
            .enable_action(ActionCategory::Magic)
            .introduction("I am a mage")
            .action_explain("I can cast spells")
            .movement_explain("I can walk")
            .build()
            .unwrap();

        assert_eq!(config.enabled_actions.len(), 2);
        assert!(config.is_action_enabled(&ActionCategory::Combat));
    }

    #[test]
    fn test_builder_with_defaults() {
        let config = ActionSystemConfigBuilder::new()
            .game_genre(GameGenre::Rpg)
            .enable_default_actions()
            .introduction("I am an adventurer")
            .action_explain("I can do many things")
            .movement_explain("I move freely")
            .build()
            .unwrap();

        assert!(!config.enabled_actions.is_empty());
        assert!(config.is_action_enabled(&ActionCategory::Combat));
        assert!(config.is_action_enabled(&ActionCategory::Magic));
    }

    #[test]
    fn test_builder_with_custom_actions() {
        let custom = CustomAction {
            name: "fireball".into(),
            description: "Cast a fireball".into(),
            animation: Some("cast_spell".into()),
            cooldown_ms: Some(3000),
            target_required: true,
            parameters: HashMap::new(),
        };

        let config = ActionSystemConfigBuilder::new()
            .game_genre(GameGenre::Magic)
            .enable_action(ActionCategory::Magic)
            .add_custom_action(custom)
            .introduction("I am a fire mage")
            .action_explain("I specialize in fire magic")
            .movement_explain("I teleport")
            .build()
            .unwrap();

        assert!(config.find_custom_action("fireball").is_some());
    }

    #[test]
    fn test_builder_validation_fails() {
        let result = ActionSystemConfigBuilder::new()
            .game_genre(GameGenre::Rpg)
            .build();

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("introduction"));
    }
}
