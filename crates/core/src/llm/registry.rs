use arc_swap::ArcSwap;
use secrecy::Secret;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use tracing::{debug, info, warn};

use npc_types::{
    ChatContext, ChatResponse, LLMProviderConfig, LLMProviderStatus, LLMProviderType,
    StreamChunk,
};

use crate::llm::client::{LLMError, LLMResult, UniversalLLMClient};
use crate::llm::metrics::{RequestMetricsCollector, RequestRecord};

const MAX_PROVIDERS: usize = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailoverStrategy {
    Priority,
    RoundRobin,
    LeastLatency,
    Random,
}

pub struct LLMProviderRegistry {
    clients: ArcSwap<HashMap<String, UniversalLLMClient>>,
    strategy: FailoverStrategy,
    counter: AtomicUsize,
    metrics: RequestMetricsCollector,
}

impl LLMProviderRegistry {
    pub fn new(strategy: FailoverStrategy) -> Self {
        Self {
            clients: ArcSwap::from_pointee(HashMap::new()),
            strategy,
            counter: AtomicUsize::new(0),
            metrics: RequestMetricsCollector::new(),
        }
    }

    pub fn metrics(&self) -> &RequestMetricsCollector {
        &self.metrics
    }

    pub async fn register(&self, config: LLMProviderConfig) -> Result<(), String> {
        let current = self.clients.load();
        if current.len() >= MAX_PROVIDERS {
            return Err(format!(
                "Maximum number of providers ({}) reached",
                MAX_PROVIDERS
            ));
        }

        let name = config.name.clone();
        let client = UniversalLLMClient::new(config)
            .map_err(|e| format!("Failed to create client '{}': {}", name, e))?;

        info!("Registered LLM provider: {} ({})", name, client.name());

        self.clients.rcu(|existing| {
            let mut updated = (**existing).clone();
            updated.insert(name.clone(), client.clone());
            updated
        });

        Ok(())
    }

    pub async fn unregister(&self, name: &str) -> bool {
        let found = self.clients.load().contains_key(name);
        if found {
            let name_owned = name.to_string();
            self.clients.rcu(|existing| {
                let mut updated = (**existing).clone();
                updated.remove(&name_owned);
                updated
            });
            info!("Unregistered LLM provider: {}", name);
        }
        found
    }

    pub async fn get(&self, name: &str) -> Option<UniversalLLMClient> {
        self.clients.load().get(name).cloned()
    }

    pub async fn list_providers(&self) -> Vec<LLMProviderStatus> {
        let clients: Vec<_> = self.clients.load().values().cloned().collect();
        let futures: Vec<_> = clients.iter().map(|c| c.get_status()).collect();
        let mut statuses = futures::future::join_all(futures).await;
        statuses.sort_by(|a, b| a.priority.cmp(&b.priority));
        statuses
    }

    pub async fn provider_count(&self) -> usize {
        self.clients.load().len()
    }

    pub async fn enabled_count(&self) -> usize {
        let clients = self.clients.load();
        let mut count = 0;
        for client in clients.values() {
            if client.is_enabled() && client.is_healthy() {
                count += 1;
            }
        }
        count
    }

    pub async fn chat(
        &self,
        agent_id: &str,
        user_id: &str,
        message: &str,
        context: ChatContext,
        system_prompt: Option<&str>,
        ai_context: Option<&str>,
    ) -> Result<ChatResponse, LLMError> {
        match self.strategy {
            FailoverStrategy::Priority => {
                self.chat_priority(agent_id, user_id, message, context, system_prompt, ai_context)
                    .await
            }
            FailoverStrategy::RoundRobin => {
                self.chat_round_robin(agent_id, user_id, message, context, system_prompt, ai_context)
                    .await
            }
            FailoverStrategy::LeastLatency => {
                self.chat_least_latency(agent_id, user_id, message, context, system_prompt, ai_context)
                    .await
            }
            FailoverStrategy::Random => {
                self.chat_random(agent_id, user_id, message, context, system_prompt, ai_context)
                    .await
            }
        }
    }

    pub async fn chat_stream(
        &self,
        agent_id: &str,
        user_id: &str,
        message: &str,
        context: ChatContext,
        system_prompt: Option<&str>,
        ai_context: Option<&str>,
    ) -> Result<tokio_stream::wrappers::ReceiverStream<LLMResult<StreamChunk>>, LLMError> {
        let ordered = self.get_ordered_clients();

        for client in ordered {
            if !client.is_enabled() || !client.is_healthy() {
                continue;
            }
            if !client.config_ref().provider_type.is_openai_compatible() {
                continue;
            }

            match client
                .chat_stream(agent_id, user_id, message, context.clone(), system_prompt, ai_context)
                .await
            {
                Ok(stream) => {
                    debug!("Streaming via provider '{}'", client.name());
                    return Ok(stream);
                }
                Err(e) => {
                    warn!("Stream failed for '{}': {}, trying next", client.name(), e);
                }
            }
        }

        Err(LLMError::Unhealthy("No streaming-capable providers available".into()))
    }

    async fn chat_priority(
        &self,
        agent_id: &str,
        user_id: &str,
        message: &str,
        context: ChatContext,
        system_prompt: Option<&str>,
        ai_context: Option<&str>,
    ) -> Result<ChatResponse, LLMError> {
        let ordered = self.get_ordered_clients();

        let mut last_error = None;
        for client in ordered {
            if !client.is_enabled() {
                continue;
            }

            match client
                .chat_with_retry(agent_id, user_id, message, context.clone(), system_prompt, ai_context)
                .await
            {
                Ok(response) => {
                    debug!("Provider '{}' succeeded (priority mode)", client.name());
                    self.record_success(agent_id, user_id, &response).await;
                    return Ok(response);
                }
                Err(e) => {
                    warn!("Provider '{}' failed, trying next: {}", client.name(), e);
                    self.record_failure(agent_id, user_id, client.name()).await;
                    last_error = Some(e);
                }
            }
        }

        Err(last_error.unwrap_or(LLMError::Unhealthy("No providers available".into())))
    }

    async fn chat_round_robin(
        &self,
        agent_id: &str,
        user_id: &str,
        message: &str,
        context: ChatContext,
        system_prompt: Option<&str>,
        ai_context: Option<&str>,
    ) -> Result<ChatResponse, LLMError> {
        let enabled = self.get_enabled_clients();
        if enabled.is_empty() {
            return Err(LLMError::Unhealthy("No enabled providers".into()));
        }

        let idx = self.counter.fetch_add(1, Ordering::Relaxed) % enabled.len();
        let client = &enabled[idx];

        match client
            .chat_with_retry(agent_id, user_id, message, context.clone(), system_prompt, ai_context)
            .await
        {
            Ok(response) => {
                self.record_success(agent_id, user_id, &response).await;
                Ok(response)
            }
            Err(e) => {
                warn!("Round-robin provider '{}' failed: {}", client.name(), e);
                self.record_failure(agent_id, user_id, &client.name().to_string()).await;
                for fallback in &enabled {
                    if fallback.name() == client.name() {
                        continue;
                    }
                    match fallback
                        .chat_with_retry(agent_id, user_id, message, context.clone(), system_prompt, ai_context)
                        .await
                    {
                        Ok(response) => {
                            self.record_success(agent_id, user_id, &response).await;
                            return Ok(response);
                        }
                        Err(fe) => {
                            warn!("Fallback '{}' also failed: {}", fallback.name(), fe);
                            self.record_failure(agent_id, user_id, fallback.name()).await;
                        }
                    }
                }
                Err(e)
            }
        }
    }

    async fn chat_least_latency(
        &self,
        agent_id: &str,
        user_id: &str,
        message: &str,
        context: ChatContext,
        system_prompt: Option<&str>,
        ai_context: Option<&str>,
    ) -> Result<ChatResponse, LLMError> {
        let clients = self.get_enabled_clients();

        let status_futures: Vec<_> = clients.iter().map(|c| c.get_status()).collect();
        let statuses_raw = futures::future::join_all(status_futures).await;

        let mut statuses: Vec<_> = clients
            .into_iter()
            .zip(statuses_raw.into_iter())
            .filter(|(_, s)| s.healthy)
            .map(|(c, s)| (c, s.avg_latency_ms))
            .collect();

        statuses.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        let mut last_error = None;
        for (client, avg_lat) in statuses {
            debug!("Trying provider '{}' (avg latency: {:.1}ms)", client.name(), avg_lat);
            match client
                .chat_with_retry(agent_id, user_id, message, context.clone(), system_prompt, ai_context)
                .await
            {
                Ok(response) => {
                    self.record_success(agent_id, user_id, &response).await;
                    return Ok(response);
                }
                Err(e) => {
                    self.record_failure(agent_id, user_id, client.name()).await;
                    last_error = Some(e);
                }
            }
        }

        Err(last_error.unwrap_or(LLMError::Unhealthy("No healthy providers".into())))
    }

    async fn chat_random(
        &self,
        agent_id: &str,
        user_id: &str,
        message: &str,
        context: ChatContext,
        system_prompt: Option<&str>,
        ai_context: Option<&str>,
    ) -> Result<ChatResponse, LLMError> {
        let mut enabled = self.get_enabled_clients();
        if enabled.is_empty() {
            return Err(LLMError::Unhealthy("No enabled providers".into()));
        }

        use rand::seq::SliceRandom;
        enabled.shuffle(&mut rand::thread_rng());

        let mut last_error = None;
        for client in &enabled {
            match client
                .chat_with_retry(agent_id, user_id, message, context.clone(), system_prompt, ai_context)
                .await
            {
                Ok(response) => {
                    self.record_success(agent_id, user_id, &response).await;
                    return Ok(response);
                }
                Err(e) => {
                    self.record_failure(agent_id, user_id, client.name()).await;
                    last_error = Some(e);
                }
            }
        }

        Err(last_error.unwrap_or(LLMError::Unhealthy("All providers failed".into())))
    }

    async fn record_success(&self, agent_id: &str, user_id: &str, response: &ChatResponse) {
        self.metrics.record(RequestRecord {
            agent_id: agent_id.to_string(),
            user_id: user_id.to_string(),
            provider: response.source.clone(),
            latency_ms: response.latency_ms,
            cached: response.cached.unwrap_or(false),
            success: true,
            timestamp: chrono::Utc::now().timestamp(),
        }).await;
    }

    async fn record_failure(&self, agent_id: &str, user_id: &str, provider: &str) {
        self.metrics.record(RequestRecord {
            agent_id: agent_id.to_string(),
            user_id: user_id.to_string(),
            provider: provider.to_string(),
            latency_ms: 0,
            cached: false,
            success: false,
            timestamp: chrono::Utc::now().timestamp(),
        }).await;
    }

    fn get_ordered_clients(&self) -> Vec<UniversalLLMClient> {
        let clients = self.clients.load();
        let mut list: Vec<_> = clients.values().cloned().collect();
        list.sort_by_key(|c| c.priority());
        list
    }

    fn get_enabled_clients(&self) -> Vec<UniversalLLMClient> {
        let clients = self.clients.load();
        let mut list: Vec<_> = clients
            .values()
            .filter(|c| c.is_enabled())
            .cloned()
            .collect();
        list.sort_by_key(|c| c.priority());
        list
    }

    pub fn load_from_env() -> Self {
        let strategy = match std::env::var("LLM_FAILOVER_STRATEGY").as_deref() {
            Ok("round_robin") => FailoverStrategy::RoundRobin,
            Ok("least_latency") => FailoverStrategy::LeastLatency,
            Ok("random") => FailoverStrategy::Random,
            _ => FailoverStrategy::Priority,
        };

        Self::new(strategy)
    }

    pub async fn init_from_env(&self) -> Result<usize, String> {
        let mut count = 0;

        let providers: Vec<(&str, LLMProviderType, &str, &str)> = vec![
            ("OPENAI", LLMProviderType::OpenAI, "https://api.openai.com/v1", "gpt-4o-mini"),
            ("ANTHROPIC", LLMProviderType::Anthropic, "https://api.anthropic.com/v1", "claude-sonnet-4-20250514"),
            ("GEMINI", LLMProviderType::Gemini, "https://generativelanguage.googleapis.com/v1beta", "gemini-2.0-flash"),
            ("XAI", LLMProviderType::XAi, "https://api.x.ai/v1", "grok-3-mini-fast"),
            ("PERPLEXITY", LLMProviderType::Perplexity, "https://api.perplexity.ai", "sonar"),
            ("VENICE", LLMProviderType::Venice, "https://api.venice.ai/api/v1", "venice-uncensored-role-play"),
            ("TOGETHER", LLMProviderType::Together, "https://api.together.xyz/v1", "meta-llama/Llama-3.3-70B-Instruct-Turbo"),
            ("GROQ", LLMProviderType::Groq, "https://api.groq.com/openai/v1", "llama-3.3-70b-versatile"),
            ("MISTRAL", LLMProviderType::Mistral, "https://api.mistral.ai/v1", "mistral-small-latest"),
            ("DEEPSEEK", LLMProviderType::DeepSeek, "https://api.deepseek.com/v1", "deepseek-chat"),
            ("FIREWORKS", LLMProviderType::Fireworks, "https://api.fireworks.ai/inference/v1", "accounts/fireworks/models/llama-v3-8b-instruct"),
            ("OPENROUTER", LLMProviderType::OpenRouter, "https://openrouter.ai/api/v1", "meta-llama/llama-3.3-70b-instruct"),
            ("OLLAMA", LLMProviderType::Ollama, "http://localhost:11434/v1", "llama3"),
        ];

        for (prefix, provider_type, default_url, default_model) in providers {
            let api_key = match std::env::var(format!("{}_API_KEY", prefix)) {
                Ok(k) if !k.is_empty() => k,
                _ => continue,
            };

            let base_url = std::env::var(format!("{}_BASE_URL", prefix))
                .unwrap_or_else(|_| default_url.to_string());
            let model = std::env::var(format!("{}_MODEL", prefix))
                .unwrap_or_else(|_| default_model.to_string());
            let max_tokens = std::env::var(format!("{}_MAX_TOKENS", prefix))
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(150);
            let temperature = std::env::var(format!("{}_TEMPERATURE", prefix))
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.8);
            let timeout = std::env::var(format!("{}_TIMEOUT_MS", prefix))
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(30000);
            let priority = std::env::var(format!("{}_PRIORITY", prefix))
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(count as u32);

            let config = LLMProviderConfig {
                name: prefix.to_lowercase(),
                provider_type,
                base_url,
                api_key: Secret::new(api_key),
                model,
                max_tokens,
                temperature,
                system_prompt: std::env::var(format!("{}_SYSTEM_PROMPT", prefix)).ok(),
                extra_headers: HashMap::new(),
                enabled: true,
                priority,
                timeout_ms: timeout,
            };

            self.register(config).await?;
            count += 1;
        }

        if count == 0 {
            warn!("No LLM providers configured via environment variables");
        } else {
            info!("Initialized {} LLM provider(s) from environment", count);
        }

        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn make_config(name: &str, priority: u32) -> LLMProviderConfig {
        LLMProviderConfig {
            name: name.into(),
            provider_type: LLMProviderType::OpenAI,
            base_url: "https://api.example.com/v1".into(),
            api_key: Secret::new("test-key".into()),
            model: "test-model".into(),
            max_tokens: 150,
            temperature: 0.8,
            system_prompt: None,
            extra_headers: HashMap::new(),
            enabled: true,
            priority,
            timeout_ms: 30000,
        }
    }

    #[tokio::test]
    async fn test_register_and_list() {
        let registry = LLMProviderRegistry::new(FailoverStrategy::Priority);

        registry.register(make_config("provider_a", 0)).await.unwrap();
        registry.register(make_config("provider_b", 1)).await.unwrap();

        assert_eq!(registry.provider_count().await, 2);
        assert!(registry.get("provider_a").await.is_some());
        assert!(registry.get("nonexistent").await.is_none());
    }

    #[tokio::test]
    async fn test_unregister() {
        let registry = LLMProviderRegistry::new(FailoverStrategy::Priority);

        registry.register(make_config("provider_a", 0)).await.unwrap();
        assert!(registry.unregister("provider_a").await);
        assert_eq!(registry.provider_count().await, 0);
    }

    #[tokio::test]
    async fn test_list_ordered_by_priority() {
        let registry = LLMProviderRegistry::new(FailoverStrategy::Priority);

        registry.register(make_config("low", 10)).await.unwrap();
        registry.register(make_config("high", 0)).await.unwrap();

        let list = registry.list_providers().await;
        assert_eq!(list[0].name, "high");
        assert_eq!(list[1].name, "low");
    }

    #[tokio::test]
    async fn test_max_providers_limit() {
        let registry = LLMProviderRegistry::new(FailoverStrategy::Priority);

        for i in 0..MAX_PROVIDERS {
            registry
                .register(make_config(&format!("p{}", i), i as u32))
                .await
                .unwrap();
        }

        let result = registry.register(make_config("overflow", 99)).await;
        assert!(result.is_err());
    }
}
