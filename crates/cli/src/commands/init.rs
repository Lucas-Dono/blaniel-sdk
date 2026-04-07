use blaniel_sdk::BlanielConfig;
use blaniel_sdk::SdkError;

#[derive(clap::Args)]
#[command(about = "Initialize Blaniel CLI configuration")]
pub struct InitCommand {
    #[arg(short, long, help = "API base URL")]
    pub url: Option<String>,
    #[arg(short, long, help = "API key (JWT token)")]
    pub key: Option<String>,
}

impl InitCommand {
    pub async fn run(&self) -> Result<(), SdkError> {
        let url = self.url.as_deref().unwrap_or("http://localhost:3001");
        
        println!("\x1b[36m\x1b[1m  Blaniel NPC AI - Configuration\x1b[0m\n");
        println!("  API URL: \x1b[32m{}\x1b[0m", url);

        let key = match &self.key {
            Some(k) => k.clone(),
            None => {
                use dialoguer::Password;
                Password::new()
                    .with_prompt("  Enter your API key")
                    .interact()
                    .map_err(|e| SdkError::Config(e.to_string()))?
            }
        };

        if key.is_empty() {
            return Err(SdkError::Config("API key cannot be empty".to_string()));
        }

        let config = BlanielConfig::new(url, &key);
        config.save().map_err(|e| SdkError::Config(e.to_string()))?;

        println!("\n  \x1b[32m\u{2713}\x1b[0m Configuration saved to ~/.blaniel/config.json");
        println!("  \x1b[90mUse `blaniel npc get <id>` to test the connection\x1b[0m\n");

        let client = blaniel_sdk::BlanielClient::new(config)?;
        match client.health().await {
            Ok(health) => {
                println!("  \x1b[32m\u{2713}\x1b[0m Connected! Status: {}, Version: {}", 
                    health.status, health.version);
            }
            Err(e) => {
                println!("  \x1b[33m\u{26A0}\x1b[0m Could not reach API: {}", e);
                println!("  \x1b[90mConfiguration saved anyway. Start the API and try again.\x1b[0m");
            }
        }

        Ok(())
    }
}
