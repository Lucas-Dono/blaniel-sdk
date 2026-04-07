use blaniel_sdk::{BlanielClient, SdkError};
use npc_types::LLMProviderType;
use std::collections::HashMap;

#[derive(clap::Subcommand)]
pub enum LlmCommands {
    Presets,
    List,
    Status {
        #[arg(help = "Provider name")]
        name: String,
    },
    Add {
        #[arg(long, help = "Provider type (openai, groq, deepseek, etc.)")]
        provider: String,
        #[arg(long, help = "API key")]
        key: String,
        #[arg(long, help = "Model name (optional, uses default)")]
        model: Option<String>,
        #[arg(long, help = "Custom base URL (optional)")]
        base_url: Option<String>,
    },
    Remove {
        #[arg(help = "Provider name")]
        name: String,
    },
    Metrics {
        #[arg(long, help = "Show global metrics")]
        global: bool,
        #[arg(long, help = "Agent ID for agent metrics")]
        agent: Option<String>,
        #[arg(long, help = "User ID for user metrics")]
        user: Option<String>,
        #[arg(long)]
        top: bool,
        #[arg(long)]
        recent: bool,
    },
}

pub async fn run(cmd: LlmCommands) -> Result<(), SdkError> {
    let client = BlanielClient::from_env()?;

    match cmd {
        LlmCommands::Presets => {
            let resp = client.get_llm_presets().await?;
            println!("\x1b[1mAvailable LLM Providers ({} presets)\x1b[0m\n", resp.total);
            for preset in &resp.presets {
                println!("  \x1b[36m{}\x1b[0m - {}", preset.name, preset.label);
                println!("    \x1b[90m{}\x1b[0m", preset.description);
                println!("    Model: {} | Streaming: {} | Functions: {}",
                    preset.default_model,
                    if preset.supports_streaming { "yes" } else { "no" },
                    if preset.supports_function_calling { "yes" } else { "no" },
                );
                println!();
            }
        }

        LlmCommands::List => {
            let providers = client.list_llm_providers().await?;
            if providers.is_empty() {
                println!("No LLM providers configured.");
                println!("Use \x1b[36mblaniel llm add --provider openai --key YOUR_KEY\x1b[0m to add one.");
                return Ok(());
            }
            println!("\x1b[1mConfigured LLM Providers\x1b[0m\n");
            for p in &providers {
                let status = if p.healthy { "\x1b[32mhealthy\x1b[0m" } else { "\x1b[31munhealthy\x1b[0m" };
                println!("  \x1b[36m{}\x1b[0m [{}] ({})",
                    p.name, p.model, status);
                println!("    Requests: {} | Errors: {} | Avg latency: {:.0}ms | Priority: {}",
                    p.total_requests, p.total_errors, p.avg_latency_ms, p.priority);
                println!();
            }
        }

        LlmCommands::Status { name } => {
            let status = client.get_llm_provider_status(&name).await?;
            let health = if status.healthy { "\x1b[32mhealthy\x1b[0m" } else { "\x1b[31munhealthy\x1b[0m" };
            println!("\x1b[1m{}\x1b[0m - {}", status.name, health);
            println!("  Model: {}", status.model);
            println!("  Enabled: {}", status.enabled);
            println!("  Priority: {}", status.priority);
            println!("  Total requests: {}", status.total_requests);
            println!("  Total errors: {}", status.total_errors);
            println!("  Avg latency: {:.1}ms", status.avg_latency_ms);
        }

        LlmCommands::Add { provider, key, model, base_url } => {
            let provider_type = parse_provider_type(&provider)?;
            println!("Adding \x1b[36m{}\x1b[0m provider...", provider);

            let status = client
                .register_provider_simple(
                    provider_type,
                    &key,
                    model.as_deref(),
                    base_url.as_deref(),
                )
                .await?;

            println!("\x1b[32m\u{2713}\x1b[0m Provider \x1b[1m{}\x1b[0m registered", status.name);
            println!("  Model: {} | Priority: {}", status.model, status.priority);
        }

        LlmCommands::Remove { name } => {
            client.unregister_llm_provider(&name).await?;
            println!("\x1b[32m\u{2713}\x1b[0m Provider \x1b[1m{}\x1b[0m removed", name);
        }

        LlmCommands::Metrics { global, agent, user, top, recent } => {
            let metrics = if global {
                client.get_llm_metrics_global().await?
            } else if let Some(aid) = agent {
                client.get_llm_metrics_by_agent(&aid).await?
            } else if let Some(uid) = user {
                client.get_llm_metrics_by_user(&uid).await?
            } else if top {
                client.get_llm_metrics_top_agents().await?
            } else if recent {
                client.get_llm_metrics_recent().await?
            } else {
                client.get_llm_metrics_global().await?
            };
            println!("{}", serde_json::to_string_pretty(&metrics)?);
        }
    }

    Ok(())
}

fn parse_provider_type(s: &str) -> Result<LLMProviderType, SdkError> {
    match s.to_lowercase().as_str() {
        "openai" => Ok(LLMProviderType::OpenAI),
        "anthropic" | "claude" => Ok(LLMProviderType::Anthropic),
        "gemini" | "google" => Ok(LLMProviderType::Gemini),
        "xai" | "grok" => Ok(LLMProviderType::XAi),
        "perplexity" => Ok(LLMProviderType::Perplexity),
        "venice" => Ok(LLMProviderType::Venice),
        "together" => Ok(LLMProviderType::Together),
        "groq" => Ok(LLMProviderType::Groq),
        "mistral" => Ok(LLMProviderType::Mistral),
        "deepseek" => Ok(LLMProviderType::DeepSeek),
        "fireworks" => Ok(LLMProviderType::Fireworks),
        "openrouter" => Ok(LLMProviderType::OpenRouter),
        "ollama" => Ok(LLMProviderType::Ollama),
        "lmstudio" | "lm-studio" => Ok(LLMProviderType::LMStudio),
        "custom" => Ok(LLMProviderType::Custom),
        _ => Err(SdkError::Config(format!(
            "Unknown provider '{}'. Available: openai, anthropic, gemini, xai, perplexity, venice, together, groq, mistral, deepseek, fireworks, openrouter, ollama, lmstudio, custom",
            s
        ))),
    }
}
