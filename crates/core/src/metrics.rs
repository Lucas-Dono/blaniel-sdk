use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use tracing::debug;

/// Detailed metrics for a single request
#[derive(Debug, Clone)]
pub struct RequestMetrics {
    pub agent_id: String,
    pub user_id: String,
    pub provider: String,
    pub latency_ms: u64,
    pub cached: bool,
    pub timestamp: Instant,
    pub success: bool,
    pub error_type: Option<String>,
}

impl RequestMetrics {
    pub fn new(
        agent_id: String,
        user_id: String,
        provider: String,
        latency_ms: u64,
        cached: bool,
    ) -> Self {
        Self {
            agent_id,
            user_id,
            provider,
            latency_ms,
            cached,
            timestamp: Instant::now(),
            success: true,
            error_type: None,
        }
    }

    pub fn with_error(mut self, error_type: String) -> Self {
        self.success = false;
        self.error_type = Some(error_type);
        self
    }
}

/// Aggregated metrics per agent
#[derive(Debug, Clone, Default)]
pub struct AgentMetrics {
    pub total_requests: u64,
    pub total_errors: u64,
    pub avg_latency_ms: f64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub providers_used: HashMap<String, u64>,
}

impl AgentMetrics {
    fn record(&mut self, metric: &RequestMetrics) {
        self.total_requests += 1;

        if !metric.success {
            self.total_errors += 1;
        }

        if metric.cached {
            self.cache_hits += 1;
        } else {
            self.cache_misses += 1;
        }

        // Update average latency with exponential moving average
        let alpha = 0.3;
        self.avg_latency_ms = alpha * metric.latency_ms as f64 + (1.0 - alpha) * self.avg_latency_ms;

        // Track provider usage
        *self.providers_used.entry(metric.provider.clone()).or_insert(0) += 1;
    }

    pub fn cache_hit_rate(&self) -> f64 {
        let total = self.cache_hits + self.cache_misses;
        if total == 0 {
            return 0.0;
        }
        (self.cache_hits as f64 / total as f64) * 100.0
    }

    pub fn error_rate(&self) -> f64 {
        if self.total_requests == 0 {
            return 0.0;
        }
        (self.total_errors as f64 / self.total_requests as f64) * 100.0
    }
}

/// Aggregated metrics per user
#[derive(Debug, Clone, Default)]
pub struct UserMetrics {
    pub total_requests: u64,
    pub agents_interacted: HashMap<String, u64>,
    pub avg_latency_ms: f64,
}

impl UserMetrics {
    fn record(&mut self, metric: &RequestMetrics) {
        self.total_requests += 1;

        // Track agent interactions
        *self.agents_interacted.entry(metric.agent_id.clone()).or_insert(0) += 1;

        // Update average latency
        let alpha = 0.3;
        self.avg_latency_ms = alpha * metric.latency_ms as f64 + (1.0 - alpha) * self.avg_latency_ms;
    }
}

/// Global metrics tracker
pub struct MetricsTracker {
    agent_metrics: Arc<RwLock<HashMap<String, AgentMetrics>>>,
    user_metrics: Arc<RwLock<HashMap<String, UserMetrics>>>,
    recent_requests: Arc<RwLock<VecDeque<RequestMetrics>>>,
    max_recent: usize,
}

impl MetricsTracker {
    pub fn new(max_recent: usize) -> Self {
        Self {
            agent_metrics: Arc::new(RwLock::new(HashMap::new())),
            user_metrics: Arc::new(RwLock::new(HashMap::new())),
            recent_requests: Arc::new(RwLock::new(VecDeque::new())),
            max_recent,
        }
    }

    /// Record a new request metric
    pub async fn record(&self, metric: RequestMetrics) {
        debug!(
            "Recording metric: agent={}, user={}, provider={}, latency={}ms, success={}",
            metric.agent_id, metric.user_id, metric.provider, metric.latency_ms, metric.success
        );

        // Update agent metrics
        let mut agent_metrics = self.agent_metrics.write().await;
        agent_metrics
            .entry(metric.agent_id.clone())
            .or_default()
            .record(&metric);
        drop(agent_metrics);

        // Update user metrics
        let mut user_metrics = self.user_metrics.write().await;
        user_metrics
            .entry(metric.user_id.clone())
            .or_default()
            .record(&metric);
        drop(user_metrics);

        // Store in recent requests (with size limit)
        let mut recent = self.recent_requests.write().await;
        recent.push_back(metric);
        while recent.len() > self.max_recent {
            recent.pop_front();
        }
    }

    /// Get metrics for a specific agent
    pub async fn get_agent_metrics(&self, agent_id: &str) -> Option<AgentMetrics> {
        let metrics = self.agent_metrics.read().await;
        metrics.get(agent_id).cloned()
    }

    /// Get metrics for a specific user
    pub async fn get_user_metrics(&self, user_id: &str) -> Option<UserMetrics> {
        let metrics = self.user_metrics.read().await;
        metrics.get(user_id).cloned()
    }

    /// Get all agent metrics
    pub async fn get_all_agent_metrics(&self) -> HashMap<String, AgentMetrics> {
        self.agent_metrics.read().await.clone()
    }

    /// Get all user metrics
    pub async fn get_all_user_metrics(&self) -> HashMap<String, UserMetrics> {
        self.user_metrics.read().await.clone()
    }

    /// Get recent requests
    pub async fn get_recent_requests(&self, limit: usize) -> Vec<RequestMetrics> {
        let recent = self.recent_requests.read().await;
        let start = recent.len().saturating_sub(limit);
        recent.range(start..).cloned().collect()
    }

    /// Get top agents by request count
    pub async fn get_top_agents(&self, limit: usize) -> Vec<(String, u64)> {
        let metrics = self.agent_metrics.read().await;
        let mut agents: Vec<(String, u64)> = metrics
            .iter()
            .map(|(id, m)| (id.clone(), m.total_requests))
            .collect();
        agents.sort_by(|a, b| b.1.cmp(&a.1));
        agents.truncate(limit);
        agents
    }

    /// Get top users by request count
    pub async fn get_top_users(&self, limit: usize) -> Vec<(String, u64)> {
        let metrics = self.user_metrics.read().await;
        let mut users: Vec<(String, u64)> = metrics
            .iter()
            .map(|(id, m)| (id.clone(), m.total_requests))
            .collect();
        users.sort_by(|a, b| b.1.cmp(&a.1));
        users.truncate(limit);
        users
    }

    /// Get global statistics
    pub async fn get_global_stats(&self) -> GlobalStats {
        let agent_metrics = self.agent_metrics.read().await;
        let user_metrics = self.user_metrics.read().await;
        let recent = self.recent_requests.read().await;

        let total_requests: u64 = agent_metrics.values().map(|m| m.total_requests).sum();
        let total_errors: u64 = agent_metrics.values().map(|m| m.total_errors).sum();
        let total_cache_hits: u64 = agent_metrics.values().map(|m| m.cache_hits).sum();
        let total_cache_misses: u64 = agent_metrics.values().map(|m| m.cache_misses).sum();

        let avg_latency_ms = if !agent_metrics.is_empty() {
            agent_metrics.values().map(|m| m.avg_latency_ms).sum::<f64>() / agent_metrics.len() as f64
        } else {
            0.0
        };

        GlobalStats {
            total_requests,
            total_errors,
            total_agents: agent_metrics.len(),
            total_users: user_metrics.len(),
            avg_latency_ms,
            cache_hit_rate: if total_requests > 0 {
                (total_cache_hits as f64 / (total_cache_hits + total_cache_misses) as f64) * 100.0
            } else {
                0.0
            },
            error_rate: if total_requests > 0 {
                (total_errors as f64 / total_requests as f64) * 100.0
            } else {
                0.0
            },
            recent_requests_count: recent.len(),
        }
    }
}

impl Default for MetricsTracker {
    fn default() -> Self {
        Self::new(1000)
    }
}

#[derive(Debug, Clone)]
pub struct GlobalStats {
    pub total_requests: u64,
    pub total_errors: u64,
    pub total_agents: usize,
    pub total_users: usize,
    pub avg_latency_ms: f64,
    pub cache_hit_rate: f64,
    pub error_rate: f64,
    pub recent_requests_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_metrics_recording() {
        let tracker = MetricsTracker::new(100);

        let metric = RequestMetrics::new(
            "agent1".to_string(),
            "user1".to_string(),
            "venice".to_string(),
            15,
            false,
        );

        tracker.record(metric).await;

        let agent_metrics = tracker.get_agent_metrics("agent1").await.unwrap();
        assert_eq!(agent_metrics.total_requests, 1);
        assert_eq!(agent_metrics.cache_misses, 1);

        let user_metrics = tracker.get_user_metrics("user1").await.unwrap();
        assert_eq!(user_metrics.total_requests, 1);
    }

    #[tokio::test]
    async fn test_cache_hit_rate() {
        let tracker = MetricsTracker::new(100);

        tracker.record(RequestMetrics::new("a1".into(), "u1".into(), "p1".into(), 10, true)).await;
        tracker.record(RequestMetrics::new("a1".into(), "u1".into(), "p1".into(), 10, true)).await;
        tracker.record(RequestMetrics::new("a1".into(), "u1".into(), "p1".into(), 10, false)).await;

        let metrics = tracker.get_agent_metrics("a1").await.unwrap();
        assert_eq!(metrics.cache_hit_rate(), 66.66666666666666);
    }

    #[tokio::test]
    async fn test_top_agents() {
        let tracker = MetricsTracker::new(100);

        for i in 0..5 {
            tracker.record(RequestMetrics::new("a1".into(), "u1".into(), "p1".into(), 10, false)).await;
        }
        for i in 0..3 {
            tracker.record(RequestMetrics::new("a2".into(), "u1".into(), "p1".into(), 10, false)).await;
        }

        let top = tracker.get_top_agents(2).await;
        assert_eq!(top[0].0, "a1");
        assert_eq!(top[0].1, 5);
        assert_eq!(top[1].0, "a2");
        assert_eq!(top[1].1, 3);
    }
}
