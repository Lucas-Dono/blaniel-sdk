mod error;
mod handlers;
mod middleware;
mod routes;
mod state;

use anyhow::Result;
use dotenvy::dotenv;
use std::net::SocketAddr;
use tower_http::cors::{Any, CorsLayer};
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use npc_cache::RedisCache;
use npc_core::{LLMProviderRegistry, NextJsClient};
use npc_db::create_pool;

use crate::{routes::create_router, state::AppState};

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,npc_api=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer().json())
        .init();

    info!("Starting Blaniel NPC API v{}", env!("CARGO_PKG_VERSION"));

    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");
    let redis_url = std::env::var("REDIS_URL")
        .expect("REDIS_URL must be set");
    let jwt_secret = std::env::var("JWT_SECRET")
        .expect("JWT_SECRET must be set");
    let nextjs_api_url = std::env::var("NEXTJS_API_URL")
        .unwrap_or_else(|_| "http://localhost:3000".into());
    let nextjs_api_key = std::env::var("NEXTJS_API_KEY")
        .unwrap_or_else(|_| "default-key".into());
    let port: u16 = std::env::var("PORT")
        .unwrap_or_else(|_| "3001".to_string())
        .parse()
        .expect("PORT must be a valid number");

    info!("Connecting to database...");
    let db_pool = create_pool(&database_url).await?;
    info!("Database connected successfully");

    info!("Connecting to Redis...");
    let redis = RedisCache::new(&redis_url).await?;
    info!("Redis connected successfully");

    let nextjs_client = NextJsClient::new(nextjs_api_url, nextjs_api_key)?;

    let llm_registry = LLMProviderRegistry::load_from_env();
    let provider_count = llm_registry.init_from_env().await.unwrap_or(0);
    info!("LLM providers initialized: {}", provider_count);

    let state = AppState::new(db_pool, redis, nextjs_client, llm_registry, jwt_secret);

    let app = create_router(state)
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        );

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    info!("Starting server on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;

    info!("NPC API server running on http://{}", addr);
    info!("Health check: http://{}/api/v1/health", addr);
    info!("Metrics: http://{}/api/v1/metrics", addr);

    axum::serve(listener, app)
        .await
        .expect("Failed to start server");

    Ok(())
}
