use priority_queue::PriorityQueue;
use serde::{Deserialize, Serialize};
use std::cmp::Reverse;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::{RwLock, Semaphore};
use tracing::{debug, info};
use ulid::Ulid;

use npc_types::ChatContext;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Priority {
    Critical = 100,
    High = 75,
    Medium = 50,
    Low = 25,
    Background = 0,
}

impl Priority {
    pub fn calculate(
        distance_to_player: f64,
        is_active_conversation: bool,
        npc_importance: u8,
        conversation_depth: usize,
    ) -> Self {
        if is_active_conversation && conversation_depth > 0 {
            return Priority::Critical;
        }
        if distance_to_player < 5.0 && npc_importance >= 7 {
            return Priority::High;
        }
        if distance_to_player < 10.0 {
            return Priority::High;
        }
        if distance_to_player < 30.0 {
            return Priority::Medium;
        }
        if npc_importance >= 8 {
            return Priority::Medium;
        }
        if distance_to_player < 100.0 {
            return Priority::Low;
        }
        Priority::Background
    }
}

#[derive(Debug, Clone)]
pub struct NPCRequest {
    pub agent_id: String,
    pub user_id: String,
    pub message: String,
    pub context: ChatContext,
    pub priority: Priority,
    pub submitted_at: std::time::Instant,
}

impl NPCRequest {
    pub fn new(
        agent_id: String,
        user_id: String,
        message: String,
        context: ChatContext,
        priority: Priority,
    ) -> Self {
        Self {
            agent_id,
            user_id,
            message,
            context,
            priority,
            submitted_at: std::time::Instant::now(),
        }
    }

    pub fn wait_time(&self) -> std::time::Duration {
        self.submitted_at.elapsed()
    }
}

pub struct NPCPriorityQueue {
    queue: Arc<RwLock<PriorityQueue<String, Reverse<Priority>>>>,
    requests: Arc<RwLock<HashMap<String, NPCRequest>>>,
    concurrency_limit: Arc<Semaphore>,
    max_queue_size: usize,
    semaphore_capacity: usize,
    total_discarded: AtomicU64,
    total_processed: AtomicU64,
    max_wait_time_ms: AtomicU64,
    discarded_by_priority: Arc<RwLock<HashMap<Priority, u64>>>,
}

impl NPCPriorityQueue {
    pub fn new(concurrency_limit: usize, max_queue_size: usize) -> Self {
        Self {
            queue: Arc::new(RwLock::new(PriorityQueue::new())),
            requests: Arc::new(RwLock::new(HashMap::new())),
            concurrency_limit: Arc::new(Semaphore::new(concurrency_limit)),
            max_queue_size,
            semaphore_capacity: concurrency_limit,
            total_discarded: AtomicU64::new(0),
            total_processed: AtomicU64::new(0),
            max_wait_time_ms: AtomicU64::new(0),
            discarded_by_priority: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn submit(&self, request: NPCRequest) -> Result<(), String> {
        let mut queue = self.queue.write().await;
        let mut requests = self.requests.write().await;

        if queue.len() >= self.max_queue_size {
            if request.priority <= Priority::Low {
                self.total_discarded.fetch_add(1, Ordering::Relaxed);
                let mut discarded = self.discarded_by_priority.write().await;
                *discarded.entry(request.priority).or_insert(0) += 1;
                return Err("Queue full, request rejected".to_string());
            }
        }

        let request_id = format!("{}:{}", request.agent_id, Ulid::new());

        queue.push(request_id.clone(), Reverse(request.priority));
        requests.insert(request_id, request);

        Ok(())
    }

    pub async fn pop(&self) -> Option<NPCRequest> {
        let mut queue = self.queue.write().await;
        let mut requests = self.requests.write().await;

        if let Some((request_id, _)) = queue.pop() {
            if let Some(request) = requests.remove(&request_id) {
                let wait_ms = request.wait_time().as_millis() as u64;
                self.total_processed.fetch_add(1, Ordering::Relaxed);
                self.update_max_wait(wait_ms);
                return Some(request);
            }
        }

        None
    }

    fn update_max_wait(&self, wait_ms: u64) {
        let mut current = self.max_wait_time_ms.load(Ordering::Relaxed);
        while wait_ms > current {
            match self.max_wait_time_ms.compare_exchange_weak(current, wait_ms, Ordering::Relaxed, Ordering::Relaxed) {
                Ok(_) => break,
                Err(actual) => current = actual,
            }
        }
    }

    pub async fn acquire_permit(&self) -> tokio::sync::SemaphorePermit<'_> {
        self.concurrency_limit.acquire().await.unwrap()
    }

    pub async fn stats(&self) -> QueueStats {
        let queue = self.queue.read().await;
        let requests = self.requests.read().await;

        let mut by_priority = HashMap::new();
        for request in requests.values() {
            *by_priority.entry(request.priority).or_insert(0) += 1;
        }

        let avg_wait_time = if !requests.is_empty() {
            requests.values()
                .map(|r| r.wait_time().as_millis() as u64)
                .sum::<u64>() / requests.len() as u64
        } else {
            0
        };

        let available = self.concurrency_limit.available_permits();
        let capacity = self.semaphore_capacity;

        QueueStats {
            total: queue.len(),
            by_priority,
            avg_wait_time_ms: avg_wait_time,
            available_permits: available,
            max_wait_time_ms: self.max_wait_time_ms.load(Ordering::Relaxed),
            total_discarded: self.total_discarded.load(Ordering::Relaxed),
            discarded_by_priority: self.discarded_by_priority.read().await.clone(),
            total_processed: self.total_processed.load(Ordering::Relaxed),
            semaphore_capacity: capacity,
            semaphore_utilization: if capacity > 0 {
                (capacity - available) as f64 / capacity as f64
            } else {
                0.0
            },
        }
    }

    pub async fn clear_old_background(&self, max_age: std::time::Duration) {
        let mut queue = self.queue.write().await;
        let mut requests = self.requests.write().await;

        let old_size = queue.len();

        let to_remove: Vec<String> = requests
            .iter()
            .filter(|(_, req)| req.priority == Priority::Background && req.wait_time() > max_age)
            .map(|(id, _)| id.clone())
            .collect();

        for id in &to_remove {
            queue.remove(id);
            requests.remove(id);
        }

        if !to_remove.is_empty() {
            info!("Cleared {} old background tasks (queue: {} -> {})", to_remove.len(), old_size, queue.len());
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueStats {
    pub total: usize,
    pub by_priority: HashMap<Priority, usize>,
    pub avg_wait_time_ms: u64,
    pub available_permits: usize,
    pub max_wait_time_ms: u64,
    pub total_discarded: u64,
    pub discarded_by_priority: HashMap<Priority, u64>,
    pub total_processed: u64,
    pub semaphore_capacity: usize,
    pub semaphore_utilization: f64,
}

pub struct QueueCleanupTask {
    queue: Arc<NPCPriorityQueue>,
    interval: std::time::Duration,
    max_age: std::time::Duration,
}

impl QueueCleanupTask {
    pub fn new(queue: Arc<NPCPriorityQueue>) -> Self {
        Self {
            queue,
            interval: std::time::Duration::from_secs(60),
            max_age: std::time::Duration::from_secs(300),
        }
    }

    pub fn start(self) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            info!("Queue cleanup task started");
            let mut interval = tokio::time::interval(self.interval);

            loop {
                interval.tick().await;
                self.queue.clear_old_background(self.max_age).await;
                let stats = self.queue.stats().await;
                debug!("Queue stats: {:?}", stats);
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_priority_calculation() {
        assert_eq!(Priority::calculate(10.0, true, 5, 3), Priority::Critical);
        assert_eq!(Priority::calculate(3.0, false, 8, 0), Priority::High);
        assert_eq!(Priority::calculate(7.0, false, 5, 0), Priority::High);
        assert_eq!(Priority::calculate(25.0, false, 5, 0), Priority::Medium);
        assert_eq!(Priority::calculate(150.0, false, 5, 0), Priority::Background);
    }

    #[tokio::test]
    async fn test_queue_ordering() {
        let queue = NPCPriorityQueue::new(10, 100);

        let low = NPCRequest::new("npc1".into(), "user1".into(), "test".into(), ChatContext::default(), Priority::Low);
        let high = NPCRequest::new("npc2".into(), "user1".into(), "test".into(), ChatContext::default(), Priority::High);

        queue.submit(low).await.unwrap();
        queue.submit(high).await.unwrap();

        let first = queue.pop().await.unwrap();
        assert_eq!(first.priority, Priority::High);

        let second = queue.pop().await.unwrap();
        assert_eq!(second.priority, Priority::Low);
    }

    #[tokio::test]
    async fn test_queue_limit() {
        let queue = NPCPriorityQueue::new(10, 2);

        let req1 = NPCRequest::new("npc1".into(), "user1".into(), "test".into(), ChatContext::default(), Priority::Medium);
        let req2 = NPCRequest::new("npc2".into(), "user1".into(), "test".into(), ChatContext::default(), Priority::Medium);
        let req3_low = NPCRequest::new("npc3".into(), "user1".into(), "test".into(), ChatContext::default(), Priority::Low);

        assert!(queue.submit(req1).await.is_ok());
        assert!(queue.submit(req2).await.is_ok());
        assert!(queue.submit(req3_low).await.is_err());
    }

    #[tokio::test]
    async fn test_queue_discard_metrics() {
        let queue = NPCPriorityQueue::new(10, 1);

        let req1 = NPCRequest::new("npc1".into(), "user1".into(), "test".into(), ChatContext::default(), Priority::Medium);
        let req2_low = NPCRequest::new("npc2".into(), "user1".into(), "test".into(), ChatContext::default(), Priority::Low);

        assert!(queue.submit(req1).await.is_ok());
        assert!(queue.submit(req2_low).await.is_err());

        let stats = queue.stats().await;
        assert_eq!(stats.total_discarded, 1);
        assert_eq!(*stats.discarded_by_priority.get(&Priority::Low).unwrap_or(&0), 1);
    }

    #[tokio::test]
    async fn test_queue_processed_and_max_wait() {
        let queue = NPCPriorityQueue::new(10, 100);

        let req = NPCRequest::new("npc1".into(), "user1".into(), "test".into(), ChatContext::default(), Priority::High);
        queue.submit(req).await.unwrap();

        tokio::time::sleep(std::time::Duration::from_millis(10)).await;

        let popped = queue.pop().await.unwrap();
        assert_eq!(popped.agent_id, "npc1");

        let stats = queue.stats().await;
        assert_eq!(stats.total_processed, 1);
        assert!(stats.max_wait_time_ms >= 10);
    }
}
