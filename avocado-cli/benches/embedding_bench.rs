//! Embedding Performance Benchmarks
//!
//! Benchmarks embedding performance for different batch sizes and models.
//! This helps users understand the performance characteristics on their hardware.

use avocado_core::embedding;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use tokio::runtime::Runtime;

/// Benchmark single text embedding
fn bench_single_embedding(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let test_text = "This is a test query for embedding performance benchmarking";

    c.bench_function("single_embedding", |b| {
        b.iter(|| {
            rt.block_on(async {
                let result = embedding::embed_text(black_box(test_text), None, None).await;
                black_box(result)
            })
        });
    });
}

/// Benchmark batch embedding with different sizes
fn bench_batch_embedding(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("batch_embedding");

    // Test different batch sizes
    for size in [10, 50, 100].iter() {
        let texts: Vec<&str> = (0..*size)
            .map(|i| match i % 3 {
                0 => "This is a test query for embedding performance",
                1 => "How does the caching system work in AvocadoDB?",
                _ => "Explain the deterministic compilation algorithm",
            })
            .collect();

        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| {
                rt.block_on(async {
                    let result = embedding::embed_batch(black_box(texts.clone()), None, None).await;
                    black_box(result)
                })
            });
        });
    }

    group.finish();
}

/// Benchmark embedding with different models (if available)
fn bench_models(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("models");

    let test_text = "This is a test query for comparing different embedding models";

    // Benchmark each available model
    for model in ["allminilml6v2", "nomicv15", "bgelarge"].iter() {
        // Set environment variable for model selection
        std::env::set_var("AVOCADODB_EMBEDDING_MODEL", model);

        group.bench_with_input(BenchmarkId::from_parameter(model), model, |b, _| {
            b.iter(|| {
                rt.block_on(async {
                    let result = embedding::embed_text(black_box(test_text), None, None).await;
                    black_box(result)
                })
            });
        });
    }

    // Reset to default
    std::env::remove_var("AVOCADODB_EMBEDDING_MODEL");

    group.finish();
}

/// Benchmark realistic workload: query + compile
fn bench_realistic_workload(c: &mut Criterion) {
    // This would benchmark the full compilation pipeline
    // For now, we focus on embedding performance
    c.bench_function("realistic_query_compilation", |b| {
        // TODO: Add full compilation benchmark once we integrate with database
        b.iter(|| {
            // Placeholder for full pipeline benchmark
            black_box(42)
        });
    });
}

criterion_group!(
    benches,
    bench_single_embedding,
    bench_batch_embedding,
    bench_models,
    bench_realistic_workload
);
criterion_main!(benches);
