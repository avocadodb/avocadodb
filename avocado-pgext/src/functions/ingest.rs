//! Document ingestion functions for AvocadoDB extension

use crate::embedding::{embed_batch_sync, model_name};
use crate::error::AvocadoError;
use crate::spi::{self, Artifact, Span};
use pgrx::prelude::*;
use sha2::{Digest, Sha256};

/// Default chunking parameters
const MIN_LINES: usize = 10;
const MAX_LINES: usize = 50;
const TARGET_LINES: usize = 25;

/// Ingest a document artifact
///
/// # Arguments
/// * `path` - Unique path/identifier for the document
/// * `content` - Full text content of the document
/// * `metadata` - Optional JSONB metadata
///
/// # Returns
/// The artifact ID (TEXT)
///
/// # Example
/// ```sql
/// SELECT avocado_ingest_artifact(
///     'docs/auth.md',
///     'Authentication is handled via JWT tokens...',
///     '{"type": "markdown", "version": "1.0"}'::jsonb
/// );
/// ```
#[pg_extern]
fn avocado_ingest_artifact(
    path: &str,
    content: &str,
    metadata: default!(Option<JsonB>, "NULL"),
) -> String {
    match ingest_artifact_impl(path, content, metadata.map(|j| j.0)) {
        Ok(artifact_id) => artifact_id,
        Err(e) => e.report(),
    }
}

fn ingest_artifact_impl(
    path: &str,
    content: &str,
    metadata: Option<serde_json::Value>,
) -> Result<String, AvocadoError> {
    // Calculate content hash
    let content_hash = format!("{:x}", Sha256::digest(content.as_bytes()));
    let artifact_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now();

    // Create artifact
    let artifact = Artifact {
        id: artifact_id.clone(),
        path: path.to_string(),
        content: content.to_string(),
        content_hash,
        metadata,
        created_at: now,
    };

    // Delete old spans if updating
    let _ = spi::delete_spans_for_artifact(&artifact_id);

    // Insert artifact (upsert on path)
    spi::insert_artifact(&artifact)?;

    // Extract spans from content
    let spans = extract_spans(content, &artifact_id)?;

    if !spans.is_empty() {
        // Generate embeddings for all spans
        let texts: Vec<&str> = spans.iter().map(|s| s.text.as_str()).collect();
        let embeddings = embed_batch_sync(&texts)?;

        // Add embeddings to spans
        let spans_with_embeddings: Vec<Span> = spans
            .into_iter()
            .zip(embeddings.into_iter())
            .map(|(mut span, embedding)| {
                span.embedding = Some(embedding);
                span.embedding_model = Some(model_name().to_string());
                span
            })
            .collect();

        // Insert spans
        spi::insert_spans(&spans_with_embeddings)?;
    }

    Ok(artifact_id)
}

/// Extract spans from content using simple line-based chunking
fn extract_spans(content: &str, artifact_id: &str) -> Result<Vec<Span>, AvocadoError> {
    let lines: Vec<&str> = content.lines().collect();
    let mut spans = Vec::new();

    if lines.is_empty() {
        return Ok(spans);
    }

    let mut start_line = 0usize;

    while start_line < lines.len() {
        // Determine chunk size (aim for TARGET_LINES, respect MIN/MAX)
        let remaining = lines.len() - start_line;
        let chunk_size = if remaining <= MAX_LINES {
            remaining
        } else {
            // Try to find a good break point near TARGET_LINES
            let target_end = (start_line + TARGET_LINES).min(lines.len());
            let mut best_break = target_end;

            // Look for blank lines near target
            for i in (start_line + MIN_LINES..=target_end.min(lines.len())).rev() {
                if i < lines.len() && lines[i].trim().is_empty() {
                    best_break = i;
                    break;
                }
            }

            best_break - start_line
        };

        let end_line = start_line + chunk_size;
        let text: String = lines[start_line..end_line].join("\n");

        // Estimate token count (rough approximation: ~4 chars per token)
        let token_count = (text.len() / 4).max(1) as i32;

        let span = Span {
            id: uuid::Uuid::new_v4().to_string(),
            artifact_id: artifact_id.to_string(),
            start_line: (start_line + 1) as i32, // 1-indexed
            end_line: end_line as i32,
            text,
            embedding: None,
            embedding_model: None,
            token_count: Some(token_count),
        };

        spans.push(span);
        start_line = end_line;
    }

    Ok(spans)
}

/// Delete an artifact and its spans
///
/// # Arguments
/// * `artifact_id` - The artifact ID to delete
///
/// # Returns
/// Number of spans deleted
#[pg_extern]
fn avocado_delete_artifact(artifact_id: &str) -> i64 {
    match spi::delete_spans_for_artifact(artifact_id) {
        Ok(count) => count,
        Err(e) => e.report(),
    }
}

/// Search spans by text query
///
/// Performs vector similarity search using the query embedding.
///
/// # Arguments
/// * `query` - Search query text
/// * `limit` - Maximum number of results (default 10)
///
/// # Returns
/// JSONB array of matching spans with scores
#[pg_extern]
fn avocado_search_spans(query: &str, limit: default!(i32, "10")) -> JsonB {
    match search_spans_impl(query, limit) {
        Ok(results) => JsonB(serde_json::json!(results)),
        Err(e) => e.report(),
    }
}

fn search_spans_impl(query: &str, limit: i32) -> Result<Vec<serde_json::Value>, AvocadoError> {
    // Embed the query
    let query_embedding = crate::embedding::embed_sync(query)?;

    // Search for similar spans
    let spans = spi::search_similar_spans(&query_embedding, limit)?;

    // Convert to JSON
    let results: Vec<serde_json::Value> = spans
        .into_iter()
        .map(|s| {
            serde_json::json!({
                "span_id": s.span.id,
                "artifact_path": s.artifact_path,
                "lines": [s.span.start_line, s.span.end_line],
                "text": s.span.text,
                "score": s.score,
                "tokens": s.span.token_count,
            })
        })
        .collect();

    Ok(results)
}
