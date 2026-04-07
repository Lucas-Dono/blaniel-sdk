use lru::LruCache;
use reqwest::{header, Client};
use secrecy::ExposeSecret;
use serde::Deserialize;
use serde_json::json;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::num::NonZeroUsize;
use std::sync::atomic::{AtomicU32, AtomicU64, AtomicI64, Ordering};
use std::sync::{Arc, RwLock as StdRwLock};
use thiserror::Error;
use tracing::{debug, error, warn};
use futures::stream::StreamExt;

use npc_types::{ChatContext, ChatResponse, LLMProviderConfig, LLMProviderStatus, LLMProviderType, StreamChunk};

const MAX_RETRIES: u32 = 3;
const INITIAL_BACKOFF_MS: u64 = 100;
const MAX_BACKOFF_MS: u64 = 2000;
const SYSTEM_PROMPT_CACHE_SIZE: usize = 256;
const CIRCUIT_BREAKER_THRESHOLD: u32 = 5;
const CIRCUIT_BREAKER_RESET_SECS: u64 = 30;
const DEFAULT_MAX_RPM: u32 = 60;

struct TokenBucket {
    tokens: AtomicU32,
    max_tokens: u32,
    last_refill: AtomicI64,
    refill_interval_ms: u64,
}

impl TokenBucket {
    fn new(max_rpm: u32) -> Self {
        let refill_interval_ms = if max_rpm > 0 { 60_000 / max_rpm as u64 } else { 0 };
        Self {
            tokens: AtomicU32::new(max_rpm),
            max_tokens: max_rpm,
            last_refill: AtomicI64::new(chrono::Utc::now().timestamp_millis()),
            refill_interval_ms,
        }
    }

    fn try_acquire(&self) -> bool {
        if self.max_tokens == 0 {
            return true;
        }
        self.refill();
        loop {
            let current = self.tokens.load(Ordering::Relaxed);
            if current == 0 {
                return false;
            }
            if self.tokens.compare_exchange_weak(
                current,
                current - 1,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ).is_ok() {
                return true;
            }
        }
    }

    fn refill(&self) {
        let now = chrono::Utc::now().timestamp_millis();
        let last = self.last_refill.load(Ordering::Relaxed);
        let elapsed = now - last;
        if elapsed < self.refill_interval_ms as i64 {
            return;
        }
        let tokens_to_add = (elapsed as u64 / self.refill_interval_ms) as u32;
        if tokens_to_add == 0 {
            return;
        }
        if self.last_refill.compare_exchange(
            last,
            now,
            Ordering::Relaxed,
            Ordering::Relaxed,
        ).is_ok() {
            let current = self.tokens.load(Ordering::Relaxed);
            let new_count = (current + tokens_to_add).min(self.max_tokens);
            self.tokens.store(new_count, Ordering::Relaxed);
        }
    }
}

#[derive(Debug, Error, Clone)]
pub enum LLMError {
    #[error("HTTP request failed: {0}")]
    Request(String),

    #[error("Request failed with status {0}: {1}")]
    RequestFailed(u16, String),

    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    #[error("Rate limited: retry after {0:?}")]
    RateLimited(Option<std::time::Duration>),

    #[error("Provider unhealthy: {0}")]
    Unhealthy(String),

    #[error("No content in response")]
    EmptyResponse,

    #[error("Timeout after {0:?}")]
    Timeout(std::time::Duration),
}

pub type LLMResult<T> = Result<T, LLMError>;

#[derive(Debug, Deserialize)]
struct OpenAIChatResponse {
    #[serde(default)]
    choices: Vec<OpenAIChoice>,
    #[serde(default)]
    error: Option<OpenAIError>,
    #[serde(default)]
    usage: Option<OpenAIUsage>,
}

#[derive(Debug, Deserialize)]
struct OpenAIChoice {
    message: OpenAIMessage,
    #[serde(default)]
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAIMessage {
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAIError {
    message: String,
    #[serde(rename = "type", default)]
    error_type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAIUsage {
    prompt_tokens: Option<u32>,
    completion_tokens: Option<u32>,
    total_tokens: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    content: Vec<AnthropicContentBlock>,
    #[serde(default)]
    usage: Option<AnthropicUsage>,
}

#[derive(Debug, Deserialize)]
struct AnthropicContentBlock {
    #[serde(rename = "type")]
    block_type: String,
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AnthropicUsage {
    input_tokens: Option<u32>,
    output_tokens: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct GeminiResponse {
    candidates: Vec<GeminiCandidate>,
}

#[derive(Debug, Deserialize)]
struct GeminiCandidate {
    content: GeminiContent,
}

#[derive(Debug, Deserialize)]
struct GeminiContent {
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Deserialize)]
struct GeminiPart {
    text: Option<String>,
}

struct CircuitState {
    open: bool,
    opened_at: Option<std::time::Instant>,
}

struct ProviderMetrics {
    total_requests: AtomicU64,
    total_errors: AtomicU64,
    consecutive_errors: AtomicU32,
    avg_latency_ms: AtomicU64,
    circuit: StdRwLock<CircuitState>,
}

impl ProviderMetrics {
    fn new() -> Self {
        Self {
            total_requests: AtomicU64::new(0),
            total_errors: AtomicU64::new(0),
            consecutive_errors: AtomicU32::new(0),
            avg_latency_ms: AtomicU64::new(0),
            circuit: StdRwLock::new(CircuitState {
                open: false,
                opened_at: None,
            }),
        }
    }

    fn record_success(&self, latency_ms: f64) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
        self.consecutive_errors.store(0, Ordering::Relaxed);

        let alpha: u64 = 30;
        let latency_fixed = (latency_ms * 1000.0) as u64;
        let mut current = self.avg_latency_ms.load(Ordering::Relaxed);
        loop {
            let new_val = (alpha * latency_fixed + (1000 - alpha) * current) / 1000;
            match self.avg_latency_ms.compare_exchange_weak(
                current,
                new_val,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(actual) => current = actual,
            }
        }

        if let Ok(mut circuit) = self.circuit.write() {
            if circuit.open {
                debug!("Circuit breaker closed after successful request");
                circuit.open = false;
                circuit.opened_at = None;
            }
        }
    }

    fn record_error(&self) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
        self.total_errors.fetch_add(1, Ordering::Relaxed);
        let consecutive = self.consecutive_errors.fetch_add(1, Ordering::Relaxed) + 1;

        if consecutive >= CIRCUIT_BREAKER_THRESHOLD {
            if let Ok(mut circuit) = self.circuit.write() {
                if !circuit.open {
                    warn!(
                        "Circuit breaker opened after {} consecutive errors",
                        consecutive
                    );
                    circuit.open = true;
                    circuit.opened_at = Some(std::time::Instant::now());
                }
            }
        }
    }

    fn is_healthy(&self) -> bool {
        if let Ok(circuit) = self.circuit.read() {
            if circuit.open {
                if let Some(opened_at) = circuit.opened_at {
                    if opened_at.elapsed() > std::time::Duration::from_secs(CIRCUIT_BREAKER_RESET_SECS) {
                        return true;
                    }
                }
                return false;
            }
        }
        self.consecutive_errors.load(Ordering::Relaxed) < CIRCUIT_BREAKER_THRESHOLD
    }

    fn get_avg_latency_ms(&self) -> f64 {
        self.avg_latency_ms.load(Ordering::Relaxed) as f64 / 1000.0
    }

    fn error_rate(&self) -> f64 {
        let total = self.total_requests.load(Ordering::Relaxed);
        if total == 0 {
            return 0.0;
        }
        self.total_errors.load(Ordering::Relaxed) as f64 / total as f64
    }
}

trait ProviderAdapter: Send + Sync {
    fn build_request_body(
        &self,
        model: &str,
        messages: Vec<serde_json::Value>,
        max_tokens: u32,
        temperature: f32,
        stream: bool,
    ) -> serde_json::Value;

    fn parse_response(&self, raw: serde_json::Value) -> LLMResult<(String, Option<(Option<u32>, Option<u32>)>)>;

    fn endpoint_url(&self, base_url: &str) -> String;

    fn build_headers(
        &self,
        api_key: &str,
        extra: &std::collections::HashMap<String, String>,
    ) -> reqwest::header::HeaderMap;
}

struct OpenAIAdapter;
struct AnthropicAdapter;
struct GeminiAdapter;

impl ProviderAdapter for OpenAIAdapter {
    fn build_request_body(
        &self,
        model: &str,
        messages: Vec<serde_json::Value>,
        max_tokens: u32,
        temperature: f32,
        stream: bool,
    ) -> serde_json::Value {
        json!({
            "model": model,
            "messages": messages,
            "max_tokens": max_tokens,
            "temperature": temperature,
            "stream": stream,
        })
    }

    fn parse_response(&self, raw: serde_json::Value) -> LLMResult<(String, Option<(Option<u32>, Option<u32>)>)> {
        let resp: OpenAIChatResponse =
            serde_json::from_value(raw).map_err(|e| LLMError::InvalidResponse(format!("Parse error: {}", e)))?;

        if let Some(err) = resp.error {
            return Err(LLMError::InvalidResponse(err.message));
        }

        let content = resp
            .choices
            .first()
            .and_then(|c| c.message.content.clone())
            .ok_or(LLMError::EmptyResponse)?;

        let usage = resp.usage.map(|u| (u.prompt_tokens, u.completion_tokens));
        Ok((content, usage))
    }

    fn endpoint_url(&self, base_url: &str) -> String {
        format!("{}/chat/completions", base_url)
    }

    fn build_headers(
        &self,
        api_key: &str,
        extra: &std::collections::HashMap<String, String>,
    ) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();
        if let Ok(val) = format!("Bearer {}", api_key).parse() {
            headers.insert(header::AUTHORIZATION, val);
        }
        if let Ok(val) = "application/json".parse() {
            headers.insert(header::CONTENT_TYPE, val);
        }
        for (key, value) in extra {
            if let (Ok(k), Ok(v)) = (key.parse::<reqwest::header::HeaderName>(), value.parse()) {
                headers.insert(k, v);
            }
        }
        headers
    }
}

impl ProviderAdapter for AnthropicAdapter {
    fn build_request_body(
        &self,
        model: &str,
        messages: Vec<serde_json::Value>,
        max_tokens: u32,
        temperature: f32,
        _stream: bool,
    ) -> serde_json::Value {
        let mut system_msg = String::new();
        let mut filtered = Vec::new();
        for msg in &messages {
            if msg.get("role").and_then(|r| r.as_str()) == Some("system") {
                system_msg = msg
                    .get("content")
                    .and_then(|c| c.as_str())
                    .unwrap_or("")
                    .to_string();
            } else {
                filtered.push(msg.clone());
            }
        }

        let mut body = json!({
            "model": model,
            "messages": filtered,
            "max_tokens": max_tokens,
            "temperature": temperature,
        });
        if !system_msg.is_empty() {
            body["system"] = json!(system_msg);
        }
        body
    }

    fn parse_response(&self, raw: serde_json::Value) -> LLMResult<(String, Option<(Option<u32>, Option<u32>)>)> {
        let resp: AnthropicResponse =
            serde_json::from_value(raw).map_err(|e| LLMError::InvalidResponse(format!("Parse error: {}", e)))?;

        let content: String = resp
            .content
            .iter()
            .filter_map(|b| {
                if b.block_type == "text" {
                    b.text.clone()
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join("");

        if content.is_empty() {
            return Err(LLMError::EmptyResponse);
        }

        let usage = resp
            .usage
            .map(|u| (u.input_tokens, u.output_tokens));
        Ok((content, usage))
    }

    fn endpoint_url(&self, base_url: &str) -> String {
        format!("{}/messages", base_url)
    }

    fn build_headers(
        &self,
        api_key: &str,
        extra: &std::collections::HashMap<String, String>,
    ) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();
        if let Ok(val) = api_key.parse() {
            headers.insert("x-api-key", val);
        }
        if let Ok(val) = "application/json".parse() {
            headers.insert(header::CONTENT_TYPE, val);
        }
        if let Ok(val) = "2023-06-01".parse() {
            headers.insert("anthropic-version", val);
        }
        for (key, value) in extra {
            if let (Ok(k), Ok(v)) = (key.parse::<reqwest::header::HeaderName>(), value.parse()) {
                headers.insert(k, v);
            }
        }
        headers
    }
}

impl ProviderAdapter for GeminiAdapter {
    fn build_request_body(
        &self,
        model: &str,
        messages: Vec<serde_json::Value>,
        max_tokens: u32,
        temperature: f32,
        _stream: bool,
    ) -> serde_json::Value {
        let mut system_instruction = None;
        let mut contents = Vec::new();
        for msg in &messages {
            let role = msg.get("role").and_then(|r| r.as_str()).unwrap_or("user");
            let text = msg
                .get("content")
                .and_then(|c| c.as_str())
                .unwrap_or("");
            if role == "system" {
                system_instruction = Some(json!({ "parts": [{ "text": text }] }));
            } else {
                let gemini_role = if role == "assistant" { "model" } else { "user" };
                contents.push(json!({
                    "role": gemini_role,
                    "parts": [{ "text": text }]
                }));
            }
        }

        let mut body = json!({});
        if let Some(si) = system_instruction {
            body["systemInstruction"] = si;
        }
        body["contents"] = json!(contents);
        body["generationConfig"] = json!({
            "maxOutputTokens": max_tokens,
            "temperature": temperature,
        });
        body["model"] = json!(model);
        body
    }

    fn parse_response(&self, raw: serde_json::Value) -> LLMResult<(String, Option<(Option<u32>, Option<u32>)>)> {
        let resp: GeminiResponse =
            serde_json::from_value(raw).map_err(|e| LLMError::InvalidResponse(format!("Parse error: {}", e)))?;

        let content: String = resp
            .candidates
            .first()
            .map(|c| {
                c.content
                    .parts
                    .iter()
                    .filter_map(|p| p.text.clone())
                    .collect::<Vec<_>>()
                    .join("")
            })
            .unwrap_or_default();

        if content.is_empty() {
            return Err(LLMError::EmptyResponse);
        }

        Ok((content, None))
    }

    fn endpoint_url(&self, base_url: &str) -> String {
        format!("{}/models/{}:generateContent", base_url, "MODEL_PLACEHOLDER")
    }

    fn build_headers(
        &self,
        api_key: &str,
        _extra: &std::collections::HashMap<String, String>,
    ) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();
        if let Ok(val) = "application/json".parse() {
            headers.insert(header::CONTENT_TYPE, val);
        }
        if let Ok(val) = api_key.parse() {
            headers.insert("x-goog-api-key", val);
        }
        headers
    }
}

fn adapter_for_type(provider_type: LLMProviderType) -> Box<dyn ProviderAdapter> {
    match provider_type {
        LLMProviderType::Anthropic => Box::new(AnthropicAdapter),
        LLMProviderType::Gemini => Box::new(GeminiAdapter),
        _ => Box::new(OpenAIAdapter),
    }
}

#[derive(Clone)]
pub struct UniversalLLMClient {
    config: LLMProviderConfig,
    http: Client,
    metrics: Arc<ProviderMetrics>,
    system_prompt_cache: Arc<StdRwLock<LruCache<u64, String>>>,
    adapter: Arc<Box<dyn ProviderAdapter>>,
    rate_limiter: Arc<TokenBucket>,
}

impl UniversalLLMClient {
    pub fn new(config: LLMProviderConfig) -> LLMResult<Self> {
        let timeout = std::time::Duration::from_millis(config.timeout_ms);
        let http = Client::builder()
            .timeout(timeout)
            .tcp_keepalive(Some(std::time::Duration::from_secs(60)))
            .pool_max_idle_per_host(10)
            .pool_idle_timeout(Some(std::time::Duration::from_secs(90)))
            .http2_keep_alive_interval(Some(std::time::Duration::from_secs(30)))
            .build()
            .map_err(|e| LLMError::Request(e.to_string()))?;

        let adapter = adapter_for_type(config.provider_type);
        let max_rpm = std::env::var("LLM_MAX_RPM")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(DEFAULT_MAX_RPM);

        Ok(Self {
            config,
            http,
            metrics: Arc::new(ProviderMetrics::new()),
            system_prompt_cache: Arc::new(StdRwLock::new(LruCache::new(
                NonZeroUsize::new(SYSTEM_PROMPT_CACHE_SIZE).expect("cache size > 0"),
            ))),
            adapter: Arc::new(adapter),
            rate_limiter: Arc::new(TokenBucket::new(max_rpm)),
        })
    }

    pub fn name(&self) -> &str {
        &self.config.name
    }

    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    pub fn is_healthy(&self) -> bool {
        self.metrics.is_healthy()
    }

    pub fn priority(&self) -> u32 {
        self.config.priority
    }

    pub fn config_ref(&self) -> &LLMProviderConfig {
        &self.config
    }

    pub async fn get_status(&self) -> LLMProviderStatus {
        LLMProviderStatus {
            name: self.config.name.clone(),
            provider_type: self.config.provider_type,
            model: self.config.model.clone(),
            enabled: self.config.enabled,
            priority: self.config.priority,
            total_requests: self.metrics.total_requests.load(Ordering::Relaxed),
            total_errors: self.metrics.total_errors.load(Ordering::Relaxed),
            avg_latency_ms: self.metrics.get_avg_latency_ms(),
            healthy: self.metrics.is_healthy(),
        }
    }

    pub async fn chat_with_retry(
        &self,
        agent_id: &str,
        user_id: &str,
        message: &str,
        context: ChatContext,
        system_prompt_override: Option<&str>,
        ai_context: Option<&str>,
    ) -> LLMResult<ChatResponse> {
        let mut last_error = None;
        let mut backoff_ms = INITIAL_BACKOFF_MS;

        for attempt in 0..=MAX_RETRIES {
            if attempt > 0 {
                debug!(
                    "Retry attempt {}/{} for provider {} after {}ms backoff",
                    attempt, MAX_RETRIES, self.config.name, backoff_ms
                );
                tokio::time::sleep(std::time::Duration::from_millis(backoff_ms)).await;
                backoff_ms = (backoff_ms * 2).min(MAX_BACKOFF_MS);
            }

            match self
                .chat(
                    agent_id,
                    user_id,
                    message,
                    context.clone(),
                    system_prompt_override,
                    ai_context,
                )
                .await
            {
                Ok(response) => return Ok(response),
                Err(e) => {
                    last_error = Some(e.clone());
                    match &e {
                        LLMError::InvalidResponse(_) => return Err(e),
                        LLMError::Unhealthy(_) => return Err(e),
                        LLMError::RateLimited(retry_after) => {
                            if let Some(duration) = retry_after {
                                if attempt < MAX_RETRIES {
                                    tokio::time::sleep(*duration).await;
                                    continue;
                                }
                            }
                            return Err(e);
                        }
                        _ => {
                            if attempt == MAX_RETRIES {
                                warn!(
                                    "All retry attempts exhausted for provider {}: {:?}",
                                    self.config.name, e
                                );
                                return Err(e);
                            }
                        }
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| LLMError::Request("All retries failed".to_string())))
    }

    pub async fn chat(
        &self,
        agent_id: &str,
        user_id: &str,
        message: &str,
        context: ChatContext,
        system_prompt_override: Option<&str>,
        ai_context: Option<&str>,
    ) -> LLMResult<ChatResponse> {
        if !self.config.enabled {
            return Err(LLMError::Unhealthy(format!(
                "Provider '{}' is disabled",
                self.config.name
            )));
        }

        if !self.is_healthy() {
            return Err(LLMError::Unhealthy(format!(
                "Provider '{}' has too many consecutive errors",
                self.config.name
            )));
        }

        if !self.rate_limiter.try_acquire() {
            return Err(LLMError::RateLimited(None));
        }

        let system_msg = self.build_system_message(system_prompt_override, ai_context, &context);
        let messages = vec![
            json!({ "role": "system", "content": system_msg }),
            json!({ "role": "user", "content": message }),
        ];

        let body = self.adapter.build_request_body(
            &self.config.model,
            messages,
            self.config.max_tokens,
            self.config.temperature,
            false,
        );

        let url = self.adapter.endpoint_url(&self.config.base_url);
        let url = if self.config.provider_type == LLMProviderType::Gemini {
            url.replace("MODEL_PLACEHOLDER", &self.config.model)
        } else {
            url
        };

        debug!(
            "LLM request to {} (model: {}, agent: {}, user: {})",
            self.config.name, self.config.model, agent_id, user_id
        );

        let start = std::time::Instant::now();

        let headers = self
            .adapter
            .build_headers(
                self.config.api_key.expose_secret(),
                &self.config.extra_headers,
            );

        let response = self.http.post(&url).headers(headers).json(&body).send().await;

        match response {
            Ok(resp) => {
                let status = resp.status();
                let latency_ms = start.elapsed().as_millis() as u64;

                if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                    let retry_after = resp
                        .headers()
                        .get("retry-after")
                        .and_then(|v| v.to_str().ok())
                        .and_then(|v| v.parse::<u64>().ok())
                        .map(std::time::Duration::from_secs);
                    self.metrics.record_error();
                    return Err(LLMError::RateLimited(retry_after));
                }

                if !status.is_success() {
                    let error_text = resp.text().await.unwrap_or_else(|_| "Unknown".into());
                    let truncated: String = error_text.chars().take(200).collect();
                    self.metrics.record_error();
                    error!(
                        "LLM {} returned {} (agent: {}, user: {})",
                        self.config.name, status, agent_id, user_id
                    );
                    return Err(LLMError::RequestFailed(status.as_u16(), truncated));
                }

                let raw: serde_json::Value = resp.json().await.map_err(|e| {
                    LLMError::InvalidResponse(format!("Failed to parse response: {}", e))
                })?;

                let (content, usage) = self.adapter.parse_response(raw)?;

                self.metrics.record_success(latency_ms as f64);

                if latency_ms > 2000 {
                    warn!("LLM {} response took {}ms", self.config.name, latency_ms);
                } else {
                    debug!(
                        "LLM {} response in {}ms (usage: {:?})",
                        self.config.name, latency_ms, usage
                    );
                }

                Ok(ChatResponse {
                    response: content,
                    emotion: "neutral".into(),
                    animation: "talk".into(),
                    source: format!("llm:{}", self.config.name),
                    latency_ms,
                    cached: Some(false),
                })
            }
            Err(e) => {
                self.metrics.record_error();
                error!(
                    "LLM {} request failed (agent: {}, user: {})",
                    self.config.name, agent_id, user_id
                );
                if e.is_timeout() {
                    return Err(LLMError::Timeout(std::time::Duration::from_millis(
                        self.config.timeout_ms,
                    )));
                }
                Err(LLMError::Request(e.to_string()))
            }
        }
    }

    pub async fn simple_complete(&self, prompt: &str, max_tokens: u32) -> LLMResult<String> {
        let start = std::time::Instant::now();

        let messages = vec![json!({ "role": "user", "content": prompt })];
        let body = self
            .adapter
            .build_request_body(&self.config.model, messages, max_tokens, 0.7, false);

        let url = self.adapter.endpoint_url(&self.config.base_url);
        let url = if self.config.provider_type == LLMProviderType::Gemini {
            url.replace("MODEL_PLACEHOLDER", &self.config.model)
        } else {
            url
        };

        let headers = self
            .adapter
            .build_headers(
                self.config.api_key.expose_secret(),
                &self.config.extra_headers,
            );

        let response = self
            .http
            .post(&url)
            .headers(headers)
            .json(&body)
            .send()
            .await;

        match response {
            Ok(resp) => {
                let status = resp.status();

                if !status.is_success() {
                    let error_text = resp.text().await.unwrap_or_else(|_| "Unknown".into());
                    self.metrics.record_error();
                    return Err(LLMError::RequestFailed(
                        status.as_u16(),
                        error_text.chars().take(200).collect(),
                    ));
                }

                let raw: serde_json::Value = resp.json().await.map_err(|e| {
                    LLMError::InvalidResponse(format!("Failed to parse response: {}", e))
                })?;

                let (content, _) = self.adapter.parse_response(raw)?;

                let latency_ms = start.elapsed().as_millis() as f64;
                self.metrics.record_success(latency_ms);

                Ok(content)
            }
            Err(e) => {
                self.metrics.record_error();
                if e.is_timeout() {
                    return Err(LLMError::Timeout(std::time::Duration::from_millis(
                        self.config.timeout_ms,
                    )));
                }
                Err(LLMError::Request(e.to_string()))
            }
        }
    }

    pub async fn chat_stream(
        &self,
        agent_id: &str,
        user_id: &str,
        message: &str,
        context: ChatContext,
        system_prompt_override: Option<&str>,
        ai_context: Option<&str>,
    ) -> LLMResult<tokio_stream::wrappers::ReceiverStream<LLMResult<StreamChunk>>> {
        if !self.config.enabled {
            return Err(LLMError::Unhealthy(format!(
                "Provider '{}' is disabled",
                self.config.name
            )));
        }

        if !self.is_healthy() {
            return Err(LLMError::Unhealthy(format!(
                "Provider '{}' has too many consecutive errors",
                self.config.name
            )));
        }

        if !self.rate_limiter.try_acquire() {
            return Err(LLMError::RateLimited(None));
        }

        if !self.config.provider_type.is_openai_compatible() {
            return Err(LLMError::InvalidResponse(
                "Streaming only supported for OpenAI-compatible providers".into(),
            ));
        }

        let system_msg = self.build_system_message(system_prompt_override, ai_context, &context);
        let messages = vec![
            json!({ "role": "system", "content": system_msg }),
            json!({ "role": "user", "content": message }),
        ];

        let body = self.adapter.build_request_body(
            &self.config.model,
            messages,
            self.config.max_tokens,
            self.config.temperature,
            true,
        );

        let url = self.adapter.endpoint_url(&self.config.base_url);
        let headers = self
            .adapter
            .build_headers(
                self.config.api_key.expose_secret(),
                &self.config.extra_headers,
            );

        let start = std::time::Instant::now();
        let provider_name = self.config.name.clone();

        debug!(
            "LLM stream request to {} (model: {}, agent: {}, user: {})",
            self.config.name, self.config.model, agent_id, user_id
        );

        let response = self
            .http
            .post(&url)
            .headers(headers)
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    LLMError::Timeout(std::time::Duration::from_millis(self.config.timeout_ms))
                } else {
                    LLMError::Request(e.to_string())
                }
            })?;

        let status = response.status();
        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            let retry_after = response
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<u64>().ok())
                .map(std::time::Duration::from_secs);
            self.metrics.record_error();
            return Err(LLMError::RateLimited(retry_after));
        }

        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown".into());
            self.metrics.record_error();
            return Err(LLMError::RequestFailed(
                status.as_u16(),
                error_text.chars().take(200).collect(),
            ));
        }

        let (tx, rx) = tokio::sync::mpsc::channel(32);
        let metrics = self.metrics.clone();

        tokio::spawn(async move {
            let mut full_text = String::new();
            let mut stream = response.bytes_stream();

            while let Some(chunk_result) = stream.next().await {
                let chunk = match chunk_result {
                    Ok(c) => c,
                    Err(e) => {
                        let _ = tx
                            .send(Err(LLMError::Request(e.to_string())))
                            .await;
                        break;
                    }
                };

                let text = String::from_utf8_lossy(&chunk);
                for line in text.lines() {
                    let line = line.trim();
                    if !line.starts_with("data: ") {
                        continue;
                    }
                    let data = &line[6..];
                    if data == "[DONE]" {
                        let elapsed = start.elapsed().as_millis() as u64;
                        metrics.record_success(elapsed as f64);
                        let _ = tx
                            .send(Ok(StreamChunk {
                                delta: String::new(),
                                done: true,
                                emotion: Some("neutral".into()),
                                source: Some(format!("llm:{}", provider_name)),
                                latency_ms: Some(elapsed),
                            }))
                            .await;
                        return;
                    }

                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(data) {
                        let delta = parsed
                            .get("choices")
                            .and_then(|c| c.get(0))
                            .and_then(|c| c.get("delta"))
                            .and_then(|d| d.get("content"))
                            .and_then(|c| c.as_str())
                            .unwrap_or("");

                        if !delta.is_empty() {
                            full_text.push_str(delta);
                            let _ = tx
                                .send(Ok(StreamChunk {
                                    delta: delta.to_string(),
                                    done: false,
                                    emotion: None,
                                    source: None,
                                    latency_ms: None,
                                }))
                                .await;
                        }
                    }
                }
            }

            metrics.record_success(start.elapsed().as_millis() as f64);
            let _ = tx
                .send(Ok(StreamChunk {
                    delta: String::new(),
                    done: true,
                    emotion: Some("neutral".into()),
                    source: Some(format!("llm:{}", provider_name)),
                    latency_ms: Some(start.elapsed().as_millis() as u64),
                }))
                .await;
        });

        Ok(tokio_stream::wrappers::ReceiverStream::new(rx))
    }

    fn build_system_message(
        &self,
        override_prompt: Option<&str>,
        ai_context: Option<&str>,
        context: &ChatContext,
    ) -> String {
        let cache_key = {
            let mut hasher = DefaultHasher::new();
            override_prompt.hash(&mut hasher);
            ai_context.hash(&mut hasher);
            self.config.system_prompt.hash(&mut hasher);
            hasher.finish()
        };

        let static_part = {
            let cache = self.system_prompt_cache.read().unwrap();
            if let Some(cached) = cache.peek(&cache_key) {
                cached.clone()
            } else {
                drop(cache);
                let mut parts = Vec::new();
                let base_prompt = override_prompt
                    .or(self.config.system_prompt.as_deref())
                    .unwrap_or(
                        "You are an NPC in a game world. Respond in character, concisely.",
                    );
                parts.push(base_prompt.to_string());
                if let Some(ctx) = ai_context {
                    if !ctx.is_empty() {
                        parts.push(ctx.to_string());
                    }
                }
                let result = parts.join("\n\n");
                let mut cache = self.system_prompt_cache.write().unwrap();
                cache.put(cache_key, result.clone());
                result
            }
        };

        let mut context_parts = Vec::new();
        if let Some(ref pos) = context.position {
            context_parts.push(format!(
                "Position: ({}, {}, {}) in {}",
                pos.x as i32, pos.y as i32, pos.z as i32, pos.world
            ));
        }
        if let Some(ref activity) = context.activity {
            context_parts.push(format!("Current activity: {:?}", activity));
        }
        if !context.nearby_players.is_empty() {
            context_parts.push(format!("Nearby players: {:?}", context.nearby_players));
        }
        if !context.nearby_npcs.is_empty() {
            context_parts.push(format!("Nearby NPCs: {:?}", context.nearby_npcs));
        }
        if let Some(tod) = context.time_of_day {
            let period = if tod < 6000 {
                "morning"
            } else if tod < 12000 {
                "afternoon"
            } else if tod < 18000 {
                "evening"
            } else {
                "night"
            };
            context_parts.push(format!("Time of day: {} ({})", tod, period));
        }
        if let Some(ref weather) = context.weather {
            context_parts.push(format!("Weather: {:?}", weather));
        }

        if !context_parts.is_empty() {
            format!(
                "{}\n\n[CONTEXT]\n{}",
                static_part,
                context_parts.join("\n")
            )
        } else {
            static_part
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use secrecy::Secret;

    fn test_config() -> LLMProviderConfig {
        LLMProviderConfig {
            name: "test".into(),
            provider_type: npc_types::LLMProviderType::OpenAI,
            base_url: "https://api.openai.com/v1".into(),
            api_key: Secret::new("test-key".into()),
            model: "gpt-4o-mini".into(),
            max_tokens: 150,
            temperature: 0.8,
            system_prompt: None,
            extra_headers: HashMap::new(),
            enabled: true,
            priority: 0,
            timeout_ms: 30000,
        }
    }

    fn test_anthropic_config() -> LLMProviderConfig {
        LLMProviderConfig {
            name: "anthropic-test".into(),
            provider_type: npc_types::LLMProviderType::Anthropic,
            base_url: "https://api.anthropic.com/v1".into(),
            api_key: Secret::new("test-key".into()),
            model: "claude-sonnet-4-20250514".into(),
            max_tokens: 150,
            temperature: 0.8,
            system_prompt: None,
            extra_headers: HashMap::new(),
            enabled: true,
            priority: 0,
            timeout_ms: 30000,
        }
    }

    #[test]
    fn test_client_creation() {
        let client = UniversalLLMClient::new(test_config());
        assert!(client.is_ok());
        let client = client.unwrap();
        assert_eq!(client.name(), "test");
        assert!(client.is_enabled());
        assert_eq!(client.priority(), 0);
    }

    #[test]
    fn test_anthropic_client_creation() {
        let client = UniversalLLMClient::new(test_anthropic_config());
        assert!(client.is_ok());
    }

    #[test]
    fn test_health_tracking() {
        let client = UniversalLLMClient::new(test_config()).unwrap();
        assert!(client.is_healthy());

        for _ in 0..5 {
            client.metrics.record_error();
        }
        assert!(!client.is_healthy());
    }

    #[test]
    fn test_metrics_success() {
        let client = UniversalLLMClient::new(test_config()).unwrap();
        client.metrics.record_success(50.0);
        client.metrics.record_success(100.0);
        assert_eq!(
            client
                .metrics
                .total_requests
                .load(std::sync::atomic::Ordering::Relaxed),
            2
        );
        assert_eq!(
            client
                .metrics
                .consecutive_errors
                .load(std::sync::atomic::Ordering::Relaxed),
            0
        );
    }

    #[test]
    fn test_build_system_message() {
        let config = test_config();
        let client = UniversalLLMClient::new(config).unwrap();

        let msg = client.build_system_message(None, None, &ChatContext::default());
        assert!(msg.contains("NPC"));
    }

    #[test]
    fn test_build_system_message_with_context() {
        let config = test_config();
        let client = UniversalLLMClient::new(config).unwrap();

        let msg = client.build_system_message(
            Some("You are a wizard."),
            Some("[ACTIONS]\nYou can cast fireballs."),
            &ChatContext::default(),
        );
        assert!(msg.contains("wizard"));
        assert!(msg.contains("fireballs"));
    }

    #[test]
    fn test_system_prompt_cache() {
        let config = test_config();
        let client = UniversalLLMClient::new(config).unwrap();

        let msg1 = client.build_system_message(Some("Test prompt"), Some("ctx"), &ChatContext::default());
        let msg2 = client.build_system_message(Some("Test prompt"), Some("ctx"), &ChatContext::default());
        assert_eq!(msg1, msg2);

        let cache = client.system_prompt_cache.read().unwrap();
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn test_openai_adapter_parse() {
        let adapter = OpenAIAdapter;
        let response = json!({
            "choices": [{
                "message": { "content": "Hello!" },
                "finish_reason": "stop"
            }],
            "usage": { "prompt_tokens": 10, "completion_tokens": 5, "total_tokens": 15 }
        });

        let (content, usage) = adapter.parse_response(response).unwrap();
        assert_eq!(content, "Hello!");
        assert_eq!(usage.unwrap().0, Some(10));
    }

    #[test]
    fn test_anthropic_adapter_parse() {
        let adapter = AnthropicAdapter;
        let response = json!({
            "content": [{ "type": "text", "text": "Hello from Claude!" }],
            "usage": { "input_tokens": 10, "output_tokens": 5 }
        });

        let (content, usage) = adapter.parse_response(response).unwrap();
        assert_eq!(content, "Hello from Claude!");
        assert_eq!(usage.unwrap().0, Some(10));
    }

    #[test]
    fn test_gemini_adapter_parse() {
        let adapter = GeminiAdapter;
        let response = json!({
            "candidates": [{
                "content": {
                    "parts": [{ "text": "Hello from Gemini!" }]
                }
            }]
        });

        let (content, _) = adapter.parse_response(response).unwrap();
        assert_eq!(content, "Hello from Gemini!");
    }

    #[test]
    fn test_anthropic_build_body_extracts_system() {
        let adapter = AnthropicAdapter;
        let messages = vec![
            json!({ "role": "system", "content": "You are helpful." }),
            json!({ "role": "user", "content": "Hi" }),
        ];
        let body = adapter.build_request_body("claude-3", messages, 100, 0.7, false);
        assert_eq!(body["system"], "You are helpful.");
        let filtered = body["messages"].as_array().unwrap();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0]["role"], "user");
    }

    #[test]
    fn test_token_bucket_allows_within_limit() {
        let bucket = TokenBucket::new(5);
        for _ in 0..5 {
            assert!(bucket.try_acquire());
        }
    }

    #[test]
    fn test_token_bucket_blocks_over_limit() {
        let bucket = TokenBucket::new(3);
        assert!(bucket.try_acquire());
        assert!(bucket.try_acquire());
        assert!(bucket.try_acquire());
        assert!(!bucket.try_acquire());
    }

    #[test]
    fn test_token_bucket_zero_means_unlimited() {
        let bucket = TokenBucket::new(0);
        for _ in 0..100 {
            assert!(bucket.try_acquire());
        }
    }

    #[test]
    fn test_openai_stream_body() {
        let adapter = OpenAIAdapter;
        let messages = vec![json!({ "role": "user", "content": "Hi" })];
        let body = adapter.build_request_body("gpt-4", messages, 100, 0.7, true);
        assert_eq!(body["stream"], true);
    }

    #[test]
    fn test_stream_chunk_serialization() {
        let chunk = StreamChunk {
            delta: "Hello".into(),
            done: false,
            emotion: None,
            source: None,
            latency_ms: None,
        };
        let json = serde_json::to_string(&chunk).unwrap();
        assert!(json.contains("Hello"));
        assert!(json.contains("\"done\":false"));

        let done_chunk = StreamChunk {
            delta: String::new(),
            done: true,
            emotion: Some("neutral".into()),
            source: Some("llm:test".into()),
            latency_ms: Some(150),
        };
        let json = serde_json::to_string(&done_chunk).unwrap();
        assert!(json.contains("\"done\":true"));
        assert!(json.contains("neutral"));
    }

    #[test]
    fn test_config_ref() {
        let client = UniversalLLMClient::new(test_config()).unwrap();
        let config = client.config_ref();
        assert_eq!(config.name, "test");
        assert_eq!(config.model, "gpt-4o-mini");
    }
}
