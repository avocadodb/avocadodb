//! Session Management Performance Benchmarks
//!
//! Benchmarks key session operations to ensure they meet performance targets:
//! - Session creation: < 5ms
//! - Message insertion: < 5ms
//! - History retrieval: < 50ms
//! - Session replay: < 100ms
//!
//! Run with: cargo bench --bench session_bench

use avocado_core::db::Database;
use avocado_core::index::VectorIndex;
use avocado_core::session::SessionManager;
use avocado_core::types::{CompilerConfig, MessageRole};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::path::PathBuf;
use tempfile::TempDir;
use tokio::runtime::Runtime;

/// Helper to create a test database
fn create_test_db() -> (TempDir, Database) {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("bench.db");
    let db = Database::new(&db_path).unwrap();
    (temp_dir, db)
}

/// Benchmark session creation
fn bench_session_creation(c: &mut Criterion) {
    c.bench_function("session_create", |b| {
        b.iter_batched(
            || {
                let (_temp_dir, db) = create_test_db();
                SessionManager::new(db)
            },
            |session_manager| {
                let session = session_manager
                    .start_session(black_box(Some("bench_user")))
                    .unwrap();
                black_box(session)
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

/// Benchmark message insertion
fn bench_message_insertion(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("message_insert", |b| {
        b.iter_batched(
            || {
                let (_temp_dir, db) = create_test_db();
                let session_manager = SessionManager::new(db.clone());
                let session = session_manager
                    .start_session(Some("bench_user"))
                    .unwrap();

                // Ingest some test data for context compilation
                rt.block_on(async {
                    db.ingest_text("test.md", "Test content for benchmarking")
                        .await
                        .unwrap();
                });

                (session_manager, session.id, VectorIndex::new())
            },
            |(session_manager, session_id, index)| {
                rt.block_on(async {
                    let config = CompilerConfig {
                        token_budget: 8000,
                        max_results: 50,
                        min_score: 0.1,
                    };

                    let result = session_manager
                        .add_user_message(
                            black_box(&session_id),
                            black_box("Test message"),
                            config,
                            &index,
                            None,
                        )
                        .await
                        .unwrap();

                    black_box(result)
                });
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

/// Benchmark assistant message insertion (simpler - no compilation)
fn bench_assistant_message_insertion(c: &mut Criterion) {
    c.bench_function("assistant_message_insert", |b| {
        b.iter_batched(
            || {
                let (_temp_dir, db) = create_test_db();
                let session_manager = SessionManager::new(db);
                let session = session_manager
                    .start_session(Some("bench_user"))
                    .unwrap();
                (session_manager, session.id)
            },
            |(session_manager, session_id)| {
                let message = session_manager
                    .add_assistant_message(
                        black_box(&session_id),
                        black_box("Test response"),
                        None,
                    )
                    .unwrap();
                black_box(message)
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

/// Benchmark history retrieval with different session sizes
fn bench_history_retrieval(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("history_retrieval");

    for size in [10, 50, 100].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter_batched(
                || {
                    let (_temp_dir, db) = create_test_db();
                    let session_manager = SessionManager::new(db.clone());
                    let session = session_manager
                        .start_session(Some("bench_user"))
                        .unwrap();

                    // Pre-populate with messages
                    rt.block_on(async {
                        db.ingest_text("test.md", "Test content").await.unwrap();
                    });

                    let index = VectorIndex::new();
                    let config = CompilerConfig {
                        token_budget: 8000,
                        max_results: 50,
                        min_score: 0.1,
                    };

                    for i in 0..size {
                        rt.block_on(async {
                            session_manager
                                .add_user_message(
                                    &session.id,
                                    &format!("Message {}", i),
                                    config.clone(),
                                    &index,
                                    None,
                                )
                                .await
                                .unwrap();

                            session_manager
                                .add_assistant_message(
                                    &session.id,
                                    &format!("Response {}", i),
                                    None,
                                )
                                .unwrap();
                        });
                    }

                    (session_manager, session.id)
                },
                |(session_manager, session_id)| {
                    let history = session_manager
                        .get_conversation_history(black_box(&session_id), None)
                        .unwrap();
                    black_box(history)
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}

/// Benchmark session replay
fn bench_session_replay(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("session_replay");

    for size in [10, 50].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, &size| {
            b.iter_batched(
                || {
                    let (_temp_dir, db) = create_test_db();
                    let session_manager = SessionManager::new(db.clone());
                    let session = session_manager
                        .start_session(Some("bench_user"))
                        .unwrap();

                    rt.block_on(async {
                        db.ingest_text("test.md", "Test content").await.unwrap();
                    });

                    let index = VectorIndex::new();
                    let config = CompilerConfig {
                        token_budget: 8000,
                        max_results: 50,
                        min_score: 0.1,
                    };

                    // Create conversation turns
                    for i in 0..size {
                        rt.block_on(async {
                            session_manager
                                .add_user_message(
                                    &session.id,
                                    &format!("Question {}", i),
                                    config.clone(),
                                    &index,
                                    None,
                                )
                                .await
                                .unwrap();

                            session_manager
                                .add_assistant_message(
                                    &session.id,
                                    &format!("Answer {}", i),
                                    None,
                                )
                                .unwrap();
                        });
                    }

                    (session_manager, session.id)
                },
                |(session_manager, session_id)| {
                    let replay = session_manager
                        .replay_session(black_box(&session_id))
                        .unwrap();
                    black_box(replay)
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_session_creation,
    bench_message_insertion,
    bench_assistant_message_insertion,
    bench_history_retrieval,
    bench_session_replay
);

criterion_main!(benches);
