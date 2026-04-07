use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlanielConfig {
    pub api_url: String,
    pub api_key: String,
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
    #[serde(default)]
    pub default_world: String,
}

fn default_timeout() -> u64 {
    30000
}

impl Default for BlanielConfig {
    fn default() -> Self {
        Self {
            api_url: "http://localhost:3001".to_string(),
            api_key: String::new(),
            timeout_ms: 30000,
            default_world: "overworld".to_string(),
        }
    }
}

impl BlanielConfig {
    pub fn new(api_url: impl Into<String>, api_key: impl Into<String>) -> Self {
        Self {
            api_url: api_url.into(),
            api_key: api_key.into(),
            timeout_ms: 30000,
            default_world: "overworld".to_string(),
        }
    }

    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }

    pub fn with_default_world(mut self, world: impl Into<String>) -> Self {
        self.default_world = world.into();
        self
    }

    pub fn config_path() -> PathBuf {
        let home = dirs_home().unwrap_or_else(|| PathBuf::from("."));
        home.join(".blaniel").join("config.json")
    }

    pub fn load() -> Option<Self> {
        let path = Self::config_path();
        let content = std::fs::read_to_string(path).ok()?;
        serde_json::from_str(&content).ok()
    }

    pub fn save(&self) -> Result<(), std::io::Error> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)
    }
}

fn dirs_home() -> Option<PathBuf> {
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map(PathBuf::from)
        .ok()
}
