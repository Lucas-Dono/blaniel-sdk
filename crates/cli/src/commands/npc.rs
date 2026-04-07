use blaniel_sdk::{BlanielClient, SdkError};
use npc_types::*;

#[derive(clap::Subcommand)]
pub enum NpcCommands {
    Get {
        #[arg(help = "NPC/Agent ID")]
        id: String,
    },
    Nearby {
        #[arg(long, default_value_t = 0.0)]
        x: f64,
        #[arg(long, default_value_t = 64.0)]
        y: f64,
        #[arg(long, default_value_t = 0.0)]
        z: f64,
        #[arg(long, default_value = "overworld")]
        world: String,
        #[arg(long, default_value_t = 50.0)]
        radius: f64,
    },
    Chat {
        #[arg(help = "NPC/Agent ID")]
        id: String,
        #[arg(help = "Message to send")]
        message: String,
    },
    Move {
        #[arg(help = "NPC/Agent ID")]
        id: String,
        #[arg(long)]
        x: f64,
        #[arg(long)]
        y: f64,
        #[arg(long)]
        z: f64,
        #[arg(long, default_value = "overworld")]
        world: String,
    },
    Pathfind {
        #[arg(help = "NPC/Agent ID")]
        id: String,
        #[arg(long)]
        from_x: f64,
        #[arg(long)]
        from_y: f64,
        #[arg(long)]
        from_z: f64,
        #[arg(long)]
        to_x: f64,
        #[arg(long)]
        to_y: f64,
        #[arg(long)]
        to_z: f64,
        #[arg(long, default_value = "overworld")]
        world: String,
    },
    Navigate {
        #[arg(help = "NPC/Agent ID")]
        id: String,
        #[arg(long, help = "Named location (word navigation)")]
        name: Option<String>,
        #[arg(long)]
        to_x: Option<f64>,
        #[arg(long)]
        to_y: Option<f64>,
        #[arg(long)]
        to_z: Option<f64>,
        #[arg(long, default_value = "overworld")]
        world: String,
    },
    Setup {
        #[arg(help = "NPC/Agent ID")]
        id: String,
        #[arg(long, default_value = "rpg")]
        genre: String,
        #[arg(long, default_value = "hybrid")]
        nav_mode: String,
        #[arg(long)]
        scene_id: Option<String>,
    },
}

pub async fn run(cmd: NpcCommands) -> Result<(), SdkError> {
    let client = BlanielClient::from_env()?;

    match cmd {
        NpcCommands::Get { id } => {
            let state = client.get_npc(&id).await?;
            println!("\x1b[1m{}\x1b[0m ({})", state.name, state.id);
            if let Some(pos) = &state.position {
                println!("  Position: ({}, {}, {}) in {}", pos.x, pos.y, pos.z, pos.world);
            }
            println!("  Action: {}", state.current_action.as_str());
            println!("  Emotion: {}", state.cached_emotion);
            println!("  Animation: {}", state.animation.as_str());
        }

        NpcCommands::Nearby { x, y, z, world, radius } => {
            let resp = client.get_nearby_npcs(x, y, z, &world, radius).await?;
            println!("Found \x1b[1m{}\x1b[0m NPCs within {} blocks of ({}, {}, {}):", 
                resp.total, radius, x, y, z);
            for npc in &resp.npcs {
                let pos = npc.position.as_ref()
                    .map(|p| format!("({}, {}, {})", p.x, p.y, p.z))
                    .unwrap_or_else(|| "unknown".to_string());
                println!("  \x1b[36m{}\x1b[0m - {} [{}]", npc.id, npc.name, pos);
            }
        }

        NpcCommands::Chat { id, message } => {
            let resp = client.chat_simple(&id, &message).await?;
            println!("\x1b[36m{}\x1b[0m", resp.response);
            println!("\x1b[90m  emotion: {} | animation: {} | source: {} | {}ms\x1b[0m",
                resp.emotion, resp.animation, resp.source, resp.latency_ms);
        }

        NpcCommands::Move { id, x, y, z, world } => {
            let pos = Position3D::new(x, y, z, world);
            let state = client.move_npc(&id, pos, None, None).await?;
            println!("\x1b[32m\u{2713}\x1b[0m Moved {} to ({}, {}, {})", 
                state.name, x, y, z);
        }

        NpcCommands::Pathfind { id, from_x, from_y, from_z, to_x, to_y, to_z, world } => {
            let start = Position3D::new(from_x, from_y, from_z, world.clone());
            let goal = Position3D::new(to_x, to_y, to_z, world);
            let result = client.pathfind(&id, start, goal, None).await?;
            println!("Path: \x1b[1m{}\x1b[0m steps, cost: {:.2}, computed in {}ms",
                result.length(), result.cost, result.computed_in_ms);
            for (i, pos) in result.path.iter().enumerate() {
                if i < 5 || i >= result.path.len() - 2 {
                    println!("  {}: ({}, {}, {})", i, pos.x, pos.y, pos.z);
                } else if i == 5 {
                    println!("  ... ({} more steps)", result.path.len() - 7);
                }
            }
        }

        NpcCommands::Navigate { id, name, to_x, to_y, to_z, world } => {
            let target = if let Some(n) = name {
                NavigationTarget::Word { name: n }
            } else {
                let x = to_x.unwrap_or(0.0);
                let y = to_y.unwrap_or(0.0);
                let z = to_z.unwrap_or(0.0);
                NavigationTarget::Coordinate {
                    position: Position3D::new(x, y, z, world),
                }
            };
            let resp = client.navigate(&id, target, None).await?;
            if resp.success {
                println!("\x1b[32m\u{2713}\x1b[0m Navigation started");
                if let Some(name) = resp.target_name {
                    println!("  Target: {}", name);
                }
                if let Some(dist) = resp.distance {
                    println!("  Distance: {:.1} blocks", dist);
                }
                if let Some(time) = resp.estimated_time_ms {
                    println!("  ETA: {}ms", time);
                }
            } else {
                println!("\x1b[31m\u{2717}\x1b[0m Navigation failed");
            }
        }

        NpcCommands::Setup { id, genre, nav_mode, scene_id } => {
            let genre = parse_genre(&genre)?;
            let nav_mode = parse_nav_mode(&nav_mode)?;

            println!("\x1b[36mSetting up NPC \x1b[1m{}\x1b[0m...", id);
            println!("  Genre: {}", genre.as_str());
            println!("  Navigation: {}", nav_mode.as_str());

            let result = client.setup_npc(&id, genre, nav_mode, scene_id.as_deref(), vec![]).await?;
            
            for step in &result.steps {
                if step.contains("_failed") {
                    println!("  \x1b[31m\u{2717}\x1b[0m {}", step);
                } else {
                    println!("  \x1b[32m\u{2713}\x1b[0m {}", step);
                }
            }
        }
    }

    Ok(())
}

fn parse_genre(s: &str) -> Result<GameGenre, SdkError> {
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

fn parse_nav_mode(s: &str) -> Result<NavigationMode, SdkError> {
    match s.to_lowercase().as_str() {
        "word" => Ok(NavigationMode::Word),
        "coordinate" => Ok(NavigationMode::Coordinate),
        "hybrid" => Ok(NavigationMode::Hybrid),
        "free" => Ok(NavigationMode::Free),
        _ => Err(SdkError::Config(format!(
            "Invalid navigation mode '{}'. Use: word, coordinate, hybrid, free",
            s
        ))),
    }
}
