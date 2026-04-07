use tokio::sync::mpsc;
use tracing::{debug, info};

use npc_types::{ChatContext, ChatResponse};
use crate::predictive_cache::ResponseTemplates;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResponseQuality {
    Template,
    AI,
}

#[derive(Debug, Clone)]
pub struct HybridResponse {
    pub initial: ChatResponse,
    pub quality: ResponseQuality,
    pub upgrade_available: bool,
}

impl HybridResponse {
    pub fn template(response: ChatResponse) -> Self {
        Self {
            initial: response,
            quality: ResponseQuality::Template,
            upgrade_available: true,
        }
    }

    pub fn ai(response: ChatResponse) -> Self {
        Self {
            initial: response,
            quality: ResponseQuality::AI,
            upgrade_available: false,
        }
    }
}

pub type ResponseUpgradeChannel = mpsc::Receiver<ChatResponse>;

pub struct HybridResponseManager;

impl HybridResponseManager {
    pub async fn process_chat<F, Fut>(
        agent_id: &str,
        npc_name: &str,
        npc_kind: &str,
        message: &str,
        context: ChatContext,
        ai_generator: F,
    ) -> (HybridResponse, mpsc::Receiver<ChatResponse>)
    where
        F: FnOnce(String, String, String, ChatContext) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = Result<ChatResponse, Box<dyn std::error::Error + Send>>> + Send,
    {
        let (tx, rx) = mpsc::channel(1);

        let time_of_day = context
            .time_of_day
            .map(|t| if t < 6000 { "morning" } else if t < 12000 { "day" } else if t < 18000 { "evening" } else { "night" })
            .unwrap_or("day");

        let template = ResponseTemplates::get_template(npc_kind, npc_name, message, time_of_day);

        let instant_response = if let Some(template) = template {
            debug!("Using template response for instant reply");
            HybridResponse::template(template)
        } else {
            debug!("Using generic template (no specific match)");
            let generic = ResponseTemplates::greeting(npc_name, time_of_day);
            HybridResponse::template(generic)
        };

        let agent_id = agent_id.to_string();
        let message = message.to_string();

        tokio::spawn(async move {
            debug!("Generating AI upgrade in background...");
            let start = std::time::Instant::now();

            match ai_generator(agent_id.clone(), message, "user_id".to_string(), context).await {
                Ok(ai_response) => {
                    let elapsed = start.elapsed();
                    info!(
                        "AI upgrade generated in {}ms for agent {}",
                        elapsed.as_millis(),
                        agent_id
                    );
                    let _ = tx.send(ai_response).await;
                }
                Err(e) => {
                    debug!("AI upgrade failed: {} (template already sent)", e);
                }
            }
        });

        (instant_response, rx)
    }

    pub async fn process_chat_simple<F, Fut>(
        agent_id: &str,
        npc_name: &str,
        npc_kind: &str,
        message: &str,
        context: ChatContext,
        urgent: bool,
        ai_generator: F,
    ) -> Result<HybridResponse, Box<dyn std::error::Error + Send>>
    where
        F: FnOnce(String, String, String, ChatContext) -> Fut,
        Fut: std::future::Future<Output = Result<ChatResponse, Box<dyn std::error::Error + Send>>>,
    {
        if urgent {
            let time_of_day = context
                .time_of_day
                .map(|t| if t < 6000 { "morning" } else if t < 12000 { "day" } else if t < 18000 { "evening" } else { "night" })
                .unwrap_or("day");

            let template = ResponseTemplates::get_template(npc_kind, npc_name, message, time_of_day)
                .unwrap_or_else(|| ResponseTemplates::greeting(npc_name, time_of_day));

            Ok(HybridResponse::template(template))
        } else {
            let response = ai_generator(
                agent_id.to_string(),
                message.to_string(),
                "user_id".to_string(),
                context,
            ).await?;

            Ok(HybridResponse::ai(response))
        }
    }
}

pub struct UrgencyDetector;

impl UrgencyDetector {
    pub fn is_urgent(
        player_count_nearby: usize,
        active_conversations: usize,
        player_movement_speed: f64,
    ) -> bool {
        if player_count_nearby > 5 {
            return true;
        }
        if active_conversations > 10 {
            return true;
        }
        if player_movement_speed > 5.0 {
            return true;
        }
        false
    }

    pub fn max_acceptable_latency_ms(player_count: usize, conversation_depth: usize) -> u64 {
        if conversation_depth == 0 {
            return 50;
        }
        if conversation_depth > 3 {
            return 20;
        }
        if player_count > 10 {
            return 10;
        }
        30
    }
}

pub struct SmartChatHandler {
    template_threshold_ms: u64,
}

impl SmartChatHandler {
    pub fn new(template_threshold_ms: u64) -> Self {
        Self {
            template_threshold_ms,
        }
    }

    pub async fn handle<F, Fut>(
        &self,
        agent_id: &str,
        npc_name: &str,
        npc_kind: &str,
        message: &str,
        context: ChatContext,
        ai_generator: F,
    ) -> HybridResponse
    where
        F: FnOnce(String, String, String, ChatContext) -> Fut,
        Fut: std::future::Future<Output = Result<ChatResponse, Box<dyn std::error::Error + Send>>>,
    {
        let start = std::time::Instant::now();

        let ai_result = tokio::time::timeout(
            std::time::Duration::from_millis(self.template_threshold_ms),
            ai_generator(
                agent_id.to_string(),
                message.to_string(),
                "user_id".to_string(),
                context.clone(),
            ),
        )
        .await;

        let elapsed = start.elapsed().as_millis() as u64;

        match ai_result {
            Ok(Ok(ai_response)) => {
                debug!("AI response within threshold: {}ms", elapsed);
                HybridResponse::ai(ai_response)
            }
            Ok(Err(e)) => {
                debug!("AI generation failed: {}, using template", e);
                self.fallback_template(npc_name, npc_kind, message, &context)
            }
            Err(_timeout) => {
                debug!("AI generation timeout (>{}ms), using template", self.template_threshold_ms);
                self.fallback_template(npc_name, npc_kind, message, &context)
            }
        }
    }

    fn fallback_template(
        &self,
        npc_name: &str,
        npc_kind: &str,
        message: &str,
        context: &ChatContext,
    ) -> HybridResponse {
        let time_of_day = context
            .time_of_day
            .map(|t| if t < 6000 { "morning" } else if t < 12000 { "day" } else if t < 18000 { "evening" } else { "night" })
            .unwrap_or("day");

        let template = ResponseTemplates::get_template(npc_kind, npc_name, message, time_of_day)
            .unwrap_or_else(|| ResponseTemplates::greeting(npc_name, time_of_day));

        HybridResponse::template(template)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_urgency_detection() {
        assert!(UrgencyDetector::is_urgent(10, 5, 3.0));
        assert!(UrgencyDetector::is_urgent(2, 15, 3.0));
        assert!(UrgencyDetector::is_urgent(2, 5, 10.0));
        assert!(!UrgencyDetector::is_urgent(2, 5, 2.0));
    }

    #[test]
    fn test_max_latency() {
        assert_eq!(UrgencyDetector::max_acceptable_latency_ms(5, 0), 50);
        assert_eq!(UrgencyDetector::max_acceptable_latency_ms(5, 5), 20);
        assert_eq!(UrgencyDetector::max_acceptable_latency_ms(15, 2), 10);
    }

    #[tokio::test]
    async fn test_smart_handler_timeout() {
        let handler = SmartChatHandler::new(10);

        let slow_ai = |_a: String, _m: String, _u: String, _c: ChatContext| async {
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            Ok(ChatResponse {
                response: "AI response".to_string(),
                emotion: "neutral".to_string(),
                animation: "talk".to_string(),
                source: "ai".to_string(),
                latency_ms: 50,
                cached: Some(false),
            })
        };

        let context = ChatContext::default();

        let response = handler.handle(
            "agent1",
            "TestNPC",
            "merchant",
            "Hello",
            context,
            slow_ai,
        ).await;

        assert_eq!(response.quality, ResponseQuality::Template);
    }
}
