//! AvocadoDB HTTP Server
//!
//! Simple REST API for AvocadoDB.

use avocado_core::{compiler, db::Database, embedding, index::VectorIndex, span, Artifact, CompilerConfig};
use axum::{
    extract::{Json, State},
    http::StatusCode,
    routing::{delete, get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use sha2::Digest;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use uuid::Uuid;

/// Shared application state
struct AppState {
    db: Database,
}

#[tokio::main]
async fn main() {
    env_logger::init();

    // Initialize database
    let db = Database::new(".avocado/db.sqlite")
        .expect("Failed to initialize database");

    let state = Arc::new(AppState { db });

    // Build router
    let app = Router::new()
        .route("/compile", post(compile_handler))
        .route("/ingest", post(ingest_handler))
        .route("/ingest/batch", post(ingest_batch_handler))
        .route("/stats", get(stats_handler))
        .route("/clear", delete(clear_handler))
        .layer(CorsLayer::permissive())
        .with_state(state);

    // Start server
    let port = std::env::var("PORT").unwrap_or_else(|_| "8765".to_string());
    let addr = format!("0.0.0.0:{}", port);
    println!("AvocadoDB server listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("Failed to bind");

    axum::serve(listener, app)
        .await
        .expect("Server error");
}

// ===== Request/Response Types =====

#[derive(Debug, Deserialize)]
struct CompileRequest {
    query: String,
    #[serde(default = "default_token_budget")]
    token_budget: usize,
    #[serde(default)]
    config: Option<CompilerConfig>,
}

fn default_token_budget() -> usize {
    8000
}

#[derive(Debug, Serialize)]
struct CompileResponse {
    working_set: avocado_core::WorkingSet,
}

#[derive(Debug, Deserialize)]
struct IngestRequest {
    path: String,
    content: String,
    metadata: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
struct IngestResponse {
    artifact_id: String,
    spans_created: usize,
    tokens_indexed: usize,
}

#[derive(Debug, Deserialize)]
struct IngestBatchRequest {
    documents: Vec<IngestRequest>,
}

#[derive(Debug, Serialize)]
struct IngestBatchResponse {
    results: Vec<IngestResult>,
}

#[derive(Debug, Serialize)]
struct IngestResult {
    artifact_id: String,
    spans_created: usize,
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Debug, Serialize)]
struct StatsResponse {
    artifacts_count: usize,
    spans_count: usize,
    total_tokens: usize,
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
}

// ===== Handlers =====

async fn compile_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CompileRequest>,
) -> Result<Json<CompileResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Load spans and build index
    let spans = state
        .db
        .get_all_spans()
        .map_err(|e| internal_error(e.to_string()))?;

    let index = VectorIndex::build(spans);

    // Use provided config or default
    let config = req.config.unwrap_or_else(|| CompilerConfig {
        token_budget: req.token_budget,
        ..Default::default()
    });

    // Compile context
    let working_set = compiler::compile(&req.query, config, &state.db, &index, None)
        .await
        .map_err(|e| internal_error(e.to_string()))?;

    Ok(Json(CompileResponse { working_set }))
}

async fn ingest_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<IngestRequest>,
) -> Result<Json<IngestResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Create artifact
    let artifact_id = Uuid::new_v4().to_string();
    let content_hash = format!("{:x}", sha2::Sha256::digest(req.content.as_bytes()));

    let artifact = Artifact {
        id: artifact_id.clone(),
        path: req.path,
        content: req.content.clone(),
        content_hash,
        metadata: req.metadata,
        created_at: chrono::Utc::now(),
    };

    state
        .db
        .insert_artifact(&artifact)
        .map_err(|e| internal_error(e.to_string()))?;

    // Extract spans
    let mut spans = span::extract_spans(&req.content, &artifact_id)
        .map_err(|e| internal_error(e.to_string()))?;

    // Embed spans
    let texts: Vec<&str> = spans.iter().map(|s| s.text.as_str()).collect();
    let embeddings = embedding::embed_batch(texts, None)
        .await
        .map_err(|e| internal_error(e.to_string()))?;

    for (span, emb) in spans.iter_mut().zip(embeddings.iter()) {
        span.embedding = Some(emb.clone());
        span.embedding_model = Some(embedding::embedding_model().to_string());
    }

    let tokens_indexed: usize = spans.iter().map(|s| s.token_count).sum();
    let spans_created = spans.len();

    state
        .db
        .insert_spans(&spans)
        .map_err(|e| internal_error(e.to_string()))?;

    Ok(Json(IngestResponse {
        artifact_id,
        spans_created,
        tokens_indexed,
    }))
}

async fn ingest_batch_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<IngestBatchRequest>,
) -> Result<Json<IngestBatchResponse>, (StatusCode, Json<ErrorResponse>)> {
    let mut results = Vec::new();

    for doc in req.documents {
        // Process each document individually
        let result = match process_single_ingest(&state.db, doc).await {
            Ok((artifact_id, spans_created)) => IngestResult {
                artifact_id,
                spans_created,
                status: "success".to_string(),
                error: None,
            },
            Err(e) => IngestResult {
                artifact_id: "".to_string(),
                spans_created: 0,
                status: "error".to_string(),
                error: Some(e.to_string()),
            },
        };

        results.push(result);
    }

    Ok(Json(IngestBatchResponse { results }))
}

async fn stats_handler(
    State(state): State<Arc<AppState>>,
) -> Result<Json<StatsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let (artifacts_count, spans_count, total_tokens) = state
        .db
        .get_stats()
        .map_err(|e| internal_error(e.to_string()))?;

    Ok(Json(StatsResponse {
        artifacts_count,
        spans_count,
        total_tokens,
    }))
}

async fn clear_handler(
    State(state): State<Arc<AppState>>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    state
        .db
        .clear()
        .map_err(|e| internal_error(e.to_string()))?;

    Ok(StatusCode::OK)
}

// ===== Helpers =====

async fn process_single_ingest(
    db: &Database,
    req: IngestRequest,
) -> anyhow::Result<(String, usize)> {
    // Create artifact
    let artifact_id = Uuid::new_v4().to_string();
    let content_hash = format!("{:x}", sha2::Sha256::digest(req.content.as_bytes()));

    let artifact = Artifact {
        id: artifact_id.clone(),
        path: req.path,
        content: req.content.clone(),
        content_hash,
        metadata: req.metadata,
        created_at: chrono::Utc::now(),
    };

    db.insert_artifact(&artifact)?;

    // Extract and embed spans
    let mut spans = span::extract_spans(&req.content, &artifact_id)?;

    let texts: Vec<&str> = spans.iter().map(|s| s.text.as_str()).collect();
    let embeddings = embedding::embed_batch(texts, None).await?;

    for (span, emb) in spans.iter_mut().zip(embeddings.iter()) {
        span.embedding = Some(emb.clone());
        span.embedding_model = Some(embedding::embedding_model().to_string());
    }

    let spans_created = spans.len();
    db.insert_spans(&spans)?;

    Ok((artifact_id, spans_created))
}

fn internal_error(msg: String) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse { error: msg }),
    )
}
