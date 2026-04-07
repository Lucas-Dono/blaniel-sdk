use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, SamplingMode};
use std::time::Duration;

// Mock types for benchmarking (replace with actual imports when integrated)
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Benchmark cache lookups (target: < 1ms, ideally < 100μs)
fn bench_cache_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache");
    group.warm_up_time(Duration::from_millis(100));
    group.measurement_time(Duration::from_secs(2));
    group.sampling_mode(SamplingMode::Flat);

    // Memory cache simulation
    let rt = tokio::runtime::Runtime::new().unwrap();

    group.bench_function("memory_cache_get", |b| {
        b.to_async(&rt).iter(|| async {
            let cache: Arc<RwLock<HashMap<String, String>>> = Arc::new(RwLock::new(HashMap::new()));
            {
                let mut c = cache.write().await;
                c.insert("test_key".to_string(), "test_value".to_string());
            }

            // Benchmark read
            let c = cache.read().await;
            black_box(c.get("test_key"));
        });
    });

    group.bench_function("memory_cache_put", |b| {
        b.to_async(&rt).iter(|| async {
            let cache: Arc<RwLock<HashMap<String, String>>> = Arc::new(RwLock::new(HashMap::new()));
            let mut c = cache.write().await;
            black_box(c.insert("test_key".to_string(), "test_value".to_string()));
        });
    });

    group.finish();
}

/// Benchmark coordinate validation (target: < 10μs)
fn bench_coordinate_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("validation");
    group.warm_up_time(Duration::from_millis(50));
    group.measurement_time(Duration::from_millis(500));

    group.bench_function("coordinate_validation", |b| {
        b.iter(|| {
            let x = black_box(100.5_f64);
            let y = black_box(64.0_f64);
            let z = black_box(-200.3_f64);
            let radius = black_box(50.0_f64);

            // Validation logic
            black_box(
                x.is_finite() &&
                y.is_finite() &&
                z.is_finite() &&
                radius.is_finite() &&
                radius > 0.0 &&
                radius <= 1000.0
            );
        });
    });

    group.finish();
}

/// Benchmark distance calculations (target: < 5μs)
fn bench_distance_calculations(c: &mut Criterion) {
    let mut group = c.benchmark_group("distance");
    group.warm_up_time(Duration::from_millis(50));
    group.measurement_time(Duration::from_millis(500));

    group.bench_function("squared_distance", |b| {
        b.iter(|| {
            let x1 = black_box(100.0_f64);
            let y1 = black_box(64.0_f64);
            let z1 = black_box(-200.0_f64);
            let x2 = black_box(150.0_f64);
            let y2 = black_box(70.0_f64);
            let z2 = black_box(-180.0_f64);

            // Optimized: squared distance (no sqrt)
            black_box(
                (x2 - x1).powi(2) +
                (y2 - y1).powi(2) +
                (z2 - z1).powi(2)
            );
        });
    });

    group.bench_function("euclidean_distance", |b| {
        b.iter(|| {
            let x1 = black_box(100.0_f64);
            let y1 = black_box(64.0_f64);
            let z1 = black_box(-200.0_f64);
            let x2 = black_box(150.0_f64);
            let y2 = black_box(70.0_f64);
            let z2 = black_box(-180.0_f64);

            // With sqrt (slower)
            black_box(
                ((x2 - x1).powi(2) +
                 (y2 - y1).powi(2) +
                 (z2 - z1).powi(2)).sqrt()
            );
        });
    });

    group.finish();
}

/// Benchmark JSON serialization/deserialization (target: < 100μs)
fn bench_json_operations(c: &mut Criterion) {
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, Clone)]
    struct NpcState {
        id: String,
        name: String,
        position: Option<Position>,
        emotion: String,
    }

    #[derive(Serialize, Deserialize, Clone)]
    struct Position {
        x: f64,
        y: f64,
        z: f64,
        world: String,
    }

    let mut group = c.benchmark_group("json");
    group.warm_up_time(Duration::from_millis(100));
    group.measurement_time(Duration::from_secs(1));

    let state = NpcState {
        id: "agent_123".to_string(),
        name: "TestNPC".to_string(),
        position: Some(Position {
            x: 100.5,
            y: 64.0,
            z: -200.3,
            world: "overworld".to_string(),
        }),
        emotion: "happy".to_string(),
    };

    group.bench_function("serialize", |b| {
        b.iter(|| {
            black_box(serde_json::to_string(&state).unwrap());
        });
    });

    let json_str = serde_json::to_string(&state).unwrap();

    group.bench_function("deserialize", |b| {
        b.iter(|| {
            black_box(serde_json::from_str::<NpcState>(&json_str).unwrap());
        });
    });

    group.finish();
}

/// Benchmark HMAC verification (target: < 50μs)
fn bench_hmac_verification(c: &mut Criterion) {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    let mut group = c.benchmark_group("hmac");
    group.warm_up_time(Duration::from_millis(100));
    group.measurement_time(Duration::from_secs(1));

    let secret = b"test-secret-key-for-webhook-auth";
    let message = b"{\"event\":\"test\",\"data\":\"sample payload\"}";

    group.bench_function("hmac_sha256_compute", |b| {
        b.iter(|| {
            let mut mac = Hmac::<Sha256>::new_from_slice(secret).unwrap();
            mac.update(black_box(message));
            black_box(mac.finalize().into_bytes());
        });
    });

    let mut mac = Hmac::<Sha256>::new_from_slice(secret).unwrap();
    mac.update(message);
    let expected = mac.finalize().into_bytes();

    group.bench_function("hmac_sha256_verify", |b| {
        b.iter(|| {
            let mut mac = Hmac::<Sha256>::new_from_slice(secret).unwrap();
            mac.update(black_box(message));
            black_box(mac.verify_slice(&expected));
        });
    });

    group.finish();
}

/// Benchmark different AI client call patterns (target: < 20ms total)
fn bench_ai_response_time(c: &mut Criterion) {
    let mut group = c.benchmark_group("ai_response");
    group.warm_up_time(Duration::from_millis(500));
    group.measurement_time(Duration::from_secs(5));
    group.sample_size(10); // Fewer samples due to network I/O

    // Note: These are simulated benchmarks
    // For real benchmarks, uncomment and use actual clients

    group.bench_function("simulated_venice_uncensored", |b| {
        b.iter(|| {
            // Simulate Venice AI response time
            std::thread::sleep(Duration::from_millis(15));
            black_box("response");
        });
    });

    group.bench_function("simulated_together_llama3", |b| {
        b.iter(|| {
            // Simulate Together.ai response time
            std::thread::sleep(Duration::from_millis(18));
            black_box("response");
        });
    });

    group.finish();
}

/// Full critical path benchmark: cache miss → DB query → AI response → cache write
/// Target: < 50ms total for critical path
fn bench_critical_path(c: &mut Criterion) {
    let mut group = c.benchmark_group("critical_path");
    group.warm_up_time(Duration::from_millis(500));
    group.measurement_time(Duration::from_secs(3));
    group.sample_size(20);

    let rt = tokio::runtime::Runtime::new().unwrap();

    group.bench_function("full_npc_state_fetch", |b| {
        b.to_async(&rt).iter(|| async {
            // Simulate critical path:
            // 1. Memory cache miss (<1ms)
            tokio::time::sleep(Duration::from_micros(50)).await;

            // 2. Redis cache miss (~1ms)
            tokio::time::sleep(Duration::from_millis(1)).await;

            // 3. DB query (~5ms)
            tokio::time::sleep(Duration::from_millis(5)).await;

            // 4. Process and cache (~1ms)
            tokio::time::sleep(Duration::from_millis(1)).await;

            black_box("npc_state");
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_cache_operations,
    bench_coordinate_validation,
    bench_distance_calculations,
    bench_json_operations,
    bench_hmac_verification,
    bench_ai_response_time,
    bench_critical_path,
);

criterion_main!(benches);
