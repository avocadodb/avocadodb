//! Correctness tests for AvocadoDB
//!
//! Verify that compilation results are correct and meet all constraints.

use avocado_core::{compiler, db::Database, span, Artifact, CompilerConfig};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use uuid::Uuid;

#[tokio::test]
#[ignore] // Requires OPENAI_API_KEY
async fn test_no_duplicate_spans() {
    let db = setup_test_db().await;
    let index = db.get_vector_index().unwrap();

    let config = CompilerConfig::default();
    let result = compiler::compile("test query", config, &db, index.as_ref(), None)
        .await
        .unwrap();

    // Check for duplicate span IDs
    let mut seen_ids = HashSet::new();
    for span in &result.spans {
        assert!(
            seen_ids.insert(&span.id),
            "Duplicate span found: {}",
            span.id
        );
    }

    println!("✅ No duplicate spans");
}

#[tokio::test]
#[ignore] // Requires OPENAI_API_KEY
async fn test_token_budget_respected() {
    let db = setup_test_db().await;
    let index = db.get_vector_index().unwrap();

    let budget = 1000;
    let config = CompilerConfig {
        token_budget: budget,
        ..Default::default()
    };

    let result = compiler::compile("test query", config, &db, index.as_ref(), None)
        .await
        .unwrap();

    assert!(
        result.tokens_used <= budget,
        "Token budget exceeded: {} > {}",
        result.tokens_used,
        budget
    );

    println!(
        "✅ Token budget respected: {}/{}",
        result.tokens_used, budget
    );
}

#[tokio::test]
#[ignore] // Requires OPENAI_API_KEY
async fn test_citations_valid() {
    let db = setup_test_db().await;
    let index = db.get_vector_index().unwrap();

    let config = CompilerConfig::default();
    let result = compiler::compile("authentication", config, &db, index.as_ref(), None)
        .await
        .unwrap();

    for citation in &result.citations {
        // Citation should point to an actual span
        let span_exists = result.spans.iter().any(|s| s.id == citation.span_id);
        assert!(
            span_exists,
            "Citation references non-existent span: {}",
            citation.span_id
        );

        // Line numbers should be valid
        assert!(citation.start_line > 0, "Invalid start_line");
        assert!(
            citation.end_line >= citation.start_line,
            "end_line < start_line"
        );

        // Artifact should exist
        let artifact = db.get_artifact(&citation.artifact_id).unwrap();
        assert!(artifact.is_some(), "Citation references missing artifact");
    }

    println!("✅ All citations valid: {}", result.citations.len());
}

#[tokio::test]
#[ignore] // Requires OPENAI_API_KEY
async fn test_spans_no_gaps_or_overlaps() {
    let artifact_id = Uuid::new_v4().to_string();
    let content = (0..100)
        .map(|i| format!("Line {}", i))
        .collect::<Vec<_>>()
        .join("\n");

    let spans = span::extract_spans(&content, &artifact_id).unwrap();

    // Group spans by artifact
    let mut spans_by_artifact: std::collections::HashMap<String, Vec<_>> =
        std::collections::HashMap::new();

    for span in spans {
        spans_by_artifact
            .entry(span.artifact_id.clone())
            .or_default()
            .push(span);
    }

    // Check each artifact's spans
    for (artifact_id, mut artifact_spans) in spans_by_artifact {
        // Sort by start_line
        artifact_spans.sort_by_key(|s| s.start_line);

        // Verify no gaps or overlaps
        for i in 0..artifact_spans.len() - 1 {
            let current = &artifact_spans[i];
            let next = &artifact_spans[i + 1];

            assert_eq!(
                current.end_line + 1,
                next.start_line,
                "Gap or overlap detected in artifact {}: span {} ends at {}, next starts at {}",
                artifact_id,
                current.id,
                current.end_line,
                next.start_line
            );
        }
    }

    println!("✅ Spans have no gaps or overlaps");
}

#[tokio::test]
async fn test_span_token_counts() {
    let content = "Hello world! This is a test.";
    let artifact_id = Uuid::new_v4().to_string();

    let spans = span::extract_spans(content, &artifact_id).unwrap();

    for span in &spans {
        assert!(
            span.token_count > 0,
            "Span has zero token count: {}",
            span.id
        );

        // Token count should be reasonable (not wildly off)
        let chars = span.text.len();
        let estimated_tokens = chars / 4;
        assert!(
            span.token_count >= estimated_tokens / 2 && span.token_count <= estimated_tokens * 2,
            "Token count seems wrong: {} tokens for {} chars",
            span.token_count,
            chars
        );
    }

    println!("✅ Span token counts look reasonable");
}

// Helper function to setup test database
async fn setup_test_db() -> Database {
    let db = Database::new(":memory:").unwrap();

    let artifact_id = Uuid::new_v4().to_string();
    let content = r#"
# Test Document

This is a test document about authentication and security.

## Authentication

Users log in with username and password.
Tokens are issued for authenticated sessions.

## Security

- Use HTTPS
- Rotate secrets
- Implement rate limiting
"#
    .to_string();

    let content_hash = format!("{:x}", Sha256::digest(content.as_bytes()));

    let artifact = Artifact {
        id: artifact_id.clone(),
        path: "test.md".to_string(),
        content: content.clone(),
        content_hash,
        metadata: None,
        created_at: chrono::Utc::now(),
    };

    db.insert_artifact(&artifact).unwrap();

    let mut spans = span::extract_spans(&content, &artifact_id).unwrap();

    let texts: Vec<&str> = spans.iter().map(|s| s.text.as_str()).collect();
    let embeddings = avocado_core::embedding::embed_batch(texts, None, None)
        .await
        .unwrap();

    for (span, emb) in spans.iter_mut().zip(embeddings.iter()) {
        span.embedding = Some(emb.clone());
        span.embedding_model = Some("text-embedding-ada-002".to_string());
    }

    db.insert_spans(&spans).unwrap();

    db
}
