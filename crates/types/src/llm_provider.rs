use secrecy::Secret;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::LazyLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LLMProviderType {
    OpenAI,
    Venice,
    Together,
    Groq,
    Mistral,
    DeepSeek,
    Fireworks,
    OpenRouter,
    Anthropic,
    Gemini,
    XAi,
    Perplexity,
    Ollama,
    LMStudio,
    Custom,
}

impl LLMProviderType {
    pub fn default_base_url(&self) -> Option<&'static str> {
        match self {
            Self::OpenAI => Some("https://api.openai.com/v1"),
            Self::Venice => Some("https://api.venice.ai/api/v1"),
            Self::Together => Some("https://api.together.xyz/v1"),
            Self::Groq => Some("https://api.groq.com/openai/v1"),
            Self::Mistral => Some("https://api.mistral.ai/v1"),
            Self::DeepSeek => Some("https://api.deepseek.com/v1"),
            Self::Fireworks => Some("https://api.fireworks.ai/inference/v1"),
            Self::OpenRouter => Some("https://openrouter.ai/api/v1"),
            Self::Anthropic => Some("https://api.anthropic.com/v1"),
            Self::Gemini => Some("https://generativelanguage.googleapis.com/v1beta"),
            Self::XAi => Some("https://api.x.ai/v1"),
            Self::Perplexity => Some("https://api.perplexity.ai"),
            Self::Ollama => Some("http://localhost:11434/v1"),
            Self::LMStudio => Some("http://localhost:1234/v1"),
            Self::Custom => None,
        }
    }

    pub fn all_presets() -> &'static [LLMProviderPreset] {
        &PRESETS
    }

    pub fn is_openai_compatible(&self) -> bool {
        !matches!(self, Self::Anthropic | Self::Gemini | Self::Custom)
    }
}

static PRESETS: LazyLock<Vec<LLMProviderPreset>> = LazyLock::new(|| {
    vec![
        LLMProviderPreset {
            provider_type: LLMProviderType::OpenAI,
            name: "openai".into(),
            label: "OpenAI".into(),
            base_url: "https://api.openai.com/v1".into(),
            default_model: "gpt-4o-mini".into(),
            supports_streaming: true,
            supports_function_calling: true,
            max_tokens_limit: 16384,
            description: "OpenAI GPT models".into(),
        },
        LLMProviderPreset {
            provider_type: LLMProviderType::Anthropic,
            name: "anthropic".into(),
            label: "Anthropic Claude".into(),
            base_url: "https://api.anthropic.com/v1".into(),
            default_model: "claude-sonnet-4-20250514".into(),
            supports_streaming: true,
            supports_function_calling: true,
            max_tokens_limit: 8192,
            description: "Anthropic Claude models".into(),
        },
        LLMProviderPreset {
            provider_type: LLMProviderType::Gemini,
            name: "gemini".into(),
            label: "Google Gemini".into(),
            base_url: "https://generativelanguage.googleapis.com/v1beta".into(),
            default_model: "gemini-2.0-flash".into(),
            supports_streaming: true,
            supports_function_calling: true,
            max_tokens_limit: 8192,
            description: "Google Gemini models".into(),
        },
        LLMProviderPreset {
            provider_type: LLMProviderType::XAi,
            name: "xai".into(),
            label: "xAI Grok".into(),
            base_url: "https://api.x.ai/v1".into(),
            default_model: "grok-3-mini-fast".into(),
            supports_streaming: true,
            supports_function_calling: true,
            max_tokens_limit: 16384,
            description: "xAI Grok models (OpenAI-compatible)".into(),
        },
        LLMProviderPreset {
            provider_type: LLMProviderType::Perplexity,
            name: "perplexity".into(),
            label: "Perplexity".into(),
            base_url: "https://api.perplexity.ai".into(),
            default_model: "sonar".into(),
            supports_streaming: true,
            supports_function_calling: false,
            max_tokens_limit: 4096,
            description: "Perplexity AI models (OpenAI-compatible)".into(),
        },
        LLMProviderPreset {
            provider_type: LLMProviderType::Venice,
            name: "venice".into(),
            label: "Venice AI".into(),
            base_url: "https://api.venice.ai/api/v1".into(),
            default_model: "venice-uncensored-role-play".into(),
            supports_streaming: true,
            supports_function_calling: false,
            max_tokens_limit: 4096,
            description: "Venice AI - uncensored roleplay models".into(),
        },
        LLMProviderPreset {
            provider_type: LLMProviderType::Together,
            name: "together".into(),
            label: "Together AI".into(),
            base_url: "https://api.together.xyz/v1".into(),
            default_model: "meta-llama/Llama-3.3-70B-Instruct-Turbo".into(),
            supports_streaming: true,
            supports_function_calling: false,
            max_tokens_limit: 4096,
            description: "Together AI - open source models at scale".into(),
        },
        LLMProviderPreset {
            provider_type: LLMProviderType::Groq,
            name: "groq".into(),
            label: "Groq".into(),
            base_url: "https://api.groq.com/openai/v1".into(),
            default_model: "llama-3.3-70b-versatile".into(),
            supports_streaming: true,
            supports_function_calling: true,
            max_tokens_limit: 8192,
            description: "Groq - ultra-fast inference (LPU)".into(),
        },
        LLMProviderPreset {
            provider_type: LLMProviderType::Mistral,
            name: "mistral".into(),
            label: "Mistral AI".into(),
            base_url: "https://api.mistral.ai/v1".into(),
            default_model: "mistral-small-latest".into(),
            supports_streaming: true,
            supports_function_calling: true,
            max_tokens_limit: 8192,
            description: "Mistral AI models".into(),
        },
        LLMProviderPreset {
            provider_type: LLMProviderType::DeepSeek,
            name: "deepseek".into(),
            label: "DeepSeek".into(),
            base_url: "https://api.deepseek.com/v1".into(),
            default_model: "deepseek-chat".into(),
            supports_streaming: true,
            supports_function_calling: false,
            max_tokens_limit: 8192,
            description: "DeepSeek AI models".into(),
        },
        LLMProviderPreset {
            provider_type: LLMProviderType::Fireworks,
            name: "fireworks".into(),
            label: "Fireworks AI".into(),
            base_url: "https://api.fireworks.ai/inference/v1".into(),
            default_model: "accounts/fireworks/models/llama-v3-8b-instruct".into(),
            supports_streaming: true,
            supports_function_calling: false,
            max_tokens_limit: 8192,
            description: "Fireworks AI - fast open-source inference".into(),
        },
        LLMProviderPreset {
            provider_type: LLMProviderType::OpenRouter,
            name: "openrouter".into(),
            label: "OpenRouter".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            default_model: "meta-llama/llama-3.3-70b-instruct".into(),
            supports_streaming: true,
            supports_function_calling: true,
            max_tokens_limit: 16384,
            description: "OpenRouter - unified gateway to all models".into(),
        },
        LLMProviderPreset {
            provider_type: LLMProviderType::Ollama,
            name: "ollama".into(),
            label: "Ollama (Local)".into(),
            base_url: "http://localhost:11434/v1".into(),
            default_model: "llama3".into(),
            supports_streaming: true,
            supports_function_calling: false,
            max_tokens_limit: 4096,
            description: "Ollama - local LLM inference".into(),
        },
        LLMProviderPreset {
            provider_type: LLMProviderType::LMStudio,
            name: "lmstudio".into(),
            label: "LM Studio (Local)".into(),
            base_url: "http://localhost:1234/v1".into(),
            default_model: "default".into(),
            supports_streaming: true,
            supports_function_calling: false,
            max_tokens_limit: 4096,
            description: "LM Studio - local LLM inference with UI".into(),
        },
    ]
});

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMProviderPreset {
    pub provider_type: LLMProviderType,
    pub name: String,
    pub label: String,
    pub base_url: String,
    pub default_model: String,
    pub supports_streaming: bool,
    pub supports_function_calling: bool,
    pub max_tokens_limit: u32,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMProviderConfig {
    pub name: String,
    pub provider_type: LLMProviderType,
    pub base_url: String,
    #[serde(skip_serializing)]
    pub api_key: Secret<String>,
    pub model: String,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    #[serde(default)]
    pub system_prompt: Option<String>,
    #[serde(default)]
    pub extra_headers: HashMap<String, String>,
    #[serde(default = "default_true_val")]
    pub enabled: bool,
    #[serde(default)]
    pub priority: u32,
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
}

fn default_max_tokens() -> u32 {
    150
}

fn default_temperature() -> f32 {
    0.8
}

fn default_true_val() -> bool {
    true
}

fn default_timeout() -> u64 {
    30000
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMProvidersResponse {
    pub presets: Vec<LLMProviderPreset>,
    pub total: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetLLMProviderRequest {
    pub provider_type: LLMProviderType,
    pub base_url: Option<String>,
    #[serde(skip_serializing)]
    pub api_key: Secret<String>,
    pub model: Option<String>,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    #[serde(default)]
    pub system_prompt: Option<String>,
    #[serde(default)]
    pub extra_headers: HashMap<String, String>,
    #[serde(default = "default_true_val")]
    pub enabled: bool,
    #[serde(default)]
    pub priority: u32,
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMChatRequest {
    pub messages: Vec<LLMMessage>,
    pub model: Option<String>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMMessage {
    pub role: LLMRole,
    pub content: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LLMRole {
    System,
    User,
    Assistant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMProviderStatus {
    pub name: String,
    pub provider_type: LLMProviderType,
    pub model: String,
    pub enabled: bool,
    pub priority: u32,
    pub total_requests: u64,
    pub total_errors: u64,
    pub avg_latency_ms: f64,
    pub healthy: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_presets() {
        let presets = LLMProviderType::all_presets();
        assert_eq!(presets.len(), 14);
        assert!(presets.iter().all(|p| !p.base_url.is_empty()));
    }

    #[test]
    fn test_default_base_urls() {
        assert_eq!(
            LLMProviderType::OpenAI.default_base_url(),
            Some("https://api.openai.com/v1")
        );
        assert_eq!(
            LLMProviderType::Groq.default_base_url(),
            Some("https://api.groq.com/openai/v1")
        );
        assert_eq!(
            LLMProviderType::Anthropic.default_base_url(),
            Some("https://api.anthropic.com/v1")
        );
        assert_eq!(
            LLMProviderType::Gemini.default_base_url(),
            Some("https://generativelanguage.googleapis.com/v1beta")
        );
        assert!(LLMProviderType::Custom.default_base_url().is_none());
    }

    #[test]
    fn test_openai_compatible_flag() {
        assert!(LLMProviderType::OpenAI.is_openai_compatible());
        assert!(LLMProviderType::Groq.is_openai_compatible());
        assert!(!LLMProviderType::Anthropic.is_openai_compatible());
        assert!(!LLMProviderType::Gemini.is_openai_compatible());
    }

    #[test]
    fn test_provider_config_defaults() {
        let config = LLMProviderConfig {
            name: "test".into(),
            provider_type: LLMProviderType::OpenAI,
            base_url: "https://api.openai.com/v1".into(),
            api_key: Secret::new("key".to_string()),
            model: "gpt-4o-mini".into(),
            max_tokens: default_max_tokens(),
            temperature: default_temperature(),
            system_prompt: None,
            extra_headers: HashMap::new(),
            enabled: default_true_val(),
            priority: 0,
            timeout_ms: default_timeout(),
        };
        assert_eq!(config.max_tokens, 150);
        assert!((config.temperature - 0.8).abs() < f32::EPSILON);
        assert!(config.enabled);
    }
}
