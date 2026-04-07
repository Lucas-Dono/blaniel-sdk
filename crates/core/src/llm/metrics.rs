use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;

const DEFAULT_MAX_RECORDS: usize = 10_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestRecord {
    pub agent_id: String,
    pub user_id: String,
    pub provider: String,
    pub latency_ms: u64,
    pub cached: bool,
    pub success: bool,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMetricsSummary {
    pub agent_id: String,
    pub total_requests: u64,
    pub total_errors: u64,
    pub total_cached: u64,
    pub avg_latency_ms: f64,
    pub p50_latency_ms: u64,
    pub p99_latency_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserMetricsSummary {
    pub user_id: String,
    pub total_requests: u64,
    pub total_errors: u64,
    pub total_cached: u64,
    pub avg_latency_ms: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalMetricsSummary {
    pub total_requests: u64,
    pub total_errors: u64,
    pub total_cached: u64,
    pub cache_hit_rate: f64,
    pub avg_latency_ms: f64,
    pub error_rate: f64,
    pub active_agents: usize,
    pub active_users: usize,
}

struct IndexedRecords {
    timeline: VecDeque<RequestRecord>,
    total_latency: u64,
    total_errors: u64,
    total_cached: u64,
}

pub struct RequestMetricsCollector {
    records: Arc<RwLock<VecDeque<RequestRecord>>>,
    agent_index: Arc<DashMap<String, IndexedRecords>>,
    user_index: Arc<DashMap<String, IndexedRecords>>,
    max_records: usize,
    total_requests: AtomicU64,
    total_errors: AtomicU64,
    total_cached: AtomicU64,
    total_latency_ms: AtomicU64,
}

impl RequestMetricsCollector {
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_MAX_RECORDS)
    }

    pub fn with_capacity(max_records: usize) -> Self {
        Self {
            records: Arc::new(RwLock::new(VecDeque::with_capacity(max_records))),
            agent_index: Arc::new(DashMap::new()),
            user_index: Arc::new(DashMap::new()),
            max_records,
            total_requests: AtomicU64::new(0),
            total_errors: AtomicU64::new(0),
            total_cached: AtomicU64::new(0),
            total_latency_ms: AtomicU64::new(0),
        }
    }

    pub async fn record(&self, record: RequestRecord) {
        if record.cached {
            self.total_cached.fetch_add(1, Ordering::Relaxed);
        }
        if !record.success {
            self.total_errors.fetch_add(1, Ordering::Relaxed);
        }
        self.total_requests.fetch_add(1, Ordering::Relaxed);
        self.total_latency_ms
            .fetch_add(record.latency_ms, Ordering::Relaxed);

        {
            let agent_id = record.agent_id.clone();
            let user_id = record.user_id.clone();
            let lat = record.latency_ms;
            let success = record.success;
            let cached = record.cached;

            self.agent_index
                .entry(agent_id.clone())
                .and_modify(|e| {
                    if e.timeline.len() >= 1000 {
                        e.timeline.pop_front();
                    }
                    e.timeline.push_back(record.clone());
                    e.total_latency += lat;
                    if !success {
                        e.total_errors += 1;
                    }
                    if cached {
                        e.total_cached += 1;
                    }
                })
                .or_insert_with(|| {
                    let mut dq = VecDeque::with_capacity(64);
                    dq.push_back(record.clone());
                    IndexedRecords {
                        timeline: dq,
                        total_latency: lat,
                        total_errors: if !success { 1 } else { 0 },
                        total_cached: if cached { 1 } else { 0 },
                    }
                });

            self.user_index
                .entry(user_id)
                .and_modify(|e| {
                    if e.timeline.len() >= 500 {
                        e.timeline.pop_front();
                    }
                    e.timeline.push_back(record.clone());
                    e.total_latency += lat;
                    if !success {
                        e.total_errors += 1;
                    }
                    if cached {
                        e.total_cached += 1;
                    }
                })
                .or_insert_with(|| {
                    let mut dq = VecDeque::with_capacity(64);
                    dq.push_back(record);
                    IndexedRecords {
                        timeline: dq,
                        total_latency: lat,
                        total_errors: if !success { 1 } else { 0 },
                        total_cached: if cached { 1 } else { 0 },
                    }
                });
        }

        let mut records = self.records.write().await;
        if records.len() >= self.max_records {
            records.pop_front();
        }
        records.push_back(RequestRecord {
            agent_id: String::new(),
            user_id: String::new(),
            provider: String::new(),
            latency_ms: 0,
            cached: false,
            success: true,
            timestamp: 0,
        });
    }

    pub fn get_by_agent(&self, agent_id: &str) -> Vec<RequestRecord> {
        if let Some(entry) = self.agent_index.get(agent_id) {
            entry.timeline.iter().cloned().collect()
        } else {
            Vec::new()
        }
    }

    pub fn get_by_user(&self, user_id: &str) -> Vec<RequestRecord> {
        if let Some(entry) = self.user_index.get(user_id) {
            entry.timeline.iter().cloned().collect()
        } else {
            Vec::new()
        }
    }

    pub fn agent_summary(&self, agent_id: &str) -> Option<AgentMetricsSummary> {
        let entry = self.agent_index.get(agent_id)?;
        let records = &entry.timeline;
        if records.is_empty() {
            return None;
        }

        let total = records.len() as u64;
        let total_lat: u64 = records.iter().map(|r| r.latency_ms).sum();

        let mut latencies: Vec<u64> = records.iter().map(|r| r.latency_ms).collect();
        let p50_idx = (total as usize * 50 / 100).min(latencies.len().saturating_sub(1));
        let p99_idx = (total as usize * 99 / 100).min(latencies.len().saturating_sub(1));

        latencies.select_nth_unstable(p50_idx);
        let p50 = latencies[p50_idx];

        if p99_idx < latencies.len() {
            latencies.select_nth_unstable(p99_idx);
        }
        let p99 = latencies[p99_idx];

        Some(AgentMetricsSummary {
            agent_id: agent_id.to_string(),
            total_requests: total,
            total_errors: entry.total_errors,
            total_cached: entry.total_cached,
            avg_latency_ms: total_lat as f64 / total as f64,
            p50_latency_ms: p50,
            p99_latency_ms: p99,
        })
    }

    pub fn user_summary(&self, user_id: &str) -> Option<UserMetricsSummary> {
        let entry = self.user_index.get(user_id)?;
        let records = &entry.timeline;
        if records.is_empty() {
            return None;
        }

        let total = records.len() as u64;

        Some(UserMetricsSummary {
            user_id: user_id.to_string(),
            total_requests: total,
            total_errors: entry.total_errors,
            total_cached: entry.total_cached,
            avg_latency_ms: entry.total_latency as f64 / total as f64,
        })
    }

    pub fn global_summary(&self) -> GlobalMetricsSummary {
        let total = self.total_requests.load(Ordering::Relaxed);
        let errors = self.total_errors.load(Ordering::Relaxed);
        let cached = self.total_cached.load(Ordering::Relaxed);
        let total_lat = self.total_latency_ms.load(Ordering::Relaxed);

        GlobalMetricsSummary {
            total_requests: total,
            total_errors: errors,
            total_cached: cached,
            cache_hit_rate: if total > 0 {
                cached as f64 / total as f64
            } else {
                0.0
            },
            avg_latency_ms: if total > 0 {
                total_lat as f64 / total as f64
            } else {
                0.0
            },
            error_rate: if total > 0 {
                errors as f64 / total as f64
            } else {
                0.0
            },
            active_agents: self.agent_index.len(),
            active_users: self.user_index.len(),
        }
    }

    pub fn top_agents(&self, limit: usize) -> Vec<AgentMetricsSummary> {
        let mut summaries: Vec<AgentMetricsSummary> = self
            .agent_index
            .iter()
            .filter_map(|entry| {
                let records = &entry.value().timeline;
                if records.is_empty() {
                    return None;
                }
                let total = records.len() as u64;
                let total_lat: u64 = records.iter().map(|r| r.latency_ms).sum();

                let mut latencies: Vec<u64> = records.iter().map(|r| r.latency_ms).collect();
                let p50_idx = (total as usize * 50 / 100).min(latencies.len().saturating_sub(1));
                let p99_idx = (total as usize * 99 / 100).min(latencies.len().saturating_sub(1));

                latencies.select_nth_unstable(p50_idx);
                let p50 = latencies[p50_idx];

                if p99_idx < latencies.len() {
                    latencies.select_nth_unstable(p99_idx);
                }
                let p99 = latencies[p99_idx];

                Some(AgentMetricsSummary {
                    agent_id: entry.key().clone(),
                    total_requests: total,
                    total_errors: entry.value().total_errors,
                    total_cached: entry.value().total_cached,
                    avg_latency_ms: total_lat as f64 / total as f64,
                    p50_latency_ms: p50,
                    p99_latency_ms: p99,
                })
            })
            .collect();

        summaries.sort_by(|a, b| b.total_requests.cmp(&a.total_requests));
        summaries.truncate(limit);
        summaries
    }

    pub async fn recent_records(&self, limit: usize) -> Vec<RequestRecord> {
        let records = self.records.read().await;
        records.iter().rev().take(limit).cloned().collect()
    }

    pub async fn record_count(&self) -> usize {
        self.records.read().await.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_record(
        agent: &str,
        user: &str,
        provider: &str,
        latency: u64,
        success: bool,
        cached: bool,
    ) -> RequestRecord {
        RequestRecord {
            agent_id: agent.to_string(),
            user_id: user.to_string(),
            provider: provider.to_string(),
            latency_ms: latency,
            cached,
            success,
            timestamp: 1000,
        }
    }

    #[tokio::test]
    async fn test_record_and_query() {
        let collector = RequestMetricsCollector::with_capacity(100);

        collector
            .record(make_record("npc1", "user1", "openai", 100, true, false))
            .await;
        collector
            .record(make_record("npc1", "user2", "openai", 200, true, true))
            .await;
        collector
            .record(make_record("npc2", "user1", "groq", 50, true, false))
            .await;
        collector
            .record(make_record("npc1", "user1", "openai", 500, false, false))
            .await;

        assert_eq!(collector.total_requests.load(Ordering::Relaxed), 4);
        assert_eq!(collector.total_errors.load(Ordering::Relaxed), 1);
        assert_eq!(collector.total_cached.load(Ordering::Relaxed), 1);

        let agent_records = collector.get_by_agent("npc1");
        assert_eq!(agent_records.len(), 3);

        let user_records = collector.get_by_user("user1");
        assert_eq!(user_records.len(), 3);
    }

    #[tokio::test]
    async fn test_agent_summary() {
        let collector = RequestMetricsCollector::with_capacity(100);

        collector
            .record(make_record("npc1", "user1", "openai", 100, true, false))
            .await;
        collector
            .record(make_record("npc1", "user1", "openai", 200, true, true))
            .await;
        collector
            .record(make_record("npc1", "user1", "groq", 300, false, false))
            .await;

        let summary = collector.agent_summary("npc1").unwrap();
        assert_eq!(summary.total_requests, 3);
        assert_eq!(summary.total_errors, 1);
        assert_eq!(summary.total_cached, 1);
        assert!(summary.avg_latency_ms > 0.0);
        assert_eq!(summary.p50_latency_ms, 200);
    }

    #[tokio::test]
    async fn test_global_summary() {
        let collector = RequestMetricsCollector::with_capacity(100);

        collector
            .record(make_record("npc1", "user1", "openai", 100, true, false))
            .await;
        collector
            .record(make_record("npc2", "user1", "groq", 50, true, true))
            .await;

        let summary = collector.global_summary();
        assert_eq!(summary.total_requests, 2);
        assert_eq!(summary.active_agents, 2);
        assert_eq!(summary.active_users, 1);
        assert!((summary.cache_hit_rate - 0.5).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn test_capacity_limit() {
        let collector = RequestMetricsCollector::with_capacity(3);

        for i in 0..5 {
            collector
                .record(make_record("npc1", "user1", "openai", i, true, false))
                .await;
        }

        assert_eq!(collector.record_count().await, 3);
        assert_eq!(collector.total_requests.load(Ordering::Relaxed), 5);
    }

    #[tokio::test]
    async fn test_top_agents() {
        let collector = RequestMetricsCollector::with_capacity(100);

        for _ in 0..5 {
            collector
                .record(make_record("npc1", "user1", "openai", 100, true, false))
                .await;
        }
        for _ in 0..2 {
            collector
                .record(make_record("npc2", "user1", "openai", 100, true, false))
                .await;
        }

        let top = collector.top_agents(10);
        assert_eq!(top.len(), 2);
        assert_eq!(top[0].agent_id, "npc1");
        assert_eq!(top[0].total_requests, 5);
    }
}
