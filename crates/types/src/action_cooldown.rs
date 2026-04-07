use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionCooldownState {
    pub agent_id: String,
    pub cooldowns: HashMap<String, CooldownEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CooldownEntry {
    pub action_name: String,
    pub last_used_timestamp: i64,
    pub cooldown_ms: u64,
    pub remaining_ms: u64,
}

impl ActionCooldownState {
    pub fn new(agent_id: String) -> Self {
        Self {
            agent_id,
            cooldowns: HashMap::new(),
        }
    }

    pub fn can_execute(&self, action_name: &str, cooldown_ms: u64) -> bool {
        match self.cooldowns.get(action_name) {
            None => true,
            Some(entry) => {
                let now = chrono::Utc::now().timestamp_millis();
                let elapsed = (now - entry.last_used_timestamp) as u64;
                elapsed >= cooldown_ms
            }
        }
    }

    pub fn mark_used(&mut self, action_name: String, cooldown_ms: u64) {
        let now = chrono::Utc::now().timestamp_millis();
        self.cooldowns.insert(
            action_name.clone(),
            CooldownEntry {
                action_name,
                last_used_timestamp: now,
                cooldown_ms,
                remaining_ms: cooldown_ms,
            },
        );
    }

    pub fn get_remaining(&self, action_name: &str, cooldown_ms: u64) -> Option<u64> {
        self.cooldowns.get(action_name).and_then(|entry| {
            let now = chrono::Utc::now().timestamp_millis();
            let elapsed = (now - entry.last_used_timestamp) as u64;

            if elapsed >= cooldown_ms {
                None
            } else {
                Some(cooldown_ms - elapsed)
            }
        })
    }

    pub fn clear_cooldown(&mut self, action_name: &str) -> bool {
        self.cooldowns.remove(action_name).is_some()
    }

    pub fn clear_all(&mut self) {
        self.cooldowns.clear();
    }

    pub fn get_all_active_cooldowns(&self) -> Vec<CooldownEntry> {
        let now = chrono::Utc::now().timestamp_millis();

        self.cooldowns
            .values()
            .filter_map(|entry| {
                let elapsed = (now - entry.last_used_timestamp) as u64;
                if elapsed < entry.cooldown_ms {
                    let remaining = entry.cooldown_ms - elapsed;
                    Some(CooldownEntry {
                        remaining_ms: remaining,
                        ..entry.clone()
                    })
                } else {
                    None
                }
            })
            .collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetCooldownsResponse {
    pub agent_id: String,
    pub active_cooldowns: Vec<CooldownEntry>,
    pub total_cooldowns: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_cooldown_state_creation() {
        let state = ActionCooldownState::new("test-agent".into());
        assert!(state.cooldowns.is_empty());
        assert!(state.can_execute("test_action", 1000));
    }

    #[test]
    fn test_mark_used() {
        let mut state = ActionCooldownState::new("test-agent".into());
        state.mark_used("test_action".into(), 1000);

        assert!(!state.can_execute("test_action", 1000));
        assert!(state.cooldowns.contains_key("test_action"));
    }

    #[test]
    fn test_cooldown_expires() {
        let mut state = ActionCooldownState::new("test-agent".into());
        state.mark_used("test_action".into(), 100);

        assert!(!state.can_execute("test_action", 100));

        thread::sleep(Duration::from_millis(150));

        assert!(state.can_execute("test_action", 100));
    }

    #[test]
    fn test_get_remaining() {
        let mut state = ActionCooldownState::new("test-agent".into());
        state.mark_used("test_action".into(), 1000);

        let remaining = state.get_remaining("test_action", 1000);
        assert!(remaining.is_some());
        assert!(remaining.unwrap() <= 1000);
    }

    #[test]
    fn test_clear_cooldown() {
        let mut state = ActionCooldownState::new("test-agent".into());
        state.mark_used("test_action".into(), 1000);

        assert!(!state.can_execute("test_action", 1000));

        assert!(state.clear_cooldown("test_action"));
        assert!(state.can_execute("test_action", 1000));
    }

    #[test]
    fn test_multiple_cooldowns() {
        let mut state = ActionCooldownState::new("test-agent".into());
        state.mark_used("action1".into(), 500);
        state.mark_used("action2".into(), 1000);
        state.mark_used("action3".into(), 1500);

        let active = state.get_all_active_cooldowns();
        assert_eq!(active.len(), 3);

        thread::sleep(Duration::from_millis(600));

        let active = state.get_all_active_cooldowns();
        assert_eq!(active.len(), 2); // action1 should have expired
    }
}
