use std::sync::Arc;
use tracing::debug;

use npc_cache::{CacheStrategy, MemoryCache, RedisCache};
use npc_types::{ChatContext, ChatResponse};

use crate::{ai_client::NextJsClient, dialogue::AmbientDialogueManager, llm::LLMProviderRegistry};

pub struct RequestRouter {
    memory_cache: MemoryCache<String, ChatResponse>,
    redis_cache: RedisCache,
    nextjs_client: NextJsClient,
    llm_registry: Arc<LLMProviderRegistry>,
    ambient_dialogue: AmbientDialogueManager,
}

impl RequestRouter {
    pub fn new(
        memory_cache: MemoryCache<String, ChatResponse>,
        redis_cache: RedisCache,
        nextjs_client: NextJsClient,
        llm_registry: Arc<LLMProviderRegistry>,
    ) -> Self {
        Self {
            memory_cache,
            redis_cache,
            nextjs_client,
            llm_registry,
            ambient_dialogue: AmbientDialogueManager::new(),
        }
    }

    pub async fn handle_chat(
        &self,
        agent_id: &str,
        user_id: &str,
        message: &str,
        context: ChatContext,
        ai_context: Option<String>,
    ) -> anyhow::Result<ChatResponse> {
        let cache_key = format!("chat:{}:{}", agent_id, message.to_lowercase());

        // 1. Check memory cache (sync via parking_lot, no .await)
        if let Some(cached) = self.memory_cache.get(&cache_key) {
            debug!("Memory cache hit for chat: {}", cache_key);
            let mut response = cached;
            response.cached = Some(true);
            return Ok(response);
        }

        // 2. Check Redis cache
        if let Ok(Some(cached)) = self.redis_cache.get::<ChatResponse>(&cache_key).await {
            debug!("Redis cache hit for chat: {}", cache_key);
            self.memory_cache.put(cache_key.clone(), cached.clone());

            let mut response = cached;
            response.cached = Some(true);
            return Ok(response);
        }

        // 3. Try simple/ambient response
        if let Some(simple) = self.ambient_dialogue.try_simple_response(message) {
            debug!("Using ambient dialogue response");

            let _ = self
                .redis_cache
                .set_with_ttl(&cache_key, &simple, CacheStrategy::SIMPLE_CHAT_TTL)
                .await;
            self.memory_cache.put(cache_key, simple.clone());

            return Ok(simple);
        }

        // 4. Analyze complexity — lowercase ONCE, reuse everywhere
        let normalized = message.to_lowercase();
        let complexity = Self::analyze_message_complexity(message, &normalized);
        debug!("Message complexity: {}", complexity);

        if complexity < 0.3 {
            debug!("Using template response (low complexity)");
            let response = Self::generate_template_response(&normalized);

            let _ = self
                .redis_cache
                .set_with_ttl(&cache_key, &response, CacheStrategy::SIMPLE_CHAT_TTL)
                .await;
            self.memory_cache.put(cache_key, response.clone());

            Ok(response)
        } else {
            debug!("Routing to AI (high complexity)");

            let llm_result = self
                .llm_registry
                .chat(
                    agent_id,
                    user_id,
                    message,
                    context.clone(),
                    None,
                    ai_context.as_deref(),
                )
                .await;

            let response = match llm_result {
                Ok(resp) => {
                    debug!("LLM registry responded via '{}'", resp.source);
                    resp
                }
                Err(e) => {
                    debug!("LLM registry failed ({}), falling back to Next.js", e);
                    self.nextjs_client
                        .process_complex_chat(
                            agent_id,
                            user_id,
                            message,
                            context,
                            ai_context.as_deref(),
                        )
                        .await?
                }
            };

            let _ = self
                .redis_cache
                .set_with_ttl(&cache_key, &response, CacheStrategy::AI_CHAT_TTL)
                .await;

            Ok(response)
        }
    }

    fn analyze_message_complexity(message: &str, normalized: &str) -> f32 {
        let indicators = [
            message.split_whitespace().count() > 10,
            message.contains('?'),
            normalized.contains("why"),
            normalized.contains("how"),
            normalized.contains("feel"),
            normalized.contains("think"),
            normalized.contains("because"),
            normalized.contains("what if"),
            normalized.contains("explain"),
        ];

        indicators.iter().filter(|&&x| x).count() as f32 / indicators.len() as f32
    }

    fn generate_template_response(normalized: &str) -> ChatResponse {
        let (response, emotion, animation) = if normalized.contains("thank") {
            ("You're welcome! I'm here to help.", "joy", "nod")
        } else if normalized.contains("help") {
            ("Of course! How can I help you?", "joy", "talk")
        } else if normalized.contains("where") {
            ("I'm not sure about the exact location.", "neutral", "shake")
        } else if normalized.contains("when") {
            ("I don't have information about when.", "neutral", "shake")
        } else {
            ("Interesting. Tell me more.", "curiosity", "nod")
        };

        ChatResponse {
            response: response.to_string(),
            emotion: emotion.to_string(),
            animation: animation.to_string(),
            source: "template".to_string(),
            latency_ms: 3,
            cached: Some(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_complexity_simple() {
        let msg = "hello";
        let normalized = msg.to_lowercase();
        let complexity = RequestRouter::analyze_message_complexity(msg, &normalized);
        assert!(complexity < 0.3);
    }

    #[test]
    fn test_complexity_complex() {
        let msg = "Why do you think sky is blue?";
        let normalized = msg.to_lowercase();
        let complexity = RequestRouter::analyze_message_complexity(msg, &normalized);
        assert!(complexity >= 0.3);
    }
}
