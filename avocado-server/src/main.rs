//! AvocadoDB HTTP Server
//!
//! MongoDB-style daemon: One server managing multiple project indexes.
//! Each project has its own database and in-memory HNSW index.

use avocado_core::{compiler, db::Database, embedding, index::VectorIndex, span, Artifact, CompilerConfig};
use axum::{
    extract::{Json, Query, State},
    http::StatusCode,
    routing::{delete, get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use sha2::Digest;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;
use uuid::Uuid;

/// Maximum number of projects to keep in memory (LRU eviction)
const MAX_PROJECTS_IN_MEMORY: usize = 10;

/// A project's index (database + in-memory HNSW)
struct ProjectIndex {
    database: Database,
    hnsw_index: Arc<VectorIndex>,
    last_accessed: Arc<RwLock<Instant>>,  // Mutable last_accessed
    project_path: PathBuf,
}

/// Shared application state - manages multiple projects
struct AppState {
    projects: Arc<RwLock<HashMap<PathBuf, Arc<ProjectIndex>>>>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    // Initialize empty project manager
    let state = Arc::new(AppState {
        projects: Arc::new(RwLock::new(HashMap::new())),
    });

    // Build router
    let app = Router::new()
        .route("/compile", post(compile_handler))
        .route("/ingest", post(ingest_handler))
        .route("/ingest/batch", post(ingest_batch_handler))
        .route("/stats", get(stats_handler))
        .route("/clear", delete(clear_handler))
        .route("/health", get(health_handler))
        .layer(CorsLayer::permissive())
        .with_state(state);

    // Start server
    let port = std::env::var("PORT").unwrap_or_else(|_| "8765".to_string());
    let addr = format!("0.0.0.0:{}", port);
    println!("🥑 AvocadoDB daemon listening on http://{}", addr);
    println!("   Managing multiple projects (MongoDB-style)");
    println!("   Max projects in memory: {}", MAX_PROJECTS_IN_MEMORY);

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .map_err(|e| format!("Failed to bind to {}: {}", addr, e))?;

    axum::serve(listener, app)
        .await
        .map_err(|e| format!("Server error: {}", e))?;

    Ok(())
}

// ===== Request/Response Types =====

#[derive(Debug, Deserialize)]
struct CompileRequest {
    query: String,
    #[serde(default = "default_token_budget")]
    token_budget: usize,
    #[serde(default)]
    config: Option<CompilerConfig>,
    /// Project path (directory containing .avocado/db.sqlite)
    /// If not provided, uses current working directory
    project: Option<String>,
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
    /// Project path (directory containing .avocado/db.sqlite)
    project: Option<String>,
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
    // Get or load project index
    let project_index = get_or_load_project(&state, &req.project)
        .await
        .map_err(|e| internal_error(e.to_string()))?;

    // Use provided config or default
    let config = req.config.unwrap_or_else(|| CompilerConfig {
        token_budget: req.token_budget,
        ..Default::default()
    });

    // Compile context using project's index
    let working_set = compiler::compile(
        &req.query,
        config,
        &project_index.database,
        project_index.hnsw_index.as_ref(),
        None,
    )
    .await
    .map_err(|e| internal_error(e.to_string()))?;

    Ok(Json(CompileResponse { working_set }))
}

async fn ingest_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<IngestRequest>,
) -> Result<Json<IngestResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Get or load project index
    let project_index = get_or_load_project(&state, &req.project)
        .await
        .map_err(|e| internal_error(e.to_string()))?;

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

    project_index
        .database
        .insert_artifact(&artifact)
        .map_err(|e| internal_error(e.to_string()))?;

    // Extract spans
    let mut spans = span::extract_spans(&req.content, &artifact_id)
        .map_err(|e| internal_error(e.to_string()))?;

    // Embed spans
    let texts: Vec<&str> = spans.iter().map(|s| s.text.as_str()).collect();
    let embeddings = embedding::embed_batch(texts, None, None)
        .await
        .map_err(|e| internal_error(e.to_string()))?;

    for (span, emb) in spans.iter_mut().zip(embeddings.iter()) {
        span.embedding = Some(emb.clone());
        span.embedding_model = Some(embedding::embedding_model().to_string());
    }

    let tokens_indexed: usize = spans.iter().map(|s| s.token_count).sum();
    let spans_created = spans.len();

    project_index
        .database
        .insert_spans(&spans)
        .map_err(|e| internal_error(e.to_string()))?;

    // Invalidate index (will be rebuilt on next compile)
    invalidate_project_index(&state, &req.project).await;

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
    // Get project from first document (all should be same project)
    let project = req.documents.first().and_then(|d| d.project.clone());
    let project_index = get_or_load_project(&state, &project)
        .await
        .map_err(|e| internal_error(e.to_string()))?;

    let mut results = Vec::new();

    for doc in req.documents {
        // Process each document individually
        let result = match process_single_ingest(&project_index.database, doc).await {
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

    // Invalidate index after batch ingest
    invalidate_project_index(&state, &project).await;

    Ok(Json(IngestBatchResponse { results }))
}

async fn stats_handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<StatsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let project = params.get("project").cloned();
    let project_index = get_or_load_project(&state, &project)
        .await
        .map_err(|e| internal_error(e.to_string()))?;

    let (artifacts_count, spans_count, total_tokens) = project_index
        .database
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
    Query(params): Query<HashMap<String, String>>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let project = params.get("project").cloned();
    let project_index = get_or_load_project(&state, &project)
        .await
        .map_err(|e| internal_error(e.to_string()))?;

    project_index
        .database
        .clear()
        .map_err(|e| internal_error(e.to_string()))?;

    // Remove from cache (will be reloaded on next access)
    if let Some(project_path) = project {
        let project_path = normalize_project_path(&project_path);
        state.projects.write().await.remove(&project_path);
    }

    Ok(StatusCode::OK)
}

async fn health_handler() -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    Ok(Json(serde_json::json!({
        "status": "ok",
        "service": "avocadodb-daemon"
    })))
}

// ===== Helpers =====

// ===== Project Management =====

/// Get or load a project index (with LRU eviction)
async fn get_or_load_project(
    state: &Arc<AppState>,
    project: &Option<String>,
) -> anyhow::Result<Arc<ProjectIndex>> {
    let project_path = normalize_project_path(project.as_deref().unwrap_or("."));

    // Check if already loaded
    {
        let projects = state.projects.read().await;
        if let Some(proj) = projects.get(&project_path) {
            // Update last accessed time
            *proj.last_accessed.write().await = Instant::now();
            return Ok(Arc::clone(proj));
        }
    }

    // Need to load the project
    let db_path = project_path.join(".avocado/db.sqlite");
    
    // Create database if it doesn't exist
    if !db_path.exists() {
        std::fs::create_dir_all(db_path.parent().unwrap())?;
    }

    let database = Database::new(&db_path)
        .map_err(|e| anyhow::anyhow!("Failed to load database at {:?}: {}", db_path, e))?;

    // Build or load index
    let hnsw_index = database
        .get_vector_index()
        .map_err(|e| anyhow::anyhow!("Failed to build index: {}", e))?;

    let project_index = ProjectIndex {
        database,
        hnsw_index,
        last_accessed: Arc::new(RwLock::new(Instant::now())),
        project_path: project_path.clone(),
    };

    // Insert into cache (with LRU eviction)
    {
        let mut projects = state.projects.write().await;
        
        // Evict least recently used if at capacity
        if projects.len() >= MAX_PROJECTS_IN_MEMORY {
            evict_lru_project(&mut projects).await;
        }

        // Store the new project
        let index_arc = Arc::new(project_index);
        projects.insert(project_path, Arc::clone(&index_arc));
        Ok(index_arc)
    }
}

/// Normalize project path to absolute PathBuf
fn normalize_project_path(project: &str) -> PathBuf {
    let path = PathBuf::from(project);
    if path.is_absolute() {
        path.canonicalize().unwrap_or(path)
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path)
            .canonicalize()
            .unwrap_or_else(|_| PathBuf::from("."))
    }
}

/// Evict the least recently used project
async fn evict_lru_project(projects: &mut HashMap<PathBuf, Arc<ProjectIndex>>) {
    if projects.is_empty() {
        return;
    }

    // Find LRU by reading last_accessed from each project
    let mut lru_key: Option<PathBuf> = None;
    let mut lru_time = Instant::now();

    for (key, proj) in projects.iter() {
        let accessed = *proj.last_accessed.read().await;
        if accessed < lru_time {
            lru_time = accessed;
            lru_key = Some(key.clone());
        }
    }

    if let Some(key) = lru_key {
        projects.remove(&key);
        eprintln!("Evicted project from memory: {:?}", key);
    }
}

/// Invalidate a project's index (will be rebuilt on next access)
async fn invalidate_project_index(state: &Arc<AppState>, project: &Option<String>) {
    let project_path = normalize_project_path(project.as_deref().unwrap_or("."));
    // Just remove from cache - it will be reloaded with fresh index on next access
    state.projects.write().await.remove(&project_path);
}

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
    let embeddings = embedding::embed_batch(texts, None, None).await?;

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
