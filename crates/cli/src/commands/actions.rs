use blaniel_sdk::{BlanielClient, SdkError};

#[derive(clap::Subcommand)]
pub enum ActionCommands {
    Catalog,
    Genre {
        #[arg(help = "Game genre (rpg, fighting, adventure, etc.)")]
        genre: String,
    },
    Get {
        #[arg(help = "NPC/Agent ID")]
        id: String,
    },
    Set {
        #[arg(help = "NPC/Agent ID")]
        id: String,
        #[arg(long, default_value = "rpg")]
        genre: String,
        #[arg(long, num_args = 0.., help = "Enabled actions (comma-separated)")]
        actions: Option<Vec<String>>,
        #[arg(long, help = "Introduction text for the AI")]
        introduction: Option<String>,
        #[arg(long, help = "Action explanation for the AI")]
        action_explain: Option<String>,
        #[arg(long, help = "Movement explanation for the AI")]
        movement_explain: Option<String>,
    },
    Context {
        #[arg(help = "NPC/Agent ID")]
        id: String,
    },
}

pub async fn run(cmd: ActionCommands) -> Result<(), SdkError> {
    let client = BlanielClient::from_env()?;

    match cmd {
        ActionCommands::Catalog => {
            let catalog = client.get_game_catalog().await?;
            println!("\x1b[1mGame Genre Catalog ({} genres)\x1b[0m\n", catalog.total);
            for genre in &catalog.genres {
                println!("  \x1b[36m{}\x1b[0m - {}", genre.genre, genre.description);
                println!("    \x1b[90mActions: {}\x1b[0m", genre.default_actions.join(", "));
                println!();
            }
        }

        ActionCommands::Genre { genre } => {
            let resp = client.get_genre_actions(&genre).await?;
            println!("{}", serde_json::to_string_pretty(&resp)?);
        }

        ActionCommands::Get { id } => {
            let config = client.get_action_config(&id).await?;
            println!("\x1b[1mAction Config for {}\x1b[0m", id);
            println!("  Genre: {}", config.game_genre.as_str());
            println!("  Enabled actions: {}",
                config.enabled_actions.iter()
                    .map(|a| a.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            if !config.custom_actions.is_empty() {
                println!("  Custom actions:");
                for a in &config.custom_actions {
                    println!("    \x1b[36m{}\x1b[0m: {}", a.name, a.description);
                }
            }
        }

        ActionCommands::Set { id, genre, actions, introduction, action_explain, movement_explain } => {
            let game_genre = parse_genre(&genre)?;
            let enabled: Vec<String> = actions.unwrap_or_default();
            
            let req = npc_types::SetActionSystemConfigRequest {
                game_genre,
                enabled_actions: enabled.iter().map(|s| parse_category(s)).collect::<Result<Vec<_>, _>>()?,
                custom_actions: vec![],
                introduction: introduction.unwrap_or_else(|| "An AI-powered NPC.".to_string()),
                action_explain: action_explain.unwrap_or_else(|| "Use available actions to interact.".to_string()),
                movement_explain: movement_explain.unwrap_or_else(|| "Move freely within bounds.".to_string()),
            };

            client.set_action_config(&id, &req).await?;
            println!("\x1b[32m\u{2713}\x1b[0m Action config set for NPC \x1b[1m{}\x1b[0m", id);
        }

        ActionCommands::Context { id } => {
            let ctx = client.get_ai_context(&id).await?;
            println!("{}", serde_json::to_string_pretty(&ctx)?);
        }
    }

    Ok(())
}

fn parse_genre(s: &str) -> Result<npc_types::GameGenre, SdkError> {
    use npc_types::GameGenre;
    match s.to_lowercase().as_str() {
        "rpg" => Ok(GameGenre::Rpg),
        "fighting" => Ok(GameGenre::Fighting),
        "magic" => Ok(GameGenre::Magic),
        "adventure" => Ok(GameGenre::Adventure),
        "romance" => Ok(GameGenre::Romance),
        "simulation" => Ok(GameGenre::Simulation),
        "visual_novel" => Ok(GameGenre::VisualNovel),
        "survival" => Ok(GameGenre::Survival),
        "horror" => Ok(GameGenre::Horror),
        "sandbox" => Ok(GameGenre::Sandbox),
        "strategy" => Ok(GameGenre::Strategy),
        "sports" => Ok(GameGenre::Sports),
        "puzzle" => Ok(GameGenre::Puzzle),
        "platformer" => Ok(GameGenre::Platformer),
        "shooter" => Ok(GameGenre::Shooter),
        "stealth" => Ok(GameGenre::Stealth),
        "racing" => Ok(GameGenre::Racing),
        "rhythm" => Ok(GameGenre::Rhythm),
        other => Ok(GameGenre::Custom(other.to_string())),
    }
}

fn parse_category(s: &str) -> Result<npc_types::ActionCategory, SdkError> {
    use npc_types::ActionCategory;
    match s {
        "combat" => Ok(ActionCategory::Combat),
        "magic" => Ok(ActionCategory::Magic),
        "crafting" => Ok(ActionCategory::Crafting),
        "trading" => Ok(ActionCategory::Trading),
        "social" => Ok(ActionCategory::Social),
        "exploration" => Ok(ActionCategory::Exploration),
        "stealth" => Ok(ActionCategory::Stealth),
        "farming" => Ok(ActionCategory::Farming),
        "fishing" => Ok(ActionCategory::Fishing),
        "mining" => Ok(ActionCategory::Mining),
        "building" => Ok(ActionCategory::Building),
        "cooking" => Ok(ActionCategory::Cooking),
        "healing" => Ok(ActionCategory::Healing),
        "music" => Ok(ActionCategory::Music),
        "emotes" => Ok(ActionCategory::Emotes),
        "navigation" => Ok(ActionCategory::Navigation),
        "interaction" => Ok(ActionCategory::Interaction),
        "dialogue" => Ok(ActionCategory::Dialogue),
        "quest" => Ok(ActionCategory::Quest),
        _ => Err(SdkError::Config(format!("Unknown action category: {}", s))),
    }
}
