use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};

use npc_core::{NavigationMesh, PathfindingEngine};
use npc_types::{PathfindOptions, Position3D};

fn bench_pathfinding_straight(c: &mut Criterion) {
    let nav_mesh = NavigationMesh::new_flat(1000, 64);
    let engine = PathfindingEngine::new(nav_mesh);
    let options = PathfindOptions::default();

    let mut group = c.benchmark_group("pathfinding_straight");

    for distance in [10, 50, 100, 200] {
        group.bench_with_input(
            BenchmarkId::new("distance", distance),
            &distance,
            |b, &dist| {
                let start = Position3D::new(0.0, 64.0, 0.0, "overworld".to_string());
                let goal = Position3D::new(dist as f64, 64.0, 0.0, "overworld".to_string());

                b.iter(|| {
                    black_box(engine.find_path(
                        black_box(&start),
                        black_box(&goal),
                        black_box(&options),
                    ))
                });
            },
        );
    }

    group.finish();
}

fn bench_pathfinding_diagonal(c: &mut Criterion) {
    let nav_mesh = NavigationMesh::new_flat(1000, 64);
    let engine = PathfindingEngine::new(nav_mesh);
    let options = PathfindOptions::default();

    c.bench_function("pathfinding_diagonal_50", |b| {
        let start = Position3D::new(0.0, 64.0, 0.0, "overworld".to_string());
        let goal = Position3D::new(50.0, 64.0, 50.0, "overworld".to_string());

        b.iter(|| {
            black_box(engine.find_path(
                black_box(&start),
                black_box(&goal),
                black_box(&options),
            ))
        });
    });
}

fn bench_pathfinding_with_obstacles(c: &mut Criterion) {
    let mut nav_mesh = NavigationMesh::new_flat(100, 64);

    for i in 0..50 {
        nav_mesh.block(i, 64, 5);
    }

    let engine = PathfindingEngine::new(nav_mesh);
    let options = PathfindOptions::default();

    c.bench_function("pathfinding_obstacles", |b| {
        let start = Position3D::new(0.0, 64.0, 0.0, "overworld".to_string());
        let goal = Position3D::new(50.0, 64.0, 0.0, "overworld".to_string());

        b.iter(|| {
            black_box(engine.find_path(
                black_box(&start),
                black_box(&goal),
                black_box(&options),
            ))
        });
    });
}

fn bench_cache_operations(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("memory_cache_put_get", |b| {
        let cache = npc_cache::MemoryCache::new(1000);

        b.to_async(&rt).iter(|| async {
            cache.put("key1".to_string(), "value1".to_string()).await;
            let _ = cache.get(&"key1".to_string()).await;
        });
    });

    c.bench_function("memory_cache_concurrent", |b| {
        let cache = npc_cache::MemoryCache::new(1000);

        b.to_async(&rt).iter(|| async {
            let mut handles = vec![];
            for i in 0..100 {
                let cache = cache.clone();
                handles.push(tokio::spawn(async move {
                    cache.put(format!("key{}", i), format!("value{}", i)).await;
                    let _ = cache.get(&format!("key{}", i)).await;
                }));
            }
            for handle in handles {
                handle.await.unwrap();
            }
        });
    });
}

criterion_group!(
    benches,
    bench_pathfinding_straight,
    bench_pathfinding_diagonal,
    bench_pathfinding_with_obstacles,
    bench_cache_operations,
);

criterion_main!(benches);
