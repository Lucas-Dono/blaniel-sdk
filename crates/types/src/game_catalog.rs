use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use crate::{ActionCondition, ActionPriority};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GameGenre {
    Rpg,
    Fighting,
    Magic,
    Adventure,
    Romance,
    Simulation,
    VisualNovel,
    Survival,
    Horror,
    Sandbox,
    Strategy,
    Sports,
    Puzzle,
    Platformer,
    Shooter,
    Stealth,
    Racing,
    Rhythm,
    #[serde(untagged)]
    Custom(String),
}

impl GameGenre {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Rpg => "rpg",
            Self::Fighting => "fighting",
            Self::Magic => "magic",
            Self::Adventure => "adventure",
            Self::Romance => "romance",
            Self::Simulation => "simulation",
            Self::VisualNovel => "visual_novel",
            Self::Survival => "survival",
            Self::Horror => "horror",
            Self::Sandbox => "sandbox",
            Self::Strategy => "strategy",
            Self::Sports => "sports",
            Self::Puzzle => "puzzle",
            Self::Platformer => "platformer",
            Self::Shooter => "shooter",
            Self::Stealth => "stealth",
            Self::Racing => "racing",
            Self::Rhythm => "rhythm",
            Self::Custom(s) => s.as_str(),
        }
    }

    pub fn default_actions(&self) -> Vec<ActionCategory> {
        match self {
            Self::Rpg => vec![
                ActionCategory::Combat,
                ActionCategory::Magic,
                ActionCategory::Trading,
                ActionCategory::Social,
                ActionCategory::Exploration,
                ActionCategory::Quest,
                ActionCategory::Crafting,
            ],
            Self::Fighting => vec![
                ActionCategory::Combat,
                ActionCategory::Emotes,
                ActionCategory::Social,
            ],
            Self::Magic => vec![
                ActionCategory::Magic,
                ActionCategory::Combat,
                ActionCategory::Healing,
                ActionCategory::Social,
            ],
            Self::Adventure => vec![
                ActionCategory::Exploration,
                ActionCategory::Interaction,
                ActionCategory::Social,
                ActionCategory::Quest,
                ActionCategory::Navigation,
            ],
            Self::Romance => vec![
                ActionCategory::Social,
                ActionCategory::Emotes,
                ActionCategory::Dialogue,
                ActionCategory::Navigation,
            ],
            Self::Simulation => vec![
                ActionCategory::Farming,
                ActionCategory::Building,
                ActionCategory::Trading,
                ActionCategory::Social,
                ActionCategory::Crafting,
                ActionCategory::Cooking,
                ActionCategory::Navigation,
            ],
            Self::VisualNovel => vec![
                ActionCategory::Dialogue,
                ActionCategory::Social,
                ActionCategory::Emotes,
                ActionCategory::Navigation,
            ],
            Self::Survival => vec![
                ActionCategory::Combat,
                ActionCategory::Crafting,
                ActionCategory::Building,
                ActionCategory::Farming,
                ActionCategory::Fishing,
                ActionCategory::Mining,
                ActionCategory::Cooking,
                ActionCategory::Exploration,
            ],
            Self::Horror => vec![
                ActionCategory::Exploration,
                ActionCategory::Stealth,
                ActionCategory::Interaction,
                ActionCategory::Social,
            ],
            Self::Sandbox => vec![
                ActionCategory::Building,
                ActionCategory::Crafting,
                ActionCategory::Mining,
                ActionCategory::Exploration,
                ActionCategory::Trading,
                ActionCategory::Navigation,
            ],
            Self::Strategy => vec![
                ActionCategory::Combat,
                ActionCategory::Trading,
                ActionCategory::Building,
                ActionCategory::Exploration,
            ],
            Self::Sports => vec![
                ActionCategory::Emotes,
                ActionCategory::Social,
                ActionCategory::Navigation,
            ],
            Self::Puzzle => vec![
                ActionCategory::Interaction,
                ActionCategory::Exploration,
                ActionCategory::Dialogue,
            ],
            Self::Platformer => vec![
                ActionCategory::Navigation,
                ActionCategory::Combat,
                ActionCategory::Exploration,
            ],
            Self::Shooter => vec![
                ActionCategory::Combat,
                ActionCategory::Stealth,
                ActionCategory::Navigation,
            ],
            Self::Stealth => vec![
                ActionCategory::Stealth,
                ActionCategory::Combat,
                ActionCategory::Interaction,
                ActionCategory::Navigation,
            ],
            Self::Racing => vec![
                ActionCategory::Navigation,
                ActionCategory::Emotes,
                ActionCategory::Social,
            ],
            Self::Rhythm => vec![
                ActionCategory::Music,
                ActionCategory::Emotes,
                ActionCategory::Social,
            ],
            Self::Custom(_) => vec![],
        }
    }

    pub fn all_genres() -> Vec<&'static str> {
        vec![
            "rpg",
            "fighting",
            "magic",
            "adventure",
            "romance",
            "simulation",
            "visual_novel",
            "survival",
            "horror",
            "sandbox",
            "strategy",
            "sports",
            "puzzle",
            "platformer",
            "shooter",
            "stealth",
            "racing",
            "rhythm",
        ]
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionCategory {
    Combat,
    Magic,
    Crafting,
    Trading,
    Social,
    Exploration,
    Stealth,
    Farming,
    Fishing,
    Mining,
    Building,
    Cooking,
    Healing,
    Music,
    Emotes,
    Navigation,
    Interaction,
    Dialogue,
    Quest,
}

impl ActionCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Combat => "combat",
            Self::Magic => "magic",
            Self::Crafting => "crafting",
            Self::Trading => "trading",
            Self::Social => "social",
            Self::Exploration => "exploration",
            Self::Stealth => "stealth",
            Self::Farming => "farming",
            Self::Fishing => "fishing",
            Self::Mining => "mining",
            Self::Building => "building",
            Self::Cooking => "cooking",
            Self::Healing => "healing",
            Self::Music => "music",
            Self::Emotes => "emotes",
            Self::Navigation => "navigation",
            Self::Interaction => "interaction",
            Self::Dialogue => "dialogue",
            Self::Quest => "quest",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::Combat => "Attack, defend, dodge, use weapons",
            Self::Magic => "Cast spells, control elements, enchant items",
            Self::Crafting => "Create items, repair equipment, combine materials",
            Self::Trading => "Buy, sell, barter, manage inventory",
            Self::Social => "Talk, befriend, build relationships, persuade",
            Self::Exploration => "Discover locations, map terrain, scout areas",
            Self::Stealth => "Sneak, hide, pickpocket, infiltrate",
            Self::Farming => "Plant, harvest, breed animals, tend crops",
            Self::Fishing => "Fish, collect aquatic resources",
            Self::Mining => "Extract minerals, gather stone, dig tunnels",
            Self::Building => "Construct structures, place blocks, design rooms",
            Self::Cooking => "Prepare food, mix ingredients, create recipes",
            Self::Healing => "Heal wounds, cure ailments, apply buffs",
            Self::Music => "Play instruments, sing, compose melodies",
            Self::Emotes => "Express emotions, gesture, perform animations",
            Self::Navigation => "Move, travel, teleport between locations",
            Self::Interaction => "Use objects, activate mechanisms, manipulate environment",
            Self::Dialogue => "Complex dialogue trees, branching conversations",
            Self::Quest => "Give, accept, track, complete quests",
        }
    }

    pub fn all_categories() -> Vec<Self> {
        vec![
            Self::Combat,
            Self::Magic,
            Self::Crafting,
            Self::Trading,
            Self::Social,
            Self::Exploration,
            Self::Stealth,
            Self::Farming,
            Self::Fishing,
            Self::Mining,
            Self::Building,
            Self::Cooking,
            Self::Healing,
            Self::Music,
            Self::Emotes,
            Self::Navigation,
            Self::Interaction,
            Self::Dialogue,
            Self::Quest,
        ]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomAction {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub animation: Option<String>,
    #[serde(default)]
    pub cooldown_ms: Option<u64>,
    #[serde(default)]
    pub target_required: bool,
    #[serde(default)]
    pub priority: ActionPriority,
    #[serde(default)]
    pub conditions: Vec<ActionCondition>,
    #[serde(default)]
    pub parameters: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct ActionSystemConfig {
    pub game_genre: GameGenre,
    pub enabled_actions: Vec<ActionCategory>,
    enabled_actions_set: HashSet<ActionCategory>,
    pub custom_actions: Vec<CustomAction>,
    custom_actions_map: HashMap<String, CustomAction>,
    pub introduction: String,
    pub action_explain: String,
    pub movement_explain: String,
    pub(crate) cached_ai_context: Option<String>,
}

impl Serialize for ActionSystemConfig {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("ActionSystemConfig", 6)?;
        state.serialize_field("game_genre", &self.game_genre)?;
        state.serialize_field("enabled_actions", &self.enabled_actions)?;
        state.serialize_field("custom_actions", &self.custom_actions)?;
        state.serialize_field("introduction", &self.introduction)?;
        state.serialize_field("action_explain", &self.action_explain)?;
        state.serialize_field("movement_explain", &self.movement_explain)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for ActionSystemConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct ActionSystemConfigHelper {
            game_genre: GameGenre,
            #[serde(default)]
            enabled_actions: Vec<ActionCategory>,
            #[serde(default)]
            custom_actions: Vec<CustomAction>,
            introduction: String,
            action_explain: String,
            movement_explain: String,
        }

        let helper = ActionSystemConfigHelper::deserialize(deserializer)?;
        Ok(ActionSystemConfig::new(
            helper.game_genre,
            helper.enabled_actions,
            helper.custom_actions,
            helper.introduction,
            helper.action_explain,
            helper.movement_explain,
        ))
    }
}

impl ActionSystemConfig {
    pub fn new(
        game_genre: GameGenre,
        enabled_actions: Vec<ActionCategory>,
        custom_actions: Vec<CustomAction>,
        introduction: String,
        action_explain: String,
        movement_explain: String,
    ) -> Self {
        let enabled_actions_set = enabled_actions.iter().cloned().collect();
        let custom_actions_map = custom_actions
            .iter()
            .map(|a| (a.name.clone(), a.clone()))
            .collect();

        Self {
            game_genre,
            enabled_actions,
            enabled_actions_set,
            custom_actions,
            custom_actions_map,
            introduction,
            action_explain,
            movement_explain,
            cached_ai_context: None,
        }
    }

    pub fn invalidate_cache(&mut self) {
        self.cached_ai_context = None;
    }

    pub fn is_action_enabled(&self, category: &ActionCategory) -> bool {
        self.enabled_actions_set.contains(category)
    }

    pub fn effective_actions(&self) -> Vec<&ActionCategory> {
        self.enabled_actions.iter().collect()
    }

    pub fn find_custom_action(&self, name: &str) -> Option<&CustomAction> {
        self.custom_actions_map.get(name)
    }

    // Deprecated: Use AiContextBuilder trait instead
    pub fn build_ai_context(&mut self) -> String {
        use crate::AiContextBuilder;
        self.build_context()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetActionSystemConfigRequest {
    pub game_genre: GameGenre,
    #[serde(default)]
    pub enabled_actions: Vec<ActionCategory>,
    #[serde(default)]
    pub custom_actions: Vec<CustomAction>,
    pub introduction: String,
    pub action_explain: String,
    pub movement_explain: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameGenreInfo {
    pub genre: String,
    pub default_actions: Vec<String>,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameCatalogResponse {
    pub genres: Vec<GameGenreInfo>,
    pub total: usize,
}

impl GameCatalogResponse {
    pub fn build_catalog() -> Self {
        static CATALOG: OnceLock<GameCatalogResponse> = OnceLock::new();
        CATALOG.get_or_init(Self::build_catalog_internal).clone()
    }

    fn build_catalog_internal() -> Self {
        let genres: Vec<GameGenreInfo> = vec![
            GameGenreInfo {
                genre: "rpg".into(),
                default_actions: vec![
                    "combat".into(),
                    "magic".into(),
                    "trading".into(),
                    "social".into(),
                    "exploration".into(),
                    "quest".into(),
                    "crafting".into(),
                ],
                description: "Role-playing game with combat, quests, and character progression"
                    .into(),
            },
            GameGenreInfo {
                genre: "fighting".into(),
                default_actions: vec!["combat".into(), "emotes".into(), "social".into()],
                description: "Fighting game focused on combat mechanics".into(),
            },
            GameGenreInfo {
                genre: "magic".into(),
                default_actions: vec![
                    "magic".into(),
                    "combat".into(),
                    "healing".into(),
                    "social".into(),
                ],
                description: "Magic-focused game with spells and enchantments".into(),
            },
            GameGenreInfo {
                genre: "adventure".into(),
                default_actions: vec![
                    "exploration".into(),
                    "interaction".into(),
                    "social".into(),
                    "quest".into(),
                    "navigation".into(),
                ],
                description: "Adventure game with exploration and puzzles".into(),
            },
            GameGenreInfo {
                genre: "romance".into(),
                default_actions: vec![
                    "social".into(),
                    "emotes".into(),
                    "dialogue".into(),
                    "navigation".into(),
                ],
                description: "Romance-focused game with relationship building".into(),
            },
            GameGenreInfo {
                genre: "simulation".into(),
                default_actions: vec![
                    "farming".into(),
                    "building".into(),
                    "trading".into(),
                    "social".into(),
                    "crafting".into(),
                    "cooking".into(),
                    "navigation".into(),
                ],
                description: "Life simulation with farming, building, and social activities".into(),
            },
            GameGenreInfo {
                genre: "visual_novel".into(),
                default_actions: vec![
                    "dialogue".into(),
                    "social".into(),
                    "emotes".into(),
                    "navigation".into(),
                ],
                description: "Story-driven visual novel with dialogue choices".into(),
            },
            GameGenreInfo {
                genre: "survival".into(),
                default_actions: vec![
                    "combat".into(),
                    "crafting".into(),
                    "building".into(),
                    "farming".into(),
                    "fishing".into(),
                    "mining".into(),
                    "cooking".into(),
                    "exploration".into(),
                ],
                description: "Survival game with resource gathering and crafting".into(),
            },
            GameGenreInfo {
                genre: "horror".into(),
                default_actions: vec![
                    "exploration".into(),
                    "stealth".into(),
                    "interaction".into(),
                    "social".into(),
                ],
                description: "Horror game with exploration and stealth".into(),
            },
            GameGenreInfo {
                genre: "sandbox".into(),
                default_actions: vec![
                    "building".into(),
                    "crafting".into(),
                    "mining".into(),
                    "exploration".into(),
                    "trading".into(),
                    "navigation".into(),
                ],
                description: "Sandbox game with creative freedom".into(),
            },
            GameGenreInfo {
                genre: "strategy".into(),
                default_actions: vec![
                    "combat".into(),
                    "trading".into(),
                    "building".into(),
                    "exploration".into(),
                ],
                description: "Strategy game with resource management".into(),
            },
            GameGenreInfo {
                genre: "sports".into(),
                default_actions: vec!["emotes".into(), "social".into(), "navigation".into()],
                description: "Sports game with competitive activities".into(),
            },
            GameGenreInfo {
                genre: "puzzle".into(),
                default_actions: vec![
                    "interaction".into(),
                    "exploration".into(),
                    "dialogue".into(),
                ],
                description: "Puzzle game with problem-solving mechanics".into(),
            },
            GameGenreInfo {
                genre: "platformer".into(),
                default_actions: vec!["navigation".into(), "combat".into(), "exploration".into()],
                description: "Platformer with movement and exploration".into(),
            },
            GameGenreInfo {
                genre: "shooter".into(),
                default_actions: vec!["combat".into(), "stealth".into(), "navigation".into()],
                description: "Shooter game with combat and tactical movement".into(),
            },
            GameGenreInfo {
                genre: "stealth".into(),
                default_actions: vec![
                    "stealth".into(),
                    "combat".into(),
                    "interaction".into(),
                    "navigation".into(),
                ],
                description: "Stealth game with sneaking and infiltration".into(),
            },
            GameGenreInfo {
                genre: "racing".into(),
                default_actions: vec!["navigation".into(), "emotes".into(), "social".into()],
                description: "Racing game with fast movement".into(),
            },
            GameGenreInfo {
                genre: "rhythm".into(),
                default_actions: vec!["music".into(), "emotes".into(), "social".into()],
                description: "Rhythm game with music and performance".into(),
            },
        ];

        let total = genres.len();
        Self { genres, total }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_genre_default_actions() {
        let rpg = GameGenre::Rpg;
        assert!(!rpg.default_actions().is_empty());
        assert!(rpg.default_actions().contains(&ActionCategory::Combat));

        let custom = GameGenre::Custom("tower_defense".into());
        assert!(custom.default_actions().is_empty());
    }

    #[test]
    fn test_action_system_config_build_context() {
        let config = ActionSystemConfig {
            game_genre: GameGenre::Rpg,
            enabled_actions: vec![ActionCategory::Combat, ActionCategory::Magic],
            custom_actions: vec![CustomAction {
                name: "fireball".into(),
                description: "Launches a fireball at the target".into(),
                animation: Some("cast_fire".into()),
                cooldown_ms: Some(3000),
                target_required: true,
                parameters: HashMap::new(),
            }],
            introduction: "You are a mage in the tower of Arcanum.".into(),
            action_explain: "You can cast fire and ice spells. You are in a peaceful world but can defend yourself.".into(),
            movement_explain: "You can walk around the tower and teleport between floors.".into(),
        };

        let context = config.build_ai_context();
        assert!(context.contains("[INTRODUCTION]"));
        assert!(context.contains("[ACTIONS]"));
        assert!(context.contains("[MOVEMENT]"));
        assert!(context.contains("[ENABLED ACTION CATEGORIES]"));
        assert!(context.contains("[CUSTOM ACTIONS]"));
        assert!(context.contains("fireball"));
    }

    #[test]
    fn test_is_action_enabled() {
        let config = ActionSystemConfig {
            game_genre: GameGenre::Fighting,
            enabled_actions: vec![ActionCategory::Combat],
            custom_actions: vec![],
            introduction: "Test".into(),
            action_explain: "Test".into(),
            movement_explain: "Test".into(),
        };

        assert!(config.is_action_enabled(&ActionCategory::Combat));
        assert!(!config.is_action_enabled(&ActionCategory::Magic));
    }

    #[test]
    fn test_catalog_response() {
        let catalog = GameCatalogResponse::build_catalog();
        assert_eq!(catalog.total, 18);
        assert!(catalog.genres.iter().any(|g| g.genre == "rpg"));
    }

    #[test]
    fn test_custom_action_find() {
        let config = ActionSystemConfig {
            game_genre: GameGenre::Rpg,
            enabled_actions: vec![],
            custom_actions: vec![CustomAction {
                name: "summon_dragon".into(),
                description: "Summons a dragon companion".into(),
                animation: None,
                cooldown_ms: Some(10000),
                target_required: false,
                parameters: HashMap::new(),
            }],
            introduction: "Test".into(),
            action_explain: "Test".into(),
            movement_explain: "Test".into(),
        };

        assert!(config.find_custom_action("summon_dragon").is_some());
        assert!(config.find_custom_action("nonexistent").is_none());
    }

    #[test]
    fn test_single_action_config() {
        let config = ActionSystemConfig {
            game_genre: GameGenre::Rpg,
            enabled_actions: vec![ActionCategory::Emotes],
            custom_actions: vec![],
            introduction: "A peaceful world.".into(),
            action_explain: "You can only express emotions. No combat allowed.".into(),
            movement_explain: "You can walk but not run.".into(),
        };

        assert_eq!(config.enabled_actions.len(), 1);
        assert!(config.is_action_enabled(&ActionCategory::Emotes));
    }
}
