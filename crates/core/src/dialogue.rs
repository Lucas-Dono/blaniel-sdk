use rand::seq::SliceRandom;

use npc_types::{AmbientDialogue, ChatResponse};

pub struct AmbientDialogueManager {
    greetings: Vec<&'static str>,
    farewells: Vec<&'static str>,
    wellbeing_responses: Vec<&'static str>,
    acknowledgments: Vec<&'static str>,
}

impl Default for AmbientDialogueManager {
    fn default() -> Self {
        Self::new()
    }
}

impl AmbientDialogueManager {
    pub fn new() -> Self {
        Self {
            greetings: vec![
                "Hello! How can I help?",
                "Hey there!",
                "Greetings, traveler!",
                "Good to see you!",
                "Welcome!",
            ],
            farewells: vec![
                "Goodbye! Have a great day!",
                "See you later!",
                "Take care!",
                "Farewell!",
                "Until next time!",
            ],
            wellbeing_responses: vec![
                "I'm doing great, thanks for asking!",
                "Pretty good, how about you?",
                "Excellent! How are you?",
                "All good here!",
                "Doing well, thank you!",
            ],
            acknowledgments: vec!["Understood.", "Okay.", "Sure thing.", "Got it.", "Right."],
        }
    }

    pub fn try_simple_response(&self, message: &str) -> Option<ChatResponse> {
        let normalized = message.trim().to_lowercase();

        if self.is_greeting(&normalized) {
            return Some(self.random_greeting());
        }

        if self.is_farewell(&normalized) {
            return Some(self.random_farewell());
        }

        if self.is_wellbeing_question(&normalized) {
            return Some(self.random_wellbeing_response());
        }

        if self.is_simple_acknowledgment(&normalized) {
            return Some(self.random_acknowledgment());
        }

        None
    }

    fn is_greeting(&self, msg: &str) -> bool {
        let greetings = ["hello", " hi ", "hey ", "good morning", "hi", "greetings"];
        let trimmed = msg.trim();
        greetings.iter().any(|&g| {
            if g.starts_with(' ') && g.ends_with(' ') {
                msg.contains(g.trim())
                    && (msg.starts_with(g.trim()) || msg.contains(&format!(" {}", g.trim())))
                    && (msg.ends_with(g.trim()) || msg.contains(&format!("{} ", g.trim())))
            } else {
                trimmed == g
                    || trimmed.starts_with(&format!("{} ", g))
                    || trimmed.starts_with(&format!("{},", g))
            }
        }) || trimmed == "hi"
            || trimmed == "hey"
    }

    fn is_farewell(&self, msg: &str) -> bool {
        let farewells = ["goodbye", "bye", "see you", "farewell", "later"];
        farewells.iter().any(|&f| msg.contains(f))
    }

    fn is_wellbeing_question(&self, msg: &str) -> bool {
        let patterns = ["how are you", "how's it going", "how are you doing"];
        patterns.iter().any(|&p| msg.contains(p))
    }

    fn is_simple_acknowledgment(&self, msg: &str) -> bool {
        let acks = ["ok", "okay", "yes", "sure", "alright"];
        acks.iter().any(|&a| msg == a)
    }

    fn random_greeting(&self) -> ChatResponse {
        self.create_response(
            self.greetings.choose(&mut rand::thread_rng()).unwrap(),
            "joy",
            "wave",
        )
    }

    fn random_farewell(&self) -> ChatResponse {
        self.create_response(
            self.farewells.choose(&mut rand::thread_rng()).unwrap(),
            "joy",
            "wave",
        )
    }

    fn random_wellbeing_response(&self) -> ChatResponse {
        self.create_response(
            self.wellbeing_responses
                .choose(&mut rand::thread_rng())
                .unwrap(),
            "joy",
            "talk",
        )
    }

    fn random_acknowledgment(&self) -> ChatResponse {
        self.create_response(
            self.acknowledgments
                .choose(&mut rand::thread_rng())
                .unwrap(),
            "neutral",
            "nod",
        )
    }

    fn create_response(&self, text: &str, emotion: &str, animation: &str) -> ChatResponse {
        ChatResponse {
            response: text.to_string(),
            emotion: emotion.to_string(),
            animation: animation.to_string(),
            source: "rust_cache".to_string(),
            latency_ms: 2,
            cached: Some(false),
        }
    }

    pub fn generate_ambient_dialogues(
        &self,
        _participant_ids: &[String],
        context: &str,
    ) -> Vec<AmbientDialogue> {
        match context {
            "tavern" => self.tavern_dialogues(),
            "marketplace" => self.marketplace_dialogues(),
            "forest" => self.forest_dialogues(),
            _ => self.generic_dialogues(),
        }
    }

    fn tavern_dialogues(&self) -> Vec<AmbientDialogue> {
        vec![
            AmbientDialogue {
                speaker_id: "npc1".to_string(),
                message: "Barkeep, another round please!".to_string(),
                emotion: "joy".to_string(),
                animation: "talk".to_string(),
            },
            AmbientDialogue {
                speaker_id: "npc2".to_string(),
                message: "Have you heard the news from the kingdom?".to_string(),
                emotion: "curiosity".to_string(),
                animation: "talk".to_string(),
            },
        ]
    }

    fn marketplace_dialogues(&self) -> Vec<AmbientDialogue> {
        vec![
            AmbientDialogue {
                speaker_id: "merchant1".to_string(),
                message: "Fresh apples! Best in the kingdom!".to_string(),
                emotion: "joy".to_string(),
                animation: "point".to_string(),
            },
            AmbientDialogue {
                speaker_id: "customer1".to_string(),
                message: "How much for the bread?".to_string(),
                emotion: "neutral".to_string(),
                animation: "talk".to_string(),
            },
        ]
    }

    fn forest_dialogues(&self) -> Vec<AmbientDialogue> {
        vec![AmbientDialogue {
            speaker_id: "ranger1".to_string(),
            message: "Be careful, there are wolves around here.".to_string(),
            emotion: "concern".to_string(),
            animation: "point".to_string(),
        }]
    }

    fn generic_dialogues(&self) -> Vec<AmbientDialogue> {
        vec![AmbientDialogue {
            speaker_id: "npc1".to_string(),
            message: "How's your day going?".to_string(),
            emotion: "neutral".to_string(),
            animation: "talk".to_string(),
        }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_greeting() {
        let manager = AmbientDialogueManager::new();

        let response = manager.try_simple_response("Hello");
        assert!(response.is_some());

        let response = response.unwrap();
        assert_eq!(response.source, "rust_cache");
        assert_eq!(response.emotion, "joy");
    }

    #[test]
    fn test_farewell() {
        let manager = AmbientDialogueManager::new();

        let response = manager.try_simple_response("Goodbye");
        assert!(response.is_some());

        let response = response.unwrap();
        assert_eq!(response.animation, "wave");
    }

    #[test]
    fn test_wellbeing_question() {
        let manager = AmbientDialogueManager::new();

        let response = manager.try_simple_response("How are you?");
        assert!(response.is_some());
    }

    #[test]
    fn test_complex_message() {
        let manager = AmbientDialogueManager::new();

        let response = manager.try_simple_response("Tell me about the history of this kingdom");
        assert!(response.is_none());
    }

    #[test]
    fn test_ambient_dialogues() {
        let manager = AmbientDialogueManager::new();

        let dialogues =
            manager.generate_ambient_dialogues(&["npc1".to_string(), "npc2".to_string()], "tavern");

        assert!(!dialogues.is_empty());
    }
}
