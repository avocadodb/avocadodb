//! Performance tests for AvocadoDB
//!
//! Ensure compilation meets speed requirements.

use avocado_core::{compiler, db::Database, span, Artifact, CompilerConfig};
use uuid::Uuid;

#[tokio::test]
#[ignore] // Requires OPENAI_API_KEY and takes time to run
async fn test_compilation_performance() {
    // Setup large test database (1000 documents, ~10K spans)
    let db = Database::new(":memory:").expect("Failed to create database");

    println!("Setting up large test database...");

    for i in 0..100 {
        let artifact_id = Uuid::new_v4().to_string();
        let content = create_large_document(i);
        let content_hash = format!("{:x}", sha2::Sha256::digest(content.as_bytes()));

        let artifact = Artifact {
            id: artifact_id.clone(),
            path: format!("doc_{}.md", i),
            content: content.clone(),
            content_hash,
            metadata: None,
            created_at: chrono::Utc::now(),
        };

        db.insert_artifact(&artifact).unwrap();

        let mut spans = span::extract_spans(&content, &artifact_id).unwrap();

        if i % 10 == 0 {
            // Only embed every 10th document to save API costs during testing
            let texts: Vec<&str> = spans.iter().map(|s| s.text.as_str()).collect();
            let embeddings = avocado_core::embedding::embed_batch(texts, None)
                .await
                .unwrap();

            for (span, emb) in spans.iter_mut().zip(embeddings.iter()) {
                span.embedding = Some(emb.clone());
                span.embedding_model = Some("text-embedding-ada-002".to_string());
            }
        }

        db.insert_spans(&spans).unwrap();
    }

    let index = db.get_vector_index().unwrap();
    println!("Database ready: {} spans", index.len());

    // Test compilation performance
    let query = "complex technical query with multiple keywords about authentication and security";
    let config = CompilerConfig {
        token_budget: 8000,
        ..Default::default()
    };

    let start = std::time::Instant::now();
    let result = compiler::compile(query, config, &db, index.as_ref(), None)
        .await
        .unwrap();
    let duration = start.elapsed();

    println!("Compilation took: {}ms", duration.as_millis());
    println!("Tokens used: {}", result.tokens_used);
    println!("Spans included: {}", result.spans.len());

    // Should complete in under 500ms
    assert!(
        duration.as_millis() < 500,
        "Compilation took {}ms (expected < 500ms)",
        duration.as_millis()
    );

    // Should use most of the token budget (>87.5% utilization)
    assert!(
        result.tokens_used > 7000,
        "Low token utilization: {}/8000 ({}%)",
        result.tokens_used,
        (result.tokens_used * 100) / 8000
    );

    println!("✅ Passed performance test");
}

fn create_large_document(index: usize) -> String {
    format!(
        r#"# Document {}

This is a large test document with multiple sections.

## Section 1: Introduction

Lorem ipsum dolor sit amet, consectetur adipiscing elit.
This document contains various technical content about authentication,
security, APIs, and other topics.

## Section 2: Authentication

Users must authenticate with valid credentials.
The system uses JWT tokens for session management.
Tokens expire after 24 hours for security.

## Section 3: Security Best Practices

- Use HTTPS in production
- Implement rate limiting
- Rotate secrets regularly
- Use strong passwords
- Enable two-factor authentication

## Section 4: API Documentation

### GET /api/users
Returns a list of all users.

### POST /api/users
Creates a new user account.

### PUT /api/users/:id
Updates an existing user.

### DELETE /api/users/:id
Removes a user from the system.

## Section 5: Database Schema

The system uses the following tables:
- users: User account information
- sessions: Active user sessions
- audit_log: Security audit trail

## Section 6: Deployment

Deploy using Docker containers for consistency.
Configure environment variables for secrets.
Use a reverse proxy for SSL termination.

## Section 7: Monitoring

Set up logging and metrics:
- Application logs
- Error tracking
- Performance metrics
- Security alerts

## Section 8: Conclusion

This document provides comprehensive guidance for document {}.
"#,
        index, index
    )
}
