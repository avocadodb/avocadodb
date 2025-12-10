use avocado_core::{compiler, db::Database, embedding, span, CompilerConfig};
use criterion::{criterion_group, criterion_main, Criterion};
use sha2::{Digest, Sha256};
use tokio::runtime::Runtime;
use uuid::Uuid;

fn build_corpus(db: &Database, num_docs: usize, lines_per_doc: usize) {
    for i in 0..num_docs {
        let artifact_id = Uuid::new_v4().to_string();
        let mut content = String::new();
        content.push_str("# AvocadoDB Bench Doc\n");
        for l in 0..lines_per_doc {
            content.push_str(&format!(
                "Line {} in doc {}: Deterministic RAG with MMR and hybrid search.\n",
                l + 1,
                i
            ));
        }
        let content_hash = format!("{:x}", Sha256::digest(content.as_bytes()));
        let artifact = avocado_core::Artifact {
            id: artifact_id.clone(),
            path: format!("bench/doc_{}.md", i),
            content: content.clone(),
            content_hash,
            metadata: None,
            created_at: chrono::Utc::now(),
        };
        db.insert_artifact(&artifact)
            .expect("insert_artifact failed");

        let mut spans = span::extract_spans(&content, &artifact_id).expect("extract_spans failed");
        let texts: Vec<&str> = spans.iter().map(|s| s.text.as_str()).collect();
        let rt = Runtime::new().unwrap();
        let embeddings = rt
            .block_on(embedding::embed_batch(texts, None, None))
            .expect("embed_batch failed (local embeddings required)");
        for (s, e) in spans.iter_mut().zip(embeddings.iter()) {
            s.embedding = Some(e.clone());
            s.embedding_model = Some(embedding::embedding_model().to_string());
        }
        db.insert_spans(&spans).expect("insert_spans failed");
    }
}

fn bench_warm_cold(c: &mut Criterion) {
    // Use file-backed DB under /tmp to avoid memory pressure
    let tmp_dir = tempfile::tempdir().expect("tmpdir");
    let db_path = tmp_dir.path().join("bench.sqlite");
    let db = Database::new(&db_path).expect("db new");

    // Build a medium corpus
    build_corpus(&db, 100, 30); // ~100 docs, ~spans per doc

    // Cold: first index build
    c.bench_function("index_build_cold", |b| {
        b.iter(|| {
            // Mark index dirty to force rebuild
            // Reopen DB to simulate cold build
            let db2 = Database::new(&db_path).expect("db new 2");
            let _ = db2.get_vector_index().expect("get_vector_index cold");
        });
    });

    // Prepare index once (warm cache in process)
    let index = db.get_vector_index().expect("index");

    // Warm compile
    let config = CompilerConfig {
        token_budget: 8000,
        ..Default::default()
    };
    c.bench_function("compile_warm", |b| {
        let rt = Runtime::new().unwrap();
        b.iter(|| {
            rt.block_on(async {
                let _ws = compiler::compile(
                    "What is AvocadoDB?",
                    config.clone(),
                    &db,
                    index.as_ref(),
                    None,
                )
                .await
                .expect("compile");
            });
        });
    });
}

criterion_group!(benches, bench_warm_cold);
criterion_main!(benches);
