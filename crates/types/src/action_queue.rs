use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};

use crate::{ActionPriority, AutonomousAction};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionQueue {
    pub agent_id: String,
    pub queue: VecDeque<QueuedAction>,
    pub max_size: usize,
    #[serde(skip)]
    pub current_action: Option<QueuedAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueuedAction {
    pub id: String,
    pub action: AutonomousAction,
    pub enqueued_at: i64,
    pub priority: ActionPriority,
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

impl ActionQueue {
    pub fn new(agent_id: String, max_size: usize) -> Self {
        Self {
            agent_id,
            queue: VecDeque::new(),
            max_size,
            current_action: None,
        }
    }

    pub fn enqueue(&mut self, action: QueuedAction) -> Result<(), String> {
        if self.queue.len() >= self.max_size {
            return Err("Queue is full".into());
        }

        // Insert based on priority (higher priority first)
        let insert_pos = self
            .queue
            .iter()
            .position(|a| a.priority < action.priority)
            .unwrap_or(self.queue.len());

        self.queue.insert(insert_pos, action);
        Ok(())
    }

    pub fn dequeue(&mut self) -> Option<QueuedAction> {
        self.queue.pop_front()
    }

    pub fn peek(&self) -> Option<&QueuedAction> {
        self.queue.front()
    }

    pub fn cancel_action(&mut self, action_id: &str) -> bool {
        if let Some(pos) = self.queue.iter().position(|a| a.id == action_id) {
            self.queue.remove(pos);
            return true;
        }
        false
    }

    pub fn clear(&mut self) {
        self.queue.clear();
        self.current_action = None;
    }

    pub fn len(&self) -> usize {
        self.queue.len()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    pub fn get_action(&self, action_id: &str) -> Option<&QueuedAction> {
        self.queue.iter().find(|a| a.id == action_id)
    }

    pub fn list_actions(&self) -> Vec<&QueuedAction> {
        self.queue.iter().collect()
    }

    pub fn reorder(&mut self) {
        let mut actions: Vec<_> = self.queue.drain(..).collect();
        actions.sort_by(|a, b| b.priority.cmp(&a.priority));
        self.queue = actions.into();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnqueueActionRequest {
    pub action: AutonomousAction,
    #[serde(default)]
    pub priority: ActionPriority,
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnqueueActionResponse {
    pub status: String,
    pub action_id: String,
    pub queue_position: usize,
    pub queue_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionQueueStatus {
    pub agent_id: String,
    pub queue_size: usize,
    pub max_size: usize,
    pub current_action: Option<QueuedAction>,
    pub next_actions: Vec<QueuedAction>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ActionType;

    fn create_test_action(id: &str, priority: ActionPriority) -> QueuedAction {
        QueuedAction {
            id: id.into(),
            action: AutonomousAction {
                action_type: ActionType::Wait,
                duration_ms: Some(1000),
                priority,
                ..Default::default()
            },
            enqueued_at: chrono::Utc::now().timestamp_millis(),
            priority,
            metadata: HashMap::new(),
        }
    }

    #[test]
    fn test_queue_creation() {
        let queue = ActionQueue::new("test-agent".into(), 10);
        assert_eq!(queue.len(), 0);
        assert!(queue.is_empty());
    }

    #[test]
    fn test_enqueue_dequeue() {
        let mut queue = ActionQueue::new("test-agent".into(), 10);

        let action = create_test_action("action-1", ActionPriority::Normal);
        assert!(queue.enqueue(action).is_ok());
        assert_eq!(queue.len(), 1);

        let dequeued = queue.dequeue();
        assert!(dequeued.is_some());
        assert_eq!(dequeued.unwrap().id, "action-1");
        assert!(queue.is_empty());
    }

    #[test]
    fn test_priority_ordering() {
        let mut queue = ActionQueue::new("test-agent".into(), 10);

        queue.enqueue(create_test_action("low", ActionPriority::Low)).unwrap();
        queue.enqueue(create_test_action("critical", ActionPriority::Critical)).unwrap();
        queue.enqueue(create_test_action("normal", ActionPriority::Normal)).unwrap();
        queue.enqueue(create_test_action("high", ActionPriority::High)).unwrap();

        assert_eq!(queue.dequeue().unwrap().id, "critical");
        assert_eq!(queue.dequeue().unwrap().id, "high");
        assert_eq!(queue.dequeue().unwrap().id, "normal");
        assert_eq!(queue.dequeue().unwrap().id, "low");
    }

    #[test]
    fn test_queue_full() {
        let mut queue = ActionQueue::new("test-agent".into(), 2);

        queue.enqueue(create_test_action("1", ActionPriority::Normal)).unwrap();
        queue.enqueue(create_test_action("2", ActionPriority::Normal)).unwrap();

        let result = queue.enqueue(create_test_action("3", ActionPriority::Normal));
        assert!(result.is_err());
    }

    #[test]
    fn test_cancel_action() {
        let mut queue = ActionQueue::new("test-agent".into(), 10);

        queue.enqueue(create_test_action("1", ActionPriority::Normal)).unwrap();
        queue.enqueue(create_test_action("2", ActionPriority::Normal)).unwrap();

        assert!(queue.cancel_action("1"));
        assert_eq!(queue.len(), 1);
        assert!(!queue.cancel_action("1")); // Already removed
    }
}
