//! Determinism tests for AvocadoDB
//!
//! The most critical test: same query → same result, every time.

use avocado_core::{compiler, db::Database, span, Artifact, CompilerConfig};
use sha2::{Digest, Sha256};
use uuid::Uuid;

#[tokio::test]
#[ignore] // Requires OPENAI_API_KEY
async fn test_deterministic_compilation() {
    // Setup test database
    let db = Database::new(":memory:").expect("Failed to create database");

    // Create test artifact
    let artifact_id = Uuid::new_v4().to_string();
    let content = create_test_document();
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

    // Extract and embed spans
    let mut spans = span::extract_spans(&content, &artifact_id).unwrap();

    // Note: This test requires OPENAI_API_KEY to be set
    let texts: Vec<&str> = spans.iter().map(|s| s.text.as_str()).collect();
    let embeddings = avocado_core::embedding::embed_batch(texts, None)
        .await
        .expect("Failed to embed (is OPENAI_API_KEY set?)");

    for (span, emb) in spans.iter_mut().zip(embeddings.iter()) {
        span.embedding = Some(emb.clone());
        span.embedding_model = Some("text-embedding-ada-002".to_string());
    }

    db.insert_spans(&spans).unwrap();

    // Get cached index
    let index = db.get_vector_index().unwrap();

    // Run compilation 100 times
    let query = "How does authentication work?";
    let config = CompilerConfig::default();

    let mut hashes = Vec::new();
    for _ in 0..100 {
        let result = compiler::compile(query, config.clone(), &db, index.as_ref(), None)
            .await
            .unwrap();

        let hash = result.deterministic_hash();
        hashes.push(hash);
    }

    // All hashes should be identical
    let first_hash = &hashes[0];
    assert!(
        hashes.iter().all(|h| h == first_hash),
        "Results were not deterministic! Got {} unique hashes",
        hashes.iter().collect::<std::collections::HashSet<_>>().len()
    );

    println!("✅ Passed determinism test: 100 identical results");
}

#[tokio::test]
#[ignore] // Requires OPENAI_API_KEY
async fn test_determinism_with_different_instances() {
    // Test that even with fresh database instances, results are identical

    let content = create_test_document();
    let query = "authentication";
    let config = CompilerConfig::default();

    let mut hashes = Vec::new();

    // Use deterministic artifact_id based on content hash for true determinism
    let content_hash = format!("{:x}", Sha256::digest(content.as_bytes()));
    let artifact_id = format!("{:x}", Sha256::digest(format!("artifact_{}", content_hash).as_bytes()));

    for _i in 0..3 {
        // Create fresh database
        let db = Database::new(":memory:").unwrap();

        // Ingest same content with same artifact_id and path for determinism
        // (path is included in citations, so must be the same for deterministic results)
        let artifact = Artifact {
            id: artifact_id.clone(),
            path: "test.md".to_string(), // Same path for all iterations
            content: content.clone(),
            content_hash: content_hash.clone(),
            metadata: None,
            created_at: chrono::Utc::now(),
        };

        db.insert_artifact(&artifact).unwrap();

        let mut spans = span::extract_spans(&content, &artifact_id).unwrap();

        let texts: Vec<&str> = spans.iter().map(|s| s.text.as_str()).collect();
        let embeddings = avocado_core::embedding::embed_batch(texts, None)
            .await
            .unwrap();

        for (span, emb) in spans.iter_mut().zip(embeddings.iter()) {
            span.embedding = Some(emb.clone());
            span.embedding_model = Some("text-embedding-ada-002".to_string());
        }

        db.insert_spans(&spans).unwrap();

        // Get cached index
        let index = db.get_vector_index().unwrap();

        let result = compiler::compile(query, config.clone(), &db, index.as_ref(), None)
            .await
            .unwrap();

        hashes.push(result.deterministic_hash());
    }

    // All should be identical
    assert_eq!(
        hashes.iter().collect::<std::collections::HashSet<_>>().len(),
        1,
        "Different database instances produced different results"
    );

    println!("✅ Passed determinism test across instances");
}

fn create_test_document() -> String {
    r#"# Authentication Guide

Our system uses JWT-based authentication for secure access control.

## Overview

Users must authenticate with valid credentials to access protected endpoints.
The authentication flow follows industry best practices.

## Login Process

1. User submits credentials to /api/login
2. Server validates username and password
3. On success, server issues JWT token
4. Token is valid for 24 hours

## Using Tokens

Include the token in the Authorization header:
```
Authorization: Bearer <token>
```

## Security Considerations

- Tokens should be stored securely
- Use HTTPS in production
- Implement rate limiting
- Rotate secrets regularly

"#
    .to_string()
}
