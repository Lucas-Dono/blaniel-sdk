use blaniel_sdk::{BlanielClient, SdkError};

#[derive(clap::Args)]
#[command(about = "Interactive quick setup wizard for a new NPC")]
pub struct SetupCommand {
    #[arg(help = "NPC/Agent ID")]
    pub id: String,
}

impl SetupCommand {
    pub async fn run(&self) -> Result<(), SdkError> {
        let client = BlanielClient::from_env()?;

        println!("\x1b[36m\x1b[1m  Blaniel NPC Quick Setup\x1b[0m\n");

        use dialoguer::{Select, Input, MultiSelect, Confirm};

        let genres = npc_types::GameGenre::all_genres();
        let genre_idx = Select::new()
            .with_prompt("  Select game genre")
            .items(&genres)
            .default(0)
            .interact()
            .map_err(|e| SdkError::Config(e.to_string()))?;

        let genre_str = genres[genre_idx];
        let genre = match genre_str {
            "rpg" => npc_types::GameGenre::Rpg,
            "fighting" => npc_types::GameGenre::Fighting,
            "magic" => npc_types::GameGenre::Magic,
            "adventure" => npc_types::GameGenre::Adventure,
            "romance" => npc_types::GameGenre::Romance,
            "simulation" => npc_types::GameGenre::Simulation,
            "visual_novel" => npc_types::GameGenre::VisualNovel,
            "survival" => npc_types::GameGenre::Survival,
            "horror" => npc_types::GameGenre::Horror,
            "sandbox" => npc_types::GameGenre::Sandbox,
            "strategy" => npc_types::GameGenre::Strategy,
            "sports" => npc_types::GameGenre::Sports,
            "puzzle" => npc_types::GameGenre::Puzzle,
            "platformer" => npc_types::GameGenre::Platformer,
            "shooter" => npc_types::GameGenre::Shooter,
            "stealth" => npc_types::GameGenre::Stealth,
            "racing" => npc_types::GameGenre::Racing,
            "rhythm" => npc_types::GameGenre::Rhythm,
            _ => npc_types::GameGenre::Custom(genre_str.to_string()),
        };

        let default_actions: Vec<String> = genre.default_actions()
            .iter()
            .map(|a| a.as_str().to_string())
            .collect();

        let action_labels: Vec<String> = npc_types::ActionCategory::all_categories()
            .iter()
            .map(|a| format!("{} - {}", a.as_str(), a.description()))
            .collect();

        let defaults: Vec<bool> = npc_types::ActionCategory::all_categories()
            .iter()
            .map(|a| default_actions.contains(&a.as_str().to_string()))
            .collect();

        println!("\n  \x1b[1mSelect actions\x1b[0m (defaults for {} shown):\n", genre_str);
        let selected = MultiSelect::new()
            .items(&action_labels)
            .defaults(&defaults)
            .interact()
            .map_err(|e| SdkError::Config(e.to_string()))?;

        let enabled: Vec<npc_types::ActionCategory> = selected.into_iter()
            .map(|i| npc_types::ActionCategory::all_categories()[i].clone())
            .collect();

        let nav_modes = vec!["word", "coordinate", "hybrid", "free"];
        let nav_idx = Select::new()
            .with_prompt("\n  Navigation mode")
            .items(&nav_modes)
            .default(2)
            .interact()
            .map_err(|e| SdkError::Config(e.to_string()))?;

        let nav_mode = match nav_idx {
            0 => npc_types::NavigationMode::Word,
            1 => npc_types::NavigationMode::Coordinate,
            2 => npc_types::NavigationMode::Hybrid,
            3 => npc_types::NavigationMode::Free,
            _ => npc_types::NavigationMode::Hybrid,
        };

        let scene_id: String = Input::new()
            .with_prompt("\n  Scene ID (leave empty to skip)")
            .allow_empty(true)
            .default(String::new())
            .interact()
            .map_err(|e| SdkError::Config(e.to_string()))?;

        let intro: String = Input::new()
            .with_prompt("\n  Introduction (tells the NPC who/where it is)")
            .default(format!("You are an NPC in a {} game world.", genre_str))
            .interact()
            .map_err(|e| SdkError::Config(e.to_string()))?;

        let action_explain: String = Input::new()
            .with_prompt("  Action explanation (tells the NPC what it can do)")
            .default("You can interact with the world using your available actions.".to_string())
            .interact()
            .map_err(|e| SdkError::Config(e.to_string()))?;

        let movement_explain: String = Input::new()
            .with_prompt("  Movement explanation (tells the NPC how it can move)")
            .default(format!("You navigate using {} mode.", nav_modes[nav_idx]))
            .interact()
            .map_err(|e| SdkError::Config(e.to_string()))?;

        println!("\n  \x1b[90mConfiguring NPC...\x1b[0m\n");

        if !scene_id.is_empty() {
            let nav_config = npc_types::NavigationConfig {
                mode: nav_mode,
                restrictions: None,
                scene_id: Some(scene_id.clone()),
                default_speed: 4.0,
                introduction: String::new(),
                explain: String::new(),
            };
            match client.set_navigation_config(&self.id, &nav_config).await {
                Ok(_) => println!("  \x1b[32m\u{2713}\x1b[0m Navigation configured"),
                Err(e) => println!("  \x1b[31m\u{2717}\x1b[0m Navigation failed: {}", e),
            }
        }

        let action_req = npc_types::SetActionSystemConfigRequest {
            game_genre: genre,
            enabled_actions: enabled,
            custom_actions: vec![],
            introduction: intro,
            action_explain,
            movement_explain,
        };
        match client.set_action_config(&self.id, &action_req).await {
            Ok(_) => println!("  \x1b[32m\u{2713}\x1b[0m Action system configured"),
            Err(e) => println!("  \x1b[31m\u{2717}\x1b[0m Actions failed: {}", e),
        }

        println!("\n  \x1b[32m\x1b[1mSetup complete!\x1b[0m");
        println!("  Use \x1b[36mblaniel npc chat {} \"Hello\"\x1b[0m to test\n", self.id);

        Ok(())
    }
}
