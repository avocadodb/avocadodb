//! Tests for new determinism and explainability features
//!
//! Tests cover:
//! - Version Manifest generation
//! - Explain Plan output
//! - Content-Hash Incremental Rebuild
//! - Evaluation metrics
//! - Working Set Diff

use avocado_core::{
    compiler, db::Database, embedding, span,
    CompilerConfig, EvalResult, EvalSummary, GoldenQuery, IngestAction, WorkingSet,
};
use sha2::{Digest, Sha256};
use tempfile::TempDir;

// ============================================================================
// Test Helpers
// ============================================================================

async fn setup_test_db() -> (TempDir, Database) {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let db = Database::new(&db_path).unwrap();
    (temp_dir, db)
}

async fn ingest_test_document(db: &Database, path: &str, content: &str) -> String {
    let artifact_id = uuid::Uuid::new_v4().to_string();
    let content_hash = format!("{:x}", Sha256::digest(content.as_bytes()));

    let artifact = avocado_core::Artifact {
        id: artifact_id.clone(),
        path: path.to_string(),
        content: content.to_string(),
        content_hash,
        metadata: None,
        created_at: chrono::Utc::now(),
    };

    db.insert_artifact(&artifact).unwrap();

    let mut spans = span::extract_spans(content, &artifact_id).unwrap();

    // Generate embeddings
    let texts: Vec<&str> = spans.iter().map(|s| s.text.as_str()).collect();
    let embeddings = embedding::embed_batch(texts, None, None).await.unwrap();

    for (span, emb) in spans.iter_mut().zip(embeddings.iter()) {
        span.embedding = Some(emb.clone());
        span.embedding_model = Some("test-model".to_string());
    }

    db.insert_spans(&spans).unwrap();

    artifact_id
}

// ============================================================================
// Version Manifest Tests
// ============================================================================

#[tokio::test]
async fn test_manifest_included_in_working_set() {
    let (_temp_dir, db) = setup_test_db().await;

    // Ingest test document
    let content = "This is a test document about authentication.\n".repeat(30);
    ingest_test_document(&db, "auth.md", &content).await;

    // Build index and compile
    let index = db.get_vector_index().unwrap();
    let config = CompilerConfig::default();

    let working_set = compiler::compile("authentication", config, &db, index.as_ref(), None)
        .await
        .unwrap();

    // Verify manifest is present
    assert!(working_set.manifest.is_some(), "Manifest should be present");

    let manifest = working_set.manifest.unwrap();

    // Verify manifest fields
    assert!(!manifest.avocado_version.is_empty(), "Version should be set");
    assert_eq!(manifest.tokenizer, "cl100k_base");
    assert!(!manifest.embedding_model.is_empty(), "Embedding model should be set");
    assert!(manifest.embedding_dimension > 0, "Dimension should be positive");
    assert!(!manifest.context_hash.is_empty(), "Context hash should be set");

    // Verify chunking params have defaults
    assert_eq!(manifest.chunking.min_lines, 20);
    assert_eq!(manifest.chunking.max_lines, 50);
    assert_eq!(manifest.chunking.target_lines, 30);

    // Verify index params have defaults
    assert_eq!(manifest.index.hnsw_m, 32);
    assert_eq!(manifest.index.hnsw_ef_construction, 200);
    assert_eq!(manifest.index.hnsw_ef_search, 50);

    println!("✅ Manifest test passed");
}

#[tokio::test]
async fn test_manifest_context_hash_matches() {
    let (_temp_dir, db) = setup_test_db().await;

    let content = "Document about database queries and SQL.\n".repeat(30);
    ingest_test_document(&db, "sql.md", &content).await;

    let index = db.get_vector_index().unwrap();
    let config = CompilerConfig::default();

    let working_set = compiler::compile("SQL queries", config, &db, index.as_ref(), None)
        .await
        .unwrap();

    let manifest = working_set.manifest.as_ref().unwrap();

    // Manually compute context hash
    let expected_hash = format!("{:x}", Sha256::digest(working_set.text.as_bytes()));

    assert_eq!(
        manifest.context_hash, expected_hash,
        "Manifest context hash should match actual text hash"
    );

    println!("✅ Context hash verification passed");
}

#[tokio::test]
async fn test_manifest_determinism() {
    let (_temp_dir, db) = setup_test_db().await;

    let content = "Deterministic content for testing manifest consistency.\n".repeat(30);
    ingest_test_document(&db, "determinism.md", &content).await;

    let index = db.get_vector_index().unwrap();
    let config = CompilerConfig::default();

    // Compile twice
    let ws1 = compiler::compile("testing", config.clone(), &db, index.as_ref(), None)
        .await
        .unwrap();
    let ws2 = compiler::compile("testing", config, &db, index.as_ref(), None)
        .await
        .unwrap();

    let m1 = ws1.manifest.unwrap();
    let m2 = ws2.manifest.unwrap();

    // Manifests should be identical
    assert_eq!(m1.context_hash, m2.context_hash);
    assert_eq!(m1.avocado_version, m2.avocado_version);
    assert_eq!(m1.embedding_model, m2.embedding_model);

    println!("✅ Manifest determinism test passed");
}

// ============================================================================
// Explain Plan Tests
// ============================================================================

#[tokio::test]
async fn test_explain_plan_generated() {
    let (_temp_dir, db) = setup_test_db().await;

    let content = "This document explains the explain plan feature.\n".repeat(30);
    ingest_test_document(&db, "explain.md", &content).await;

    let index = db.get_vector_index().unwrap();
    let config = CompilerConfig::default();

    // Compile with explain=true
    let working_set =
        compiler::compile_with_options("explain plan", config, &db, index.as_ref(), None, true)
            .await
            .unwrap();

    // Verify explain plan is present
    assert!(working_set.explain.is_some(), "Explain plan should be present");

    let explain = working_set.explain.unwrap();

    // Verify explain plan fields
    assert_eq!(explain.query, "explain plan");
    assert!(!explain.query_embedding_hash.is_empty(), "Query embedding hash should be set");

    // Verify timing was recorded
    assert!(explain.timing.total_ms > 0, "Total time should be recorded");
    // Timing values are u64, so always >= 0
    let _ = explain.timing.embed_query_ms;
    let _ = explain.timing.semantic_search_ms;

    // Verify thresholds are populated
    assert_eq!(explain.thresholds.semantic_k, 50);
    assert_eq!(explain.thresholds.lexical_k, 20);
    assert_eq!(explain.thresholds.token_budget, 8000);

    println!("✅ Explain plan generation test passed");
}

#[tokio::test]
async fn test_explain_plan_not_generated_when_disabled() {
    let (_temp_dir, db) = setup_test_db().await;

    let content = "Content for testing explain disabled.\n".repeat(30);
    ingest_test_document(&db, "no_explain.md", &content).await;

    let index = db.get_vector_index().unwrap();
    let config = CompilerConfig::default();

    // Compile with explain=false (default)
    let working_set =
        compiler::compile_with_options("test", config, &db, index.as_ref(), None, false)
            .await
            .unwrap();

    // Explain plan should be None
    assert!(
        working_set.explain.is_none(),
        "Explain plan should not be present when disabled"
    );

    println!("✅ Explain disabled test passed");
}

#[tokio::test]
async fn test_explain_plan_shows_pipeline_stages() {
    let (_temp_dir, db) = setup_test_db().await;

    // Create multiple documents for interesting pipeline behavior
    for i in 0..5 {
        let content = format!(
            "Document {} about search and retrieval algorithms.\n",
            i
        )
        .repeat(30);
        ingest_test_document(&db, &format!("doc{}.md", i), &content).await;
    }

    let index = db.get_vector_index().unwrap();
    let config = CompilerConfig::default();

    let working_set =
        compiler::compile_with_options("search algorithms", config, &db, index.as_ref(), None, true)
            .await
            .unwrap();

    let explain = working_set.explain.unwrap();

    // With multiple documents, we should see candidates at various stages
    // Note: exact counts depend on content, but we verify structure
    println!("Semantic candidates: {}", explain.semantic_candidates.len());
    println!("Lexical candidates: {}", explain.lexical_candidates.len());
    println!("Fused candidates: {}", explain.fused_candidates.len());
    println!("MMR candidates: {}", explain.mmr_candidates.len());
    println!("Packed candidates: {}", explain.packed_candidates.len());
    println!("Final candidates: {}", explain.final_candidates.len());

    // Verify candidates have required fields
    if !explain.final_candidates.is_empty() {
        let candidate = &explain.final_candidates[0];
        assert!(!candidate.span_id.is_empty());
        assert!(!candidate.artifact_path.is_empty());
        assert!(candidate.rank > 0);
        assert!(candidate.tokens > 0);
    }

    println!("✅ Explain pipeline stages test passed");
}

// ============================================================================
// Content-Hash Incremental Rebuild Tests
// ============================================================================

#[tokio::test]
async fn test_determine_ingest_action_create() {
    let (_temp_dir, db) = setup_test_db().await;

    let content_hash = format!("{:x}", Sha256::digest(b"new content"));

    let action = db
        .determine_ingest_action("new_file.md", &content_hash)
        .unwrap();

    match action {
        IngestAction::Create => println!("✅ Create action test passed"),
        _ => panic!("Expected Create action for new file"),
    }
}

#[tokio::test]
async fn test_determine_ingest_action_skip() {
    let (_temp_dir, db) = setup_test_db().await;

    let content = "existing content";
    let content_hash = format!("{:x}", Sha256::digest(content.as_bytes()));

    // Insert existing artifact
    let artifact = avocado_core::Artifact {
        id: uuid::Uuid::new_v4().to_string(),
        path: "existing.md".to_string(),
        content: content.to_string(),
        content_hash: content_hash.clone(),
        metadata: None,
        created_at: chrono::Utc::now(),
    };
    db.insert_artifact(&artifact).unwrap();

    // Same content hash should skip
    let action = db
        .determine_ingest_action("existing.md", &content_hash)
        .unwrap();

    match action {
        IngestAction::Skip { artifact_id, reason } => {
            assert_eq!(artifact_id, artifact.id);
            assert!(reason.contains("unchanged"));
            println!("✅ Skip action test passed");
        }
        _ => panic!("Expected Skip action for unchanged content"),
    }
}

#[tokio::test]
async fn test_determine_ingest_action_update() {
    let (_temp_dir, db) = setup_test_db().await;

    let old_content = "old content";
    let old_hash = format!("{:x}", Sha256::digest(old_content.as_bytes()));

    // Insert existing artifact
    let artifact = avocado_core::Artifact {
        id: uuid::Uuid::new_v4().to_string(),
        path: "changing.md".to_string(),
        content: old_content.to_string(),
        content_hash: old_hash,
        metadata: None,
        created_at: chrono::Utc::now(),
    };
    db.insert_artifact(&artifact).unwrap();

    // Different content hash should trigger update
    let new_hash = format!("{:x}", Sha256::digest(b"new content"));
    let action = db
        .determine_ingest_action("changing.md", &new_hash)
        .unwrap();

    match action {
        IngestAction::Update { artifact_id } => {
            assert_eq!(artifact_id, artifact.id);
            println!("✅ Update action test passed");
        }
        _ => panic!("Expected Update action for changed content"),
    }
}

#[tokio::test]
async fn test_delete_artifact() {
    let (_temp_dir, db) = setup_test_db().await;

    // Insert artifact with spans
    let content = "Content to be deleted.\n".repeat(30);
    let artifact_id = ingest_test_document(&db, "to_delete.md", &content).await;

    // Verify spans exist
    let (_, spans_before, _) = db.get_stats().unwrap();
    assert!(spans_before > 0, "Spans should exist before deletion");

    // Delete artifact
    let deleted_count = db.delete_artifact(&artifact_id).unwrap();
    assert!(deleted_count > 0, "Should have deleted spans");

    // Verify artifact and spans are gone
    let artifact = db.get_artifact(&artifact_id).unwrap();
    assert!(artifact.is_none(), "Artifact should be deleted");

    println!("✅ Delete artifact test passed");
}

// ============================================================================
// Evaluation Module Tests
// ============================================================================

#[test]
fn test_eval_types_serialize() {
    let query = GoldenQuery {
        query: "test query".to_string(),
        expected_paths: vec!["a.md".to_string(), "b.md".to_string()],
        k: 5,
    };

    let json = serde_json::to_string(&query).unwrap();
    let parsed: GoldenQuery = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed.query, query.query);
    assert_eq!(parsed.expected_paths, query.expected_paths);
    assert_eq!(parsed.k, query.k);

    println!("✅ GoldenQuery serialization test passed");
}

#[test]
fn test_eval_result_types() {
    let result = EvalResult {
        query: "test".to_string(),
        recall_at_k: 0.8,
        precision_at_k: 0.6,
        mrr: 0.5,
        latency_ms: 100,
    };

    let summary = EvalSummary {
        mean_recall: 0.75,
        mean_precision: 0.65,
        mean_mrr: 0.55,
        p50_latency_ms: 80,
        p99_latency_ms: 200,
        query_count: 10,
        results: vec![result],
    };

    let json = serde_json::to_string(&summary).unwrap();
    let parsed: EvalSummary = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed.mean_recall, summary.mean_recall);
    assert_eq!(parsed.query_count, 10);
    assert_eq!(parsed.results.len(), 1);

    println!("✅ EvalSummary serialization test passed");
}

// ============================================================================
// Diff Module Tests
// ============================================================================

#[test]
fn test_diff_identical_working_sets() {
    let ws = create_test_working_set(vec![("1", "a.md", 0.9), ("2", "b.md", 0.8)]);

    let diff = avocado_core::diff::diff_working_sets(&ws, &ws);

    assert!(diff.added.is_empty(), "No spans should be added");
    assert!(diff.removed.is_empty(), "No spans should be removed");
    assert!(diff.reranked.is_empty(), "No spans should be reranked");
    assert_eq!(diff.before_hash, diff.after_hash);

    println!("✅ Identical working sets diff test passed");
}

#[test]
fn test_diff_added_spans() {
    let before = create_test_working_set(vec![("1", "a.md", 0.9)]);
    let after = create_test_working_set(vec![("1", "a.md", 0.9), ("2", "b.md", 0.8)]);

    let diff = avocado_core::diff::diff_working_sets(&before, &after);

    assert_eq!(diff.added.len(), 1);
    assert_eq!(diff.added[0].span_id, "2");
    assert!(diff.removed.is_empty());

    println!("✅ Added spans diff test passed");
}

#[test]
fn test_diff_removed_spans() {
    let before = create_test_working_set(vec![("1", "a.md", 0.9), ("2", "b.md", 0.8)]);
    let after = create_test_working_set(vec![("1", "a.md", 0.9)]);

    let diff = avocado_core::diff::diff_working_sets(&before, &after);

    assert!(diff.added.is_empty());
    assert_eq!(diff.removed.len(), 1);
    assert_eq!(diff.removed[0].span_id, "2");

    println!("✅ Removed spans diff test passed");
}

#[test]
fn test_diff_reranked_spans() {
    let before = create_test_working_set(vec![("1", "a.md", 0.9), ("2", "b.md", 0.8)]);
    let after = create_test_working_set(vec![("2", "b.md", 0.95), ("1", "a.md", 0.85)]);

    let diff = avocado_core::diff::diff_working_sets(&before, &after);

    assert!(diff.added.is_empty());
    assert!(diff.removed.is_empty());
    assert_eq!(diff.reranked.len(), 2);

    // Find the reranked entry for span "1"
    let span1_rerank = diff.reranked.iter().find(|r| r.span_id == "1").unwrap();
    assert_eq!(span1_rerank.old_rank, 1);
    assert_eq!(span1_rerank.new_rank, 2);

    println!("✅ Reranked spans diff test passed");
}

#[test]
fn test_diff_summarize() {
    let before = create_test_working_set(vec![("1", "a.md", 0.9)]);
    let after = create_test_working_set(vec![("2", "b.md", 0.8)]);

    let diff = avocado_core::diff::diff_working_sets(&before, &after);
    let summary = avocado_core::diff::summarize_diff(&diff);

    assert!(summary.contains("added"));
    assert!(summary.contains("removed"));

    println!("✅ Diff summarize test passed");
}

#[test]
fn test_working_sets_identical() {
    let ws1 = create_test_working_set(vec![("1", "a.md", 0.9)]);
    let ws2 = create_test_working_set(vec![("1", "a.md", 0.9)]);

    assert!(avocado_core::diff::working_sets_identical(&ws1, &ws2));

    let ws3 = create_test_working_set(vec![("2", "b.md", 0.8)]);
    assert!(!avocado_core::diff::working_sets_identical(&ws1, &ws3));

    println!("✅ Working sets identical test passed");
}

// ============================================================================
// Integration Tests
// ============================================================================

#[tokio::test]
async fn test_full_pipeline_with_all_features() {
    let (_temp_dir, db) = setup_test_db().await;

    // Ingest multiple documents
    for i in 0..3 {
        let content = format!(
            "Document {} covering topics like authentication, authorization, and security.\n",
            i
        )
        .repeat(30);
        ingest_test_document(&db, &format!("security{}.md", i), &content).await;
    }

    let index = db.get_vector_index().unwrap();
    let config = CompilerConfig::default();

    // Compile with explain
    let ws = compiler::compile_with_options(
        "authentication security",
        config,
        &db,
        index.as_ref(),
        None,
        true,
    )
    .await
    .unwrap();

    // Verify all features are present
    assert!(ws.manifest.is_some(), "Manifest should be present");
    assert!(ws.explain.is_some(), "Explain should be present");
    assert!(!ws.citations.is_empty(), "Should have citations");
    assert!(ws.tokens_used > 0, "Should have used tokens");

    let manifest = ws.manifest.as_ref().unwrap();
    let explain = ws.explain.as_ref().unwrap();

    // Cross-verify manifest hash matches working set hash
    assert_eq!(manifest.context_hash, ws.deterministic_hash());

    // Verify explain timing adds up reasonably
    let timing = &explain.timing;
    let sum = timing.embed_query_ms
        + timing.semantic_search_ms
        + timing.lexical_search_ms
        + timing.fusion_ms
        + timing.mmr_ms
        + timing.packing_ms
        + timing.build_context_ms;

    // Total should be >= sum of parts (allow some overhead)
    assert!(
        timing.total_ms >= sum.saturating_sub(10),
        "Timing should be consistent"
    );

    println!("✅ Full pipeline integration test passed");
}

#[tokio::test]
async fn test_incremental_rebuild_flow() {
    let (_temp_dir, db) = setup_test_db().await;

    let path = "incremental.md";
    let content_v1 = "Version 1 content about APIs.\n".repeat(30);
    let content_v2 = "Version 2 updated content about REST APIs.\n".repeat(30);

    // First ingest
    let hash_v1 = format!("{:x}", Sha256::digest(content_v1.as_bytes()));
    let action1 = db.determine_ingest_action(path, &hash_v1).unwrap();
    assert!(matches!(action1, IngestAction::Create));

    ingest_test_document(&db, path, &content_v1).await;

    // Same content should skip
    let action2 = db.determine_ingest_action(path, &hash_v1).unwrap();
    assert!(matches!(action2, IngestAction::Skip { .. }));

    // Different content should update
    let hash_v2 = format!("{:x}", Sha256::digest(content_v2.as_bytes()));
    let action3 = db.determine_ingest_action(path, &hash_v2).unwrap();
    assert!(matches!(action3, IngestAction::Update { .. }));

    println!("✅ Incremental rebuild flow test passed");
}

// ============================================================================
// Helper Functions
// ============================================================================

fn create_test_working_set(citations: Vec<(&str, &str, f32)>) -> WorkingSet {
    let cites: Vec<avocado_core::Citation> = citations
        .into_iter()
        .map(|(id, path, score)| avocado_core::Citation {
            span_id: id.to_string(),
            artifact_id: "art".to_string(),
            artifact_path: path.to_string(),
            start_line: 1,
            end_line: 10,
            score,
        })
        .collect();

    WorkingSet {
        text: cites
            .iter()
            .map(|c| c.artifact_path.as_str())
            .collect::<Vec<_>>()
            .join(","),
        spans: vec![],
        citations: cites,
        tokens_used: 100,
        query: "test".to_string(),
        compilation_time_ms: 50,
        manifest: None,
        explain: None,
    }
}
