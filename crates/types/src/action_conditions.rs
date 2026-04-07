use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum ActionCondition {
    MinStat { stat: String, min_value: f64 },
    MaxStat { stat: String, max_value: f64 },
    HasItem { item_id: String, quantity: Option<u32> },
    LocationIs { location: String },
    TimeOfDay { min_hour: u8, max_hour: u8 },
    Cooldown { cooldown_ms: u64 },
    CustomScript { script: String },
}

impl ActionCondition {
    pub fn evaluate(&self, context: &ActionContext) -> bool {
        match self {
            Self::MinStat { stat, min_value } => {
                context.stats.get(stat).map_or(false, |v| v >= min_value)
            }
            Self::MaxStat { stat, max_value } => {
                context.stats.get(stat).map_or(false, |v| v <= max_value)
            }
            Self::HasItem { item_id, quantity } => {
                let has_item = context.inventory.contains(item_id);
                if let Some(required_qty) = quantity {
                    if let Some(&actual_qty) = context.item_quantities.get(item_id) {
                        return has_item && actual_qty >= *required_qty;
                    }
                    false
                } else {
                    has_item
                }
            }
            Self::LocationIs { location } => {
                &context.location == location
            }
            Self::TimeOfDay { min_hour, max_hour } => {
                let hour = context.time_of_day;
                if min_hour <= max_hour {
                    hour >= *min_hour && hour <= *max_hour
                } else {
                    // Wrap around midnight
                    hour >= *min_hour || hour <= *max_hour
                }
            }
            Self::Cooldown { cooldown_ms } => {
                context.last_action_time
                    .map(|timestamp| {
                        let now = chrono::Utc::now().timestamp_millis();
                        let elapsed = (now - timestamp) as u64;
                        elapsed >= *cooldown_ms
                    })
                    .unwrap_or(true)
            }
            Self::CustomScript { .. } => {
                // Custom scripts would need a scripting engine
                // For now, always return true
                true
            }
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ActionContext {
    pub stats: HashMap<String, f64>,
    pub inventory: Vec<String>,
    pub item_quantities: HashMap<String, u32>,
    pub location: String,
    pub time_of_day: u8, // 0-23
    pub last_action_time: Option<i64>,
    pub custom_data: HashMap<String, String>,
}

impl ActionContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_stat(mut self, stat: String, value: f64) -> Self {
        self.stats.insert(stat, value);
        self
    }

    pub fn with_item(mut self, item_id: String, quantity: u32) -> Self {
        self.inventory.push(item_id.clone());
        self.item_quantities.insert(item_id, quantity);
        self
    }

    pub fn with_location(mut self, location: String) -> Self {
        self.location = location;
        self
    }

    pub fn with_time(mut self, hour: u8) -> Self {
        self.time_of_day = hour % 24;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConditionalAction {
    pub action_name: String,
    pub conditions: Vec<ActionCondition>,
    #[serde(default)]
    pub all_conditions_required: bool, // true = AND, false = OR
}

impl ConditionalAction {
    pub fn is_available(&self, context: &ActionContext) -> bool {
        if self.conditions.is_empty() {
            return true;
        }

        if self.all_conditions_required {
            // All conditions must be true (AND)
            self.conditions.iter().all(|cond| cond.evaluate(context))
        } else {
            // At least one condition must be true (OR)
            self.conditions.iter().any(|cond| cond.evaluate(context))
        }
    }

    pub fn evaluate_conditions(&self, context: &ActionContext) -> Vec<(usize, bool)> {
        self.conditions
            .iter()
            .enumerate()
            .map(|(idx, cond)| (idx, cond.evaluate(context)))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_min_stat_condition() {
        let cond = ActionCondition::MinStat {
            stat: "mana".into(),
            min_value: 50.0,
        };

        let mut context = ActionContext::new();
        assert!(!cond.evaluate(&context));

        context.stats.insert("mana".into(), 60.0);
        assert!(cond.evaluate(&context));

        context.stats.insert("mana".into(), 40.0);
        assert!(!cond.evaluate(&context));
    }

    #[test]
    fn test_has_item_condition() {
        let cond = ActionCondition::HasItem {
            item_id: "sword".into(),
            quantity: Some(2),
        };

        let context = ActionContext::new()
            .with_item("sword".into(), 3);

        assert!(cond.evaluate(&context));

        let context2 = ActionContext::new()
            .with_item("sword".into(), 1);

        assert!(!cond.evaluate(&context2));
    }

    #[test]
    fn test_time_of_day_condition() {
        let cond = ActionCondition::TimeOfDay {
            min_hour: 8,
            max_hour: 18,
        };

        let context = ActionContext::new().with_time(12);
        assert!(cond.evaluate(&context));

        let context2 = ActionContext::new().with_time(22);
        assert!(!cond.evaluate(&context2));
    }

    #[test]
    fn test_time_wrap_around() {
        // Night time: 22:00 to 6:00
        let cond = ActionCondition::TimeOfDay {
            min_hour: 22,
            max_hour: 6,
        };

        let context = ActionContext::new().with_time(23);
        assert!(cond.evaluate(&context));

        let context2 = ActionContext::new().with_time(3);
        assert!(cond.evaluate(&context2));

        let context3 = ActionContext::new().with_time(12);
        assert!(!cond.evaluate(&context3));
    }

    #[test]
    fn test_conditional_action_all_required() {
        let action = ConditionalAction {
            action_name: "cast_fireball".into(),
            conditions: vec![
                ActionCondition::MinStat {
                    stat: "mana".into(),
                    min_value: 30.0,
                },
                ActionCondition::HasItem {
                    item_id: "wand".into(),
                    quantity: None,
                },
            ],
            all_conditions_required: true,
        };

        let context = ActionContext::new()
            .with_stat("mana".into(), 50.0)
            .with_item("wand".into(), 1);

        assert!(action.is_available(&context));

        let context2 = ActionContext::new()
            .with_stat("mana".into(), 50.0);
        // Missing wand
        assert!(!action.is_available(&context2));
    }

    #[test]
    fn test_conditional_action_any_required() {
        let action = ConditionalAction {
            action_name: "attack".into(),
            conditions: vec![
                ActionCondition::HasItem {
                    item_id: "sword".into(),
                    quantity: None,
                },
                ActionCondition::HasItem {
                    item_id: "axe".into(),
                    quantity: None,
                },
            ],
            all_conditions_required: false,
        };

        let context = ActionContext::new()
            .with_item("sword".into(), 1);

        assert!(action.is_available(&context));

        let context2 = ActionContext::new()
            .with_item("axe".into(), 1);

        assert!(action.is_available(&context2));

        let context3 = ActionContext::new();
        assert!(!action.is_available(&context3));
    }
}
