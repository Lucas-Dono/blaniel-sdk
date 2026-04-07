use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

use npc_types::{ChatResponse, Position3D};

/// Predicts what responses to pre-cache based on player behavior
///
/// Strategies:
/// - Proximity: Pre-load responses for NPCs near players
/// - Time-based: Pre-load time-specific greetings (morning/night)
/// - Behavioral: Learn common conversation patterns
pub struct PredictiveCacheWarmer {
    common_greetings: Arc<RwLock<HashMap<String, Vec<String>>>>,
    context_templates: Arc<RwLock<HashMap<String, ChatResponse>>>,
}

impl PredictiveCacheWarmer {
    pub fn new() -> Self {
        Self {
            common_greetings: Arc::new(RwLock::new(Self::default_greetings())),
            context_templates: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Default common greetings by NPC type
    fn default_greetings() -> HashMap<String, Vec<String>> {
        let mut greetings = HashMap::new();

        greetings.insert(
            "merchant".to_string(),
            vec![
                "Hello".to_string(),
                "What are you selling?".to_string(),
                "I need supplies".to_string(),
                "Show me your wares".to_string(),
            ],
        );

        greetings.insert(
            "guard".to_string(),
            vec![
                "Hello".to_string(),
                "Is everything okay?".to_string(),
                "Have you seen anything suspicious?".to_string(),
            ],
        );

        greetings.insert(
            "villager".to_string(),
            vec![
                "Hello".to_string(),
                "Hi".to_string(),
                "How are you?".to_string(),
                "What's new?".to_string(),
            ],
        );

        greetings.insert(
            "quest_giver".to_string(),
            vec![
                "Hello".to_string(),
                "Do you have any quests?".to_string(),
                "What do you need help with?".to_string(),
                "Any work for me?".to_string(),
            ],
        );

        greetings
    }

    /// Get common messages for an NPC type
    pub async fn get_warmup_messages(&self, npc_kind: &str) -> Vec<String> {
        let greetings = self.common_greetings.read().await;

        greetings
            .get(npc_kind)
            .cloned()
            .unwrap_or_else(|| {
                greetings
                    .get("villager")
                    .cloned()
                    .unwrap_or_default()
            })
    }

    /// Predict which NPCs need cache warming based on player position
    ///
    /// Returns list of (npc_id, predicted_messages)
    pub fn predict_nearby_npcs(
        player_pos: &Position3D,
        npc_positions: &[(String, Position3D, String)], // (id, pos, kind)
        radius: f64,
    ) -> Vec<(String, String)> {
        let radius_squared = radius * radius;
        let mut predictions = Vec::new();

        for (npc_id, npc_pos, npc_kind) in npc_positions {
            // Calculate squared distance
            let dx = player_pos.x - npc_pos.x;
            let dy = player_pos.y - npc_pos.y;
            let dz = player_pos.z - npc_pos.z;
            let dist_sq = dx * dx + dy * dy + dz * dz;

            if dist_sq <= radius_squared {
                // Predict most likely greeting based on NPC type
                let message = match npc_kind.as_str() {
                    "merchant" => "What are you selling?",
                    "guard" => "Hello",
                    "quest_giver" => "Do you have any quests?",
                    _ => "Hello",
                };

                predictions.push((npc_id.clone(), message.to_string()));
            }
        }

        debug!(
            "Predicted {} cache warmup targets near player",
            predictions.len()
        );

        predictions
    }

    /// Get time-based greeting context
    pub fn time_based_context(time_of_day: &str) -> Option<String> {
        match time_of_day {
            "morning" | "dawn" => Some("It's early morning. The NPC should greet warmly with morning-specific phrases.".to_string()),
            "day" | "noon" => Some("It's daytime. The NPC should be active and welcoming.".to_string()),
            "evening" | "dusk" => Some("It's evening. The NPC might be tired or closing shop.".to_string()),
            "night" => Some("It's nighttime. The NPC should be sleepy or wary of strangers.".to_string()),
            _ => None,
        }
    }

    /// Store a template response for quick lookup
    pub async fn cache_template(&self, key: String, response: npc_types::ChatResponse) {
        let mut templates = self.context_templates.write().await;
        let key_str = key.clone();
        templates.insert(key, response);
        debug!("Cached template: {}", key_str);
    }

    /// Get cached template if available
    pub async fn get_template(&self, key: &str) -> Option<ChatResponse> {
        let templates = self.context_templates.read().await;
        templates.get(key).cloned()
    }

    /// Generate cache key for template lookup
    pub fn template_key(npc_id: &str, message: &str, time: &str) -> String {
        format!("{}:{}:{}", npc_id, message.to_lowercase(), time)
    }
}

/// Automatic cache warming task
///
/// Runs in background and pre-loads likely responses
pub struct CacheWarmingTask {
    warmer: Arc<PredictiveCacheWarmer>,
    interval: std::time::Duration,
}

impl CacheWarmingTask {
    pub fn new(warmer: Arc<PredictiveCacheWarmer>) -> Self {
        Self {
            warmer,
            interval: std::time::Duration::from_secs(30), // Warm every 30s
        }
    }

    /// Start background warming task
    pub fn start(self) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            info!("Cache warming task started");
            let mut interval = tokio::time::interval(self.interval);

            loop {
                interval.tick().await;

                // Here you would:
                // 1. Query active players from database
                // 2. Get NPCs near each player
                // 3. Pre-generate responses for likely interactions
                // 4. Store in cache

                debug!("Cache warming tick (placeholder - needs DB integration)");
            }
        })
    }
}

/// Response template generator
///
/// Creates instant responses for common patterns
pub struct ResponseTemplates;

impl ResponseTemplates {
    /// Generate instant greeting response
    pub fn greeting(npc_name: &str, time_of_day: &str) -> npc_types::ChatResponse {
        let message = match time_of_day {
            "morning" | "dawn" => format!("Good morning! I'm {}, how may I help you today?", npc_name),
            "day" | "noon" => format!("Hello! I'm {}, what brings you here?", npc_name),
            "evening" | "dusk" => format!("Good evening. I'm {}. How can I assist you?", npc_name),
            "night" => format!("*yawns* Oh, hello. I'm {}. It's quite late...", npc_name),
            _ => format!("Hello! I'm {}.", npc_name),
        };

        npc_types::ChatResponse {
            response: message,
            emotion: "neutral".to_string(),
            animation: "wave".to_string(),
            source: "template".to_string(),
            latency_ms: 0,
            cached: Some(true),
        }
    }

    pub fn merchant_greeting(npc_name: &str) -> npc_types::ChatResponse {
        npc_types::ChatResponse {
            response: format!(
                "Welcome! I'm {}, the merchant. Looking for supplies? I have the finest goods!",
                npc_name
            ),
            emotion: "cheerful".to_string(),
            animation: "wave".to_string(),
            source: "template".to_string(),
            latency_ms: 0,
            cached: Some(true),
        }
    }

    pub fn guard_greeting(npc_name: &str) -> npc_types::ChatResponse {
        npc_types::ChatResponse {
            response: format!(
                "Halt! I'm {}, city guard. State your business.",
                npc_name
            ),
            emotion: "serious".to_string(),
            animation: "nod".to_string(),
            source: "template".to_string(),
            latency_ms: 0,
            cached: Some(true),
        }
    }

    pub fn get_template(npc_kind: &str, npc_name: &str, message: &str, _time: &str) -> Option<npc_types::ChatResponse> {
        let message_lower = message.to_lowercase();

        if message_lower.contains("hello") || message_lower.contains("hi") || message_lower == "greet" {
            match npc_kind {
                "merchant" => return Some(Self::merchant_greeting(npc_name)),
                "guard" => return Some(Self::guard_greeting(npc_name)),
                _ => return Some(Self::greeting(npc_name, _time)),
            }
        }

        if npc_kind == "merchant" {
            if message_lower.contains("sell") || message_lower.contains("buy") || message_lower.contains("shop") {
                return Some(Self::merchant_greeting(npc_name));
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_warmup_messages() {
        let warmer = PredictiveCacheWarmer::new();
        let messages = warmer.get_warmup_messages("merchant").await;
        assert!(!messages.is_empty());
        assert!(messages.contains(&"What are you selling?".to_string()));
    }

    #[test]
    fn test_proximity_prediction() {
        let player_pos = Position3D::new(0.0, 64.0, 0.0, "overworld".to_string());

        let npcs = vec![
            ("npc1".to_string(), Position3D::new(5.0, 64.0, 5.0, "overworld".to_string()), "merchant".to_string()),
            ("npc2".to_string(), Position3D::new(100.0, 64.0, 100.0, "overworld".to_string()), "guard".to_string()),
        ];

        let predictions = PredictiveCacheWarmer::predict_nearby_npcs(&player_pos, &npcs, 20.0);

        assert_eq!(predictions.len(), 1);
        assert_eq!(predictions[0].0, "npc1");
    }

    #[test]
    fn test_templates() {
        let response = ResponseTemplates::greeting("TestNPC", "morning");
        assert!(response.response.contains("Good morning"));
        assert_eq!(response.latency_ms, 0);

        let merchant = ResponseTemplates::merchant_greeting("Trader Bob");
        assert!(merchant.response.contains("supplies"));
    }
}
