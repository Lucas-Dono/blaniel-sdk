/// Real-world AI provider benchmarks
/// Run with: cargo bench --bench ai_providers
///
/// Requires:
/// - VENICE_API_KEY env var
/// - TOGETHER_API_KEY env var (optional)
///
/// This benchmark measures ACTUAL API response times to help achieve <20ms target

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::time::Duration;

#[cfg(feature = "integration-tests")]
use npc_core::{TogetherClient, VeniceClient};
#[cfg(feature = "integration-tests")]
use npc_types::ChatContext;

fn bench_venice_real(c: &mut Criterion) {
    #[cfg(feature = "integration-tests")]
    {
        let api_key = std::env::var("VENICE_API_KEY")
            .expect("VENICE_API_KEY must be set for benchmarks");

        let rt = tokio::runtime::Runtime::new().unwrap();
        let client = VeniceClient::new(api_key, Some("venice-uncensored-role-play".to_string()))
            .expect("Failed to create Venice client");

        let mut group = c.benchmark_group("venice_real");
        group.sample_size(10);
        group.measurement_time(Duration::from_secs(30));

        group.bench_function("simple_completion", |b| {
            b.to_async(&rt).iter(|| async {
                let result = client.complete("Say 'Hello' in one word", 5).await;
                black_box(result);
            });
        });

        group.bench_function("npc_chat", |b| {
            b.to_async(&rt).iter(|| async {
                let context = ChatContext {
                    world: Some("overworld".to_string()),
                    nearby_players: vec!["Player1".to_string()],
                    nearby_items: vec![],
                    time_of_day: Some("day".to_string()),
                };

                let result = client
                    .process_chat(
                        "test_agent",
                        "test_user",
                        "Hello",
                        context,
                        Some("You are a friendly NPC. Be brief."),
                    )
                    .await;

                black_box(result);
            });
        });

        group.finish();
    }

    #[cfg(not(feature = "integration-tests"))]
    {
        println!("Skipping Venice benchmarks - enable 'integration-tests' feature");
    }
}

fn bench_together_real(c: &mut Criterion) {
    #[cfg(feature = "integration-tests")]
    {
        let api_key = match std::env::var("TOGETHER_API_KEY") {
            Ok(key) => key,
            Err(_) => {
                println!("TOGETHER_API_KEY not set, skipping Together.ai benchmarks");
                return;
            }
        };

        let rt = tokio::runtime::Runtime::new().unwrap();
        let client = TogetherClient::new(api_key, Some("meta-llama/Llama-3-8b-chat-hf".to_string()))
            .expect("Failed to create Together client");

        let mut group = c.benchmark_group("together_real");
        group.sample_size(10);
        group.measurement_time(Duration::from_secs(30));

        group.bench_function("simple_completion", |b| {
            b.to_async(&rt).iter(|| async {
                let result = client.complete("Say 'Hello' in one word", 5).await;
                black_box(result);
            });
        });

        group.bench_function("npc_chat", |b| {
            b.to_async(&rt).iter(|| async {
                let context = ChatContext {
                    world: Some("overworld".to_string()),
                    nearby_players: vec!["Player1".to_string()],
                    nearby_items: vec![],
                    time_of_day: Some("day".to_string()),
                };

                let result = client
                    .process_chat(
                        "test_agent",
                        "test_user",
                        "Hello",
                        context,
                        Some("You are a friendly NPC. Be brief."),
                    )
                    .await;

                black_box(result);
            });
        });

        group.finish();
    }

    #[cfg(not(feature = "integration-tests"))]
    {
        println!("Skipping Together.ai benchmarks - enable 'integration-tests' feature");
    }
}

criterion_group!(
    name = ai_benches;
    config = Criterion::default()
        .warm_up_time(Duration::from_secs(2))
        .measurement_time(Duration::from_secs(30));
    targets = bench_venice_real, bench_together_real
);

criterion_main!(ai_benches);
