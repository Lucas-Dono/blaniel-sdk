use blaniel_sdk::{BlanielClient, SdkError, Position3D, ScenePosition};
use serde_json::Value;

#[derive(clap::Subcommand)]
pub enum SceneCommands {
    Register {
        #[arg(long, help = "Scene ID")]
        scene_id: String,
        #[arg(long, default_value = "overworld")]
        world: String,
        #[arg(long, help = "JSON file with scene positions")]
        file: Option<String>,
        #[arg(long, help = "Inline JSON positions")]
        positions: Option<String>,
    },
    List {
        #[arg(help = "Scene ID")]
        scene_id: String,
    },
}

pub async fn run(cmd: SceneCommands) -> Result<(), SdkError> {
    let client = BlanielClient::from_env()?;

    match cmd {
        SceneCommands::Register { scene_id, world, file, positions } => {
            let positions_json = if let Some(path) = file {
                std::fs::read_to_string(&path)
                    .map_err(|e| SdkError::Config(format!("Failed to read {}: {}", path, e)))?
            } else if let Some(json) = positions {
                json
            } else {
                return Err(SdkError::Config(
                    "Provide --file <path> or --positions <json>".to_string(),
                ));
            };

            let positions: Vec<ScenePosition> =
                serde_json::from_str(&positions_json).map_err(|e| {
                    SdkError::Config(format!("Invalid positions JSON: {}", e))
                })?;

            let count = positions.len();
            client.register_scene(&scene_id, &world, positions).await?;
            println!("\x1b[32m\u{2713}\x1b[0m Registered scene \x1b[1m{}\x1b[0m with {} positions", scene_id, count);
        }

        SceneCommands::List { scene_id } => {
            let resp = client.get_scene_positions(&scene_id).await?;
            println!("\x1b[1m{}\x1b[0m - {} positions\n", resp.scene_id, resp.total);
            for pos in &resp.positions {
                let p = &pos.position;
                println!("  \x1b[36m{}\x1b[0m at ({}, {}, {})",
                    pos.name, p.x, p.y, p.z);
                if !pos.aliases.is_empty() {
                    println!("    Aliases: {}", pos.aliases.join(", "));
                }
                if !pos.tags.is_empty() {
                    println!("    Tags: {}", pos.tags.join(", "));
                }
            }
        }
    }

    Ok(())
}
