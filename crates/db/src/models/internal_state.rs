use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct InternalStateRecord {
    pub id: String,
    #[sqlx(rename = "agentId")]
    pub agent_id: String,
    #[sqlx(rename = "currentEmotions")]
    pub current_emotions: serde_json::Value,
    #[sqlx(rename = "moodValence")]
    pub mood_valence: f64,
    #[sqlx(rename = "moodArousal")]
    pub mood_arousal: f64,
    #[sqlx(rename = "moodDominance")]
    pub mood_dominance: f64,
    #[sqlx(rename = "lastUpdated")]
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

impl InternalStateRecord {
    /// Get the dominant emotion from currentEmotions JSON
    pub fn get_dominant_emotion(&self) -> String {
        // currentEmotions is typically a JSON object like:
        // { "joy": 0.8, "sadness": 0.2, "anger": 0.1 }
        if let Some(emotions) = self.current_emotions.as_object() {
            let mut max_emotion = "neutral";
            let mut max_value = 0.0;

            for (emotion, value) in emotions {
                if let Some(v) = value.as_f64() {
                    if v > max_value {
                        max_value = v;
                        max_emotion = emotion;
                    }
                }
            }

            max_emotion.to_string()
        } else {
            "neutral".to_string()
        }
    }

    /// Check if emotional state is stale
    pub fn is_stale(&self, max_age_seconds: i64) -> bool {
        let now = chrono::Utc::now();
        let age = now.signed_duration_since(self.last_updated);
        age.num_seconds() > max_age_seconds
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_get_dominant_emotion() {
        let state = InternalStateRecord {
            id: "test".to_string(),
            agent_id: "agent123".to_string(),
            current_emotions: json!({
                "joy": 0.8,
                "sadness": 0.2,
                "anger": 0.1
            }),
            mood_valence: 0.5,
            mood_arousal: 0.5,
            mood_dominance: 0.5,
            last_updated: chrono::Utc::now(),
        };

        assert_eq!(state.get_dominant_emotion(), "joy");
    }

    #[test]
    fn test_is_stale() {
        let old_state = InternalStateRecord {
            id: "test".to_string(),
            agent_id: "agent123".to_string(),
            current_emotions: json!({}),
            mood_valence: 0.5,
            mood_arousal: 0.5,
            mood_dominance: 0.5,
            last_updated: chrono::Utc::now() - chrono::Duration::seconds(120),
        };

        assert!(old_state.is_stale(60));

        let fresh_state = InternalStateRecord {
            last_updated: chrono::Utc::now(),
            ..old_state
        };

        assert!(!fresh_state.is_stale(60));
    }
}
