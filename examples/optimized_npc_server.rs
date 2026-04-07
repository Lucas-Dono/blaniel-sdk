/// Example: Complete optimized NPC server with all advanced features
///
/// Run with: cargo run --example optimized_npc_server
///
/// Demonstrates:
/// - Multi-key load balancing
/// - Predictive cache warming
/// - Hybrid responses
/// - Priority queue
/// - Full monitoring

use std::sync::Arc;
use tokio::sync::RwLock;

use npc_core::{
    AILoadBalancer, LoadBalanceStrategy,
    PredictiveCacheWarmer, ResponseTemplates, CacheWarmingTask,
    HybridResponseManager, SmartChatHandler, UrgencyDetector,
    NPCPriorityQueue, NPCRequest, Priority, QueueCleanupTask,
};
use npc_types::{ChatContext, Position3D};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter("debug,optimized_npc_server=trace")
        .init();

    println!("Starting Optimized NPC Server\n");

    println!("Initializing AI Load Balancer...");

    let venice_keys = vec![
        std::env::var("VENICE_API_KEY").unwrap_or_else(|_| "test_key_1".into()),
        std::env::var("VENICE_API_KEY_1").unwrap_or_else(|_| "test_key_2".into()),
        std::env::var("VENICE_API_KEY_2").unwrap_or_else(|_| "test_key_3".into()),
    ];

    let balancer = Arc::new(AILoadBalancer::new(
        venice_keys,
        vec![],
        Some("venice-uncensored-role-play".into()),
        None,
        LoadBalanceStrategy::LeastLatency,
    )?);

    println!("  Load balancer: {} Venice keys\n", 3);

    println!("Initializing Predictive Cache...");

    let cache_warmer = Arc::new(PredictiveCacheWarmer::new());

    let warming_task = CacheWarmingTask::new(cache_warmer.clone());
    let _warming_handle = warming_task.start();

    println!("  Cache warmer started\n");

    println!("Initializing Priority Queue...");

    let queue = Arc::new(NPCPriorityQueue::new(
        20,
        200,
    ));

    let cleanup = QueueCleanupTask::new(queue.clone());
    let _cleanup_handle = cleanup.start();

    println!("  Priority queue: 20 concurrent, 200 max\n");

    println!("Initializing Hybrid Response Handler...");

    let chat_handler = SmartChatHandler::new(20);

    println!("  Hybrid handler: 20ms threshold\n");

    println!("Simulating NPC interactions...\n");

    let test_scenarios = vec![
        ("merchant_bob", "merchant", 5.0, "What are you selling?", true),
        ("guard_alice", "guard", 3.0, "Hello", true),
        ("villager_john", "villager", 25.0, "Hi", false),
        ("quest_giver", "quest_giver", 8.0, "Any quests?", true),
        ("background_npc", "villager", 150.0, "Hello", false),
    ];

    for (npc_id, npc_kind, distance, message, is_active) in test_scenarios {
        println!("─────────────────────────────────────");
        println!("Processing: {} ({})", npc_id, npc_kind);
        println!("   Distance: {:.1} blocks", distance);
        println!("   Message: \"{}\"", message);

        let priority = Priority::calculate(
            distance,
            is_active,
            if npc_kind == "quest_giver" { 9 } else { 5 },
            if is_active { 2 } else { 0 },
        );
        println!("   Priority: {:?}", priority);

        let context = ChatContext {
            world: Some("overworld".to_string()),
            nearby_players: vec!["Player1".to_string()],
            nearby_items: vec![],
            time_of_day: Some("day".to_string()),
        };

        let request = NPCRequest::new(
            npc_id.to_string(),
            "player_1".to_string(),
            message.to_string(),
            context.clone(),
            priority,
        );

        queue.submit(request).await?;
        println!("   Queued");
    }

    println!("\n─────────────────────────────────────\n");

    println!("Processing queue...\n");

    while let Some(request) = queue.pop().await {
        println!("Processing: {}", request.agent_id);
        println!("   Priority: {:?}", request.priority);
        println!("   Wait time: {}ms", request.wait_time().as_millis());

        let _permit = queue.acquire_permit().await;

        let urgency = UrgencyDetector::is_urgent(
            1,
            5,
            2.0
        );

        let balancer_clone = balancer.clone();
        let agent_id = request.agent_id.clone();
        let message = request.message.clone();
        let context = request.context.clone();

        let response = chat_handler.handle(
            &agent_id,
            "TestNPC",
            "merchant",
            &message,
            context,
            move |agent, msg, user, ctx| {
                let balancer = balancer_clone.clone();
                async move {
                    println!("   Calling AI for {}", agent);
                    tokio::time::sleep(tokio::time::Duration::from_millis(15)).await;

                    Ok(npc_types::ChatResponse {
                        agent_id: agent,
                        message: format!("AI response to: {}", msg),
                        emotion: Some("happy".to_string()),
                        actions: vec![],
                        source: "venice".to_string(),
                        latency_ms: 15,
                        cached: false,
                    })
                }
            },
        ).await;

        println!("   Response: {:?}", response.quality);
        println!("   Message: \"{}\"", response.initial.message);
        println!();
    }

    println!("\n─────────────────────────────────────");
    println!("Final Statistics:\n");

    let stats = queue.stats().await;
    println!("Queue Stats:");
    println!("  Total pending: {}", stats.total);
    println!("  Avg wait time: {}ms", stats.avg_wait_time_ms);
    println!("  Available permits: {}", stats.available_permits);

    let health = balancer.get_health_metrics().await;
    println!("\nAI Health:");
    println!("{}", serde_json::to_string_pretty(&health)?);

    println!("\nDemo complete!");

    Ok(())
}
