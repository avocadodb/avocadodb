//! AvocadoDB HTTP Server
//!
//! Multi-project daemon server managing project indexes with LRU caching.
//! Each project has its own database and in-memory HNSW index.

use avocado_core::{
    compiler, db::Database, embedding, session::SessionManager, span, storage::SqliteBackend,
    Artifact, CompilerConfig, Message, MessageRole, Session, VERSION,
};
use axum::middleware;
use axum::{
    extract::{Json, Path, Query, State},
    http::{header, Method, StatusCode},
    routing::{delete, get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use sha2::Digest;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};
use std::time::Instant;
use tokio::sync::RwLock;
use tower_http::cors::{Any, CorsLayer};
use tower_http::limit::RequestBodyLimitLayer;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;
use uuid::Uuid;

/// Maximum number of projects to keep in memory (LRU eviction)
const MAX_PROJECTS_IN_MEMORY: usize = 10;

/// A project's index (database + in-memory HNSW + session manager)
struct ProjectIndex {
    database: Database,
    hnsw_index: Arc<avocado_core::index::VectorIndex>,
    session_manager: Arc<SessionManager>,
    last_accessed: Arc<RwLock<Instant>>,
    /// Storage backend for async operations (lazy-initialized)
    /// Ready for use with StorageBackend trait methods
    #[allow(dead_code)]
    backend: tokio::sync::OnceCell<SqliteBackend>,
}

impl ProjectIndex {
    /// Get the storage backend (lazy-initializes on first call)
    ///
    /// Use this method to access backend-agnostic async operations.
    /// The backend is lazily created from the Database on first access.
    #[allow(dead_code)]
    pub async fn get_backend(&self) -> Result<&SqliteBackend, anyhow::Error> {
        self.backend
            .get_or_try_init(|| async {
                self.database
                    .as_storage_backend()
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to create storage backend: {}", e))
            })
            .await
    }
}

/// Shared application state - manages multiple projects
struct AppState {
    projects: Arc<RwLock<HashMap<PathBuf, Arc<ProjectIndex>>>>,
    start_time: Instant,
    // Metrics
    compile_count: AtomicU64,
    total_compile_ms: AtomicU64,
    // Auth
    api_token: Option<String>,
}

#[derive(OpenApi)]
#[openapi(
    info(
        title = "AvocadoDB API",
        version = "0.1.0",
        description = "The first deterministic context database for AI agents"
    ),
    tags(
        (name = "Health", description = "Server health and status endpoints"),
        (name = "Context", description = "Context compilation and document ingestion"),
        (name = "Sessions", description = "Session management for multi-turn conversations"),
        (name = "Statistics", description = "Database statistics and metrics")
    )
)]
struct ApiDoc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    // Initialize empty project manager
    let state = Arc::new(AppState {
        projects: Arc::new(RwLock::new(HashMap::new())),
        start_time: Instant::now(),
        compile_count: AtomicU64::new(0),
        total_compile_ms: AtomicU64::new(0),
        api_token: std::env::var("API_TOKEN").ok().filter(|s| !s.is_empty()),
    });

    // Configure CORS
    let cors = if std::env::var("CORS_PERMISSIVE").is_ok() {
        // Permissive mode for development
        CorsLayer::permissive()
    } else {
        // Production-ready CORS with configurable origins
        let cors_origins = std::env::var("CORS_ALLOWED_ORIGINS").unwrap_or_else(|_| {
            "http://localhost:3000,http://localhost:8080,http://localhost:8765".to_string()
        });

        let origins: Vec<_> = cors_origins
            .split(',')
            .filter_map(|s| s.trim().parse().ok())
            .collect();

        if origins.is_empty() {
            // No specific origins configured, allow any
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods([Method::GET, Method::POST, Method::DELETE, Method::OPTIONS])
                .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE, header::ACCEPT])
                .allow_credentials(false)
        } else {
            // Use configured origins
            CorsLayer::new()
                .allow_origin(origins)
                .allow_methods([Method::GET, Method::POST, Method::DELETE, Method::OPTIONS])
                .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE, header::ACCEPT])
                .allow_credentials(false)
        }
    };

    // Request limits
    let max_body_bytes: usize = std::env::var("MAX_BODY_BYTES")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(2 * 1024 * 1024); // 2MB default

    // Build router
    let app = Router::new()
        .route("/compile", post(compile_handler))
        .route("/ingest", post(ingest_handler))
        .route("/ingest/batch", post(ingest_batch_handler))
        .route("/stats", get(stats_handler))
        .route("/clear", delete(clear_handler))
        .route("/health", get(health_handler))
        .route("/metrics", get(metrics_handler))
        // Session management endpoints
        .route("/sessions", post(create_session_handler))
        .route("/sessions", get(list_sessions_handler))
        .route("/sessions/:id", get(get_session_handler))
        .route("/sessions/:id", delete(delete_session_handler))
        .route("/sessions/:id/messages", post(add_message_handler))
        .route("/sessions/:id/compile", post(session_compile_handler))
        .route("/sessions/:id/history", get(session_history_handler))
        .route("/sessions/:id/replay", get(session_replay_handler))
        // Multi-agent orchestration endpoints
        .route("/agents", post(register_agent_handler))
        .route("/agents", get(list_agents_handler))
        .route("/agents/:agent_id", get(get_agent_handler))
        .route("/sessions/:id/relations", post(add_agent_relation_handler))
        .route("/sessions/:id/relations", get(get_agent_relations_handler))
        // API documentation endpoints
        .merge(SwaggerUi::new("/api-docs").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .layer(cors)
        .layer(RequestBodyLimitLayer::new(max_body_bytes))
        .layer(middleware::from_fn_with_state(
            Arc::clone(&state),
            auth_middleware,
        ))
        .with_state(state);

    // Start server
    let port = std::env::var("PORT").unwrap_or_else(|_| "8765".to_string());
    // Bind to localhost by default; allow override via BIND_ADDR for explicit exposure
    let bind_addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| "127.0.0.1".to_string());
    let addr = format!("{}:{}", bind_addr, port);
    println!("🥑 AvocadoDB v{} listening on http://{}", VERSION, addr);
    println!(
        "   Multi-project server with LRU caching (max {} projects)",
        MAX_PROJECTS_IN_MEMORY
    );
    println!(
        "   API Documentation: http://{}/api-docs",
        addr.replace("0.0.0.0", "localhost")
    );
    println!(
        "   OpenAPI Spec: http://{}/api-docs/openapi.json",
        addr.replace("0.0.0.0", "localhost")
    );

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
    /// Optional backend identifier (e.g., "bge-large-1024", "openai-1536")
    /// Reserved for future embedding backend selection
    #[serde(default)]
    #[allow(dead_code)]
    backend: Option<String>,
    /// Project path (directory containing .avocado/db.sqlite)
    /// If not provided, uses current working directory
    project: Option<String>,
    /// Whether to include explain plan in response
    #[serde(default)]
    explain: bool,
}

fn default_token_budget() -> usize {
    8000
}

#[derive(Debug, Serialize)]
struct CompileResponse {
    working_set: WorkingSetOut,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<serde_json::Value>,
}

impl ErrorResponse {
    #[allow(dead_code)]
    fn new(error: impl Into<String>) -> Self {
        Self {
            error: error.into(),
            code: None,
            details: None,
        }
    }

    fn with_code(error: impl Into<String>, code: impl Into<String>) -> Self {
        Self {
            error: error.into(),
            code: Some(code.into()),
            details: None,
        }
    }

    #[allow(dead_code)]
    fn with_details(
        error: impl Into<String>,
        code: impl Into<String>,
        details: serde_json::Value,
    ) -> Self {
        Self {
            error: error.into(),
            code: Some(code.into()),
            details: Some(details),
        }
    }
}

// ===== Session Request/Response Types =====

#[derive(Debug, Deserialize)]
struct CreateSessionRequest {
    user_id: Option<String>,
    title: Option<String>,
    /// Project path (directory containing .avocado/db.sqlite)
    project: Option<String>,
}

#[derive(Debug, Serialize)]
struct CreateSessionResponse {
    session: Session,
}

#[derive(Debug, Serialize)]
struct ListSessionsResponse {
    sessions: Vec<Session>,
}

#[derive(Debug, Serialize)]
struct GetSessionResponse {
    session: Session,
    messages: Vec<Message>,
}

#[derive(Debug, Deserialize)]
struct AddMessageRequest {
    role: String,
    content: String,
    metadata: Option<serde_json::Value>,
    /// Project path (directory containing .avocado/db.sqlite)
    project: Option<String>,
}

#[derive(Debug, Serialize)]
struct AddMessageResponse {
    message: Message,
}

#[derive(Debug, Deserialize)]
struct SessionCompileRequest {
    query: String,
    #[serde(default = "default_token_budget")]
    token_budget: usize,
    #[serde(default)]
    config: Option<CompilerConfig>,
    /// Project path (directory containing .avocado/db.sqlite)
    project: Option<String>,
}

#[derive(Debug, Serialize)]
struct SessionCompileResponse {
    message: Message,
    working_set: WorkingSetOut,
}

// ===== Multi-Agent Request/Response Types =====

#[derive(Debug, Deserialize)]
struct RegisterAgentRequest {
    /// Unique name for the agent (e.g., "moderator", "researcher")
    name: String,
    /// Agent's role/persona description
    role: String,
    /// LLM model identifier (e.g., "gpt-4", "claude-3", "qwen2.5:32b")
    model: String,
    /// Optional system prompt / personality
    system_prompt: Option<String>,
    /// Optional DID for decentralized identity
    did: Option<String>,
    /// Optional capabilities (e.g., ["web_search", "code_execution"])
    capabilities: Option<Vec<String>>,
    /// Project path
    project: Option<String>,
}

#[derive(Debug, Serialize)]
struct RegisterAgentResponse {
    agent: avocado_core::Agent,
}

#[derive(Debug, Serialize)]
struct ListAgentsResponse {
    agents: Vec<avocado_core::Agent>,
}

#[derive(Debug, Deserialize)]
struct AddAgentRelationRequest {
    /// The message ID that expresses the stance
    message_id: String,
    /// The agent ID who is expressing the stance
    from_agent_id: String,
    /// The target message ID being responded to
    target_message_id: String,
    /// The stance: "agree", "disagree", "neutral", "question"
    stance: String,
    /// Project path
    project: Option<String>,
}

#[derive(Debug, Serialize)]
struct AddAgentRelationResponse {
    relation: avocado_core::AgentRelation,
}

#[derive(Debug, Serialize)]
struct GetAgentRelationsResponse {
    relations: avocado_core::AgentRelationSummary,
    agents: Vec<avocado_core::Agent>,
}

// ===== Output Types (enriched spans) =====

#[derive(Debug, Serialize)]
struct SpanOut {
    id: String,
    artifact_id: String,
    artifact_path: String,
    start_line: usize,
    end_line: usize,
    text: String,
    token_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    embedding_model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    metadata: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    score: Option<f32>,
}

#[derive(Debug, Serialize)]
struct WorkingSetOut {
    text: String,
    spans: Vec<SpanOut>,
    citations: Vec<avocado_core::Citation>,
    tokens_used: usize,
    query: String,
    compilation_time_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    manifest: Option<avocado_core::Manifest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    explain: Option<avocado_core::ExplainPlan>,
}

fn enrich_working_set(ws: &avocado_core::WorkingSet, db: &Database) -> WorkingSetOut {
    // Build a small cache of artifact_id -> path
    use std::collections::HashMap;
    let mut path_cache: HashMap<String, String> = HashMap::new();
    // Seed with citations if present
    for c in &ws.citations {
        path_cache.insert(c.artifact_id.clone(), c.artifact_path.clone());
    }

    let spans_out: Vec<SpanOut> = ws
        .spans
        .iter()
        .map(|s| {
            // Lookup path from cache or DB
            let artifact_path = if let Some(p) = path_cache.get(&s.artifact_id) {
                p.clone()
            } else {
                match db.get_artifact(&s.artifact_id) {
                    Ok(opt) => {
                        let p = opt.map(|a| a.path).unwrap_or_else(|| "unknown".to_string());
                        path_cache.insert(s.artifact_id.clone(), p.clone());
                        p
                    }
                    Err(_) => "unknown".to_string(),
                }
            };

            SpanOut {
                id: s.id.clone(),
                artifact_id: s.artifact_id.clone(),
                artifact_path,
                start_line: s.start_line,
                end_line: s.end_line,
                text: s.text.clone(),
                token_count: s.token_count,
                embedding_model: s.embedding_model.clone(),
                metadata: s.metadata.clone(),
                score: None,
            }
        })
        .collect();

    WorkingSetOut {
        text: ws.text.clone(),
        spans: spans_out,
        citations: ws.citations.clone(),
        tokens_used: ws.tokens_used,
        query: ws.query.clone(),
        compilation_time_ms: ws.compilation_time_ms,
        manifest: ws.manifest.clone(),
        explain: ws.explain.clone(),
    }
}

#[derive(Debug, Serialize)]
struct ConversationHistoryResponse {
    history: String,
}

#[derive(Debug, Serialize)]
struct DeleteSessionResponse {
    success: bool,
}

// ===== Handlers =====

async fn compile_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CompileRequest>,
) -> Result<Json<CompileResponse>, (StatusCode, Json<ErrorResponse>)> {
    let t0 = Instant::now();
    // Get or load project index
    let project_index = get_or_load_project(&state, &req.project)
        .await
        .map_err(|e| internal_error(e.to_string()))?;

    // Use provided config or default
    let config = req.config.unwrap_or_else(|| CompilerConfig {
        token_budget: req.token_budget,
        ..Default::default()
    });

    // Compile context using project's index (with optional explain)
    let working_set = compiler::compile_with_options(
        &req.query,
        config,
        &project_index.database,
        project_index.hnsw_index.as_ref(),
        None,
        req.explain,
    )
    .await
    .map_err(|e| internal_error(e.to_string()))?;

    let ws_out = enrich_working_set(&working_set, &project_index.database);

    // Metrics
    let elapsed = t0.elapsed().as_millis() as u64;
    state.compile_count.fetch_add(1, Ordering::Relaxed);
    state.total_compile_ms.fetch_add(elapsed, Ordering::Relaxed);

    Ok(Json(CompileResponse {
        working_set: ws_out,
    }))
}

async fn ingest_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<IngestRequest>,
) -> Result<Json<IngestResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Get or load project index
    let project_index = get_or_load_project(&state, &req.project)
        .await
        .map_err(|e| internal_error(e.to_string()))?;

    // Calculate content hash
    let content_hash = format!("{:x}", sha2::Sha256::digest(req.content.as_bytes()));

    // Determine action based on content hash comparison
    let action = project_index
        .database
        .determine_ingest_action(&req.path, &content_hash)
        .map_err(|e| internal_error(e.to_string()))?;

    match action {
        avocado_core::IngestAction::Skip { artifact_id, .. } => {
            // Content unchanged, return existing artifact with 0 new spans
            return Ok(Json(IngestResponse {
                artifact_id,
                spans_created: 0,
                tokens_indexed: 0,
            }));
        }
        avocado_core::IngestAction::Update { artifact_id } => {
            // Content changed, delete old spans and re-ingest
            project_index
                .database
                .delete_artifact(&artifact_id)
                .map_err(|e| internal_error(e.to_string()))?;
            // Fall through to create new artifact
        }
        avocado_core::IngestAction::Create => {
            // New document, proceed normally
        }
    }

    // Create new artifact
    let artifact_id = Uuid::new_v4().to_string();
    let artifact = Artifact {
        id: artifact_id.clone(),
        path: req.path.clone(),
        content: req.content.clone(),
        content_hash,
        metadata: req.metadata.clone(),
        created_at: chrono::Utc::now(),
    };

    // Insert artifact
    project_index
        .database
        .insert_artifact(&artifact)
        .map_err(|e| internal_error(e.to_string()))?;

    // Extract spans
    let mut spans = span::extract_spans(&req.content, &artifact.id)
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
        artifact_id: artifact.id,
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
        let project_path = secure_normalize_project_path(&project_path)
            .unwrap_or_else(|_| normalize_project_path(&project_path));
        state.projects.write().await.remove(&project_path);
    }

    Ok(StatusCode::OK)
}

async fn health_handler(
    State(state): State<Arc<AppState>>,
) -> (StatusCode, Json<serde_json::Value>) {
    // Calculate uptime
    let uptime_seconds = state.start_time.elapsed().as_secs();

    // Check database status by trying to load default project
    let database_status = match get_or_load_project(&state, &None).await {
        Ok(_) => "ok",
        Err(_) => "degraded",
    };

    // Count loaded projects
    let projects_loaded = state.projects.read().await.len();

    let response = serde_json::json!({
        "status": if database_status == "ok" { "ok" } else { "degraded" },
        "service": "avocadodb-daemon",
        "version": VERSION,
        "uptime_seconds": uptime_seconds,
        "database_status": database_status,
        "projects_loaded": projects_loaded,
        "max_projects_in_memory": MAX_PROJECTS_IN_MEMORY,
    });

    let status_code = if database_status == "ok" {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    (status_code, Json(response))
}

async fn metrics_handler(
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let compile_count = state.compile_count.load(Ordering::Relaxed);
    let total_ms = state.total_compile_ms.load(Ordering::Relaxed);
    let avg_ms = if compile_count > 0 {
        total_ms / compile_count
    } else {
        0
    };
    let metrics = serde_json::json!({
        "compile_count": compile_count,
        "total_compile_ms": total_ms,
        "avg_compile_ms": avg_ms
    });
    Ok(Json(metrics))
}

// ===== Helpers =====

// ===== Project Management =====

/// Get or load a project index (with LRU eviction)
async fn get_or_load_project(
    state: &Arc<AppState>,
    project: &Option<String>,
) -> anyhow::Result<Arc<ProjectIndex>> {
    let project_path = secure_normalize_project_path(project.as_deref().unwrap_or("."))?;

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

    // Build or load index (measure and track whether loaded from cache)
    let t0 = Instant::now();
    let (hnsw_index, load_kind) = database
        .get_vector_index_with_kind()
        .map_err(|e| anyhow::anyhow!("Failed to build/load index: {}", e))?;
    let elapsed_ms = t0.elapsed().as_millis() as u64;
    // Update basic metrics (approximate): treat LoadedFromCache as "load", BuiltFromSpans as "build"
    match load_kind {
        avocado_core::db::IndexLoadKind::LoadedFromCache => {
            // Reuse total_compile_ms to avoid growing struct too much; append to it for now
            state
                .total_compile_ms
                .fetch_add(elapsed_ms, std::sync::atomic::Ordering::Relaxed);
        }
        avocado_core::db::IndexLoadKind::BuiltFromSpans => {
            state
                .total_compile_ms
                .fetch_add(elapsed_ms, std::sync::atomic::Ordering::Relaxed);
        }
        avocado_core::db::IndexLoadKind::CachedInMemory => {}
    }

    // Create session manager
    let session_manager = Arc::new(SessionManager::new(database.clone()));

    let project_index = ProjectIndex {
        database,
        hnsw_index,
        session_manager,
        last_accessed: Arc::new(RwLock::new(Instant::now())),
        backend: tokio::sync::OnceCell::new(),
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

/// Securely normalize project path and enforce optional root restriction.
///
/// If AVOCADODB_ROOT is set, all projects must be within this root. Requests
/// with paths outside the root will be rejected.
fn secure_normalize_project_path(project: &str) -> anyhow::Result<PathBuf> {
    let normalized = normalize_project_path(project);
    if let Ok(root_str) = std::env::var("AVOCADODB_ROOT") {
        let root = PathBuf::from(root_str)
            .canonicalize()
            .map_err(|e| anyhow::anyhow!("Invalid AVOCADODB_ROOT: {}", e))?;
        if !normalized.starts_with(&root) {
            return Err(anyhow::anyhow!(
                "Project path {:?} is outside of configured root {:?}",
                normalized,
                root
            ));
        }
    }
    Ok(normalized)
}

/// Simple auth middleware using API_TOKEN and X-Avocado-Token header.
async fn auth_middleware(
    State(state): State<Arc<AppState>>,
    req: axum::http::Request<axum::body::Body>,
    next: middleware::Next,
) -> Result<axum::http::Response<axum::body::Body>, (StatusCode, Json<ErrorResponse>)> {
    // If no token configured, allow all
    if state.api_token.is_none() {
        return Ok(next.run(req).await);
    }
    // Allow health and docs without auth
    let path = req.uri().path();
    if path == "/health" || path.starts_with("/api-docs") {
        return Ok(next.run(req).await);
    }
    let token = state.api_token.as_ref().unwrap();
    let header = req
        .headers()
        .get("x-avocado-token")
        .and_then(|h| h.to_str().ok());
    if header != Some(token.as_str()) {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse::with_code("Unauthorized", "UNAUTHORIZED")),
        ));
    }
    Ok(next.run(req).await)
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
    let project_path = secure_normalize_project_path(project.as_deref().unwrap_or("."))
        .unwrap_or_else(|_| normalize_project_path(project.as_deref().unwrap_or(".")));
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

// ===== Session Handlers =====

async fn create_session_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateSessionRequest>,
) -> Result<Json<CreateSessionResponse>, (StatusCode, Json<ErrorResponse>)> {
    let project_index = get_or_load_project(&state, &req.project)
        .await
        .map_err(|e| internal_error(e.to_string()))?;

    let session = project_index
        .session_manager
        .start_session(req.user_id.as_deref())
        .map_err(|e| internal_error(e.to_string()))?;

    // Update title if provided
    if let Some(title) = req.title {
        project_index
            .database
            .update_session(&session.id, Some(&title), None)
            .map_err(|e| internal_error(e.to_string()))?;

        // Fetch updated session
        let updated_session = project_index
            .database
            .get_session(&session.id)
            .map_err(|e| internal_error(e.to_string()))?
            .ok_or_else(|| internal_error("Session not found after creation".to_string()))?;

        return Ok(Json(CreateSessionResponse {
            session: updated_session,
        }));
    }

    Ok(Json(CreateSessionResponse { session }))
}

async fn list_sessions_handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<ListSessionsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let project = params.get("project").cloned();
    let user_id = params.get("user_id").map(|s| s.as_str());
    let limit = params.get("limit").and_then(|s| s.parse::<usize>().ok());

    let project_index = get_or_load_project(&state, &project)
        .await
        .map_err(|e| internal_error(e.to_string()))?;

    let sessions = project_index
        .database
        .list_sessions(user_id, limit)
        .map_err(|e| internal_error(e.to_string()))?;

    Ok(Json(ListSessionsResponse { sessions }))
}

async fn get_session_handler(
    State(state): State<Arc<AppState>>,
    Path(session_id): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<GetSessionResponse>, (StatusCode, Json<ErrorResponse>)> {
    let project = params.get("project").cloned();
    let project_index = get_or_load_project(&state, &project)
        .await
        .map_err(|e| internal_error(e.to_string()))?;

    let session = project_index
        .database
        .get_session(&session_id)
        .map_err(|e| internal_error(e.to_string()))?
        .ok_or_else(|| not_found_error(format!("Session not found: {}", session_id)))?;

    let messages = project_index
        .database
        .get_messages(&session_id, None)
        .map_err(|e| internal_error(e.to_string()))?;

    Ok(Json(GetSessionResponse { session, messages }))
}

async fn delete_session_handler(
    State(state): State<Arc<AppState>>,
    Path(session_id): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<DeleteSessionResponse>, (StatusCode, Json<ErrorResponse>)> {
    let project = params.get("project").cloned();
    let project_index = get_or_load_project(&state, &project)
        .await
        .map_err(|e| internal_error(e.to_string()))?;

    project_index
        .database
        .delete_session(&session_id)
        .map_err(|e| internal_error(e.to_string()))?;

    Ok(Json(DeleteSessionResponse { success: true }))
}

async fn add_message_handler(
    State(state): State<Arc<AppState>>,
    Path(session_id): Path<String>,
    Json(req): Json<AddMessageRequest>,
) -> Result<Json<AddMessageResponse>, (StatusCode, Json<ErrorResponse>)> {
    let project_index = get_or_load_project(&state, &req.project)
        .await
        .map_err(|e| internal_error(e.to_string()))?;

    // Parse role
    let role = match req.role.to_lowercase().as_str() {
        "user" => MessageRole::User,
        "assistant" => MessageRole::Assistant,
        "system" => MessageRole::System,
        "tool" => MessageRole::Tool,
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::with_code(
                    format!(
                        "Invalid role: {}. Must be one of: user, assistant, system, tool",
                        req.role
                    ),
                    "INVALID_ROLE",
                )),
            ))
        }
    };

    let message = project_index
        .database
        .add_message(&session_id, role, &req.content, req.metadata.as_ref())
        .map_err(|e| internal_error(e.to_string()))?;

    Ok(Json(AddMessageResponse { message }))
}

async fn session_compile_handler(
    State(state): State<Arc<AppState>>,
    Path(session_id): Path<String>,
    Json(req): Json<SessionCompileRequest>,
) -> Result<Json<SessionCompileResponse>, (StatusCode, Json<ErrorResponse>)> {
    let project_index = get_or_load_project(&state, &req.project)
        .await
        .map_err(|e| internal_error(e.to_string()))?;

    // Use provided config or default
    let config = req.config.unwrap_or_else(|| CompilerConfig {
        token_budget: req.token_budget,
        ..Default::default()
    });

    // Ensure session exists, return 404 if not found
    let maybe_session = project_index
        .database
        .get_session(&session_id)
        .map_err(|e| internal_error(e.to_string()))?;
    if maybe_session.is_none() {
        return Err(not_found_error(format!(
            "Session not found: {}",
            session_id
        )));
    }

    // Add user message and compile
    let (message, working_set) = project_index
        .session_manager
        .add_user_message(
            &session_id,
            &req.query,
            config,
            project_index.hnsw_index.as_ref(),
            None,
        )
        .await
        .map_err(|e| {
            let msg = e.to_string();
            if msg.contains("FOREIGN KEY constraint failed") {
                return not_found_error(format!("Session not found: {}", session_id));
            }
            match e {
                avocado_core::Error::NotFound(m) => not_found_error(m),
                other => internal_error(other.to_string()),
            }
        })?;

    let ws_out = enrich_working_set(&working_set, &project_index.database);
    Ok(Json(SessionCompileResponse {
        message,
        working_set: ws_out,
    }))
}

async fn session_history_handler(
    State(state): State<Arc<AppState>>,
    Path(session_id): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<ConversationHistoryResponse>, (StatusCode, Json<ErrorResponse>)> {
    let project = params.get("project").cloned();
    let max_tokens = params
        .get("max_tokens")
        .and_then(|s| s.parse::<usize>().ok());

    let project_index = get_or_load_project(&state, &project)
        .await
        .map_err(|e| internal_error(e.to_string()))?;

    let history = project_index
        .session_manager
        .get_conversation_history(&session_id, max_tokens)
        .map_err(|e| internal_error(e.to_string()))?;

    Ok(Json(ConversationHistoryResponse { history }))
}

async fn session_replay_handler(
    State(state): State<Arc<AppState>>,
    Path(session_id): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<avocado_core::session::SessionReplay>, (StatusCode, Json<ErrorResponse>)> {
    let project = params.get("project").cloned();
    let project_index = get_or_load_project(&state, &project)
        .await
        .map_err(|e| internal_error(e.to_string()))?;

    let replay = project_index
        .session_manager
        .replay_session(&session_id)
        .map_err(|e| internal_error(e.to_string()))?;

    Ok(Json(replay))
}

// ===== Multi-Agent Handlers =====

async fn register_agent_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RegisterAgentRequest>,
) -> Result<Json<RegisterAgentResponse>, (StatusCode, Json<ErrorResponse>)> {
    let project_index = get_or_load_project(&state, &req.project)
        .await
        .map_err(|e| internal_error(e.to_string()))?;

    let agent = avocado_core::Agent {
        id: uuid::Uuid::new_v4().to_string(),
        name: req.name,
        role: req.role,
        model: req.model,
        system_prompt: req.system_prompt,
        did: req.did,
        capabilities: req.capabilities,
        metadata: None,
        created_at: chrono::Utc::now(),
    };

    let registered = project_index
        .database
        .register_agent(&agent)
        .map_err(|e| internal_error(e.to_string()))?;

    Ok(Json(RegisterAgentResponse { agent: registered }))
}

async fn list_agents_handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<ListAgentsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let project = params.get("project").cloned();
    let project_index = get_or_load_project(&state, &project)
        .await
        .map_err(|e| internal_error(e.to_string()))?;

    let agents = project_index
        .database
        .list_agents()
        .map_err(|e| internal_error(e.to_string()))?;

    Ok(Json(ListAgentsResponse { agents }))
}

async fn get_agent_handler(
    State(state): State<Arc<AppState>>,
    Path(agent_id): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<RegisterAgentResponse>, (StatusCode, Json<ErrorResponse>)> {
    let project = params.get("project").cloned();
    let project_index = get_or_load_project(&state, &project)
        .await
        .map_err(|e| internal_error(e.to_string()))?;

    let agent = project_index
        .database
        .get_agent(&agent_id)
        .map_err(|e| internal_error(e.to_string()))?
        .ok_or_else(|| not_found_error(format!("Agent not found: {}", agent_id)))?;

    Ok(Json(RegisterAgentResponse { agent }))
}

async fn add_agent_relation_handler(
    State(state): State<Arc<AppState>>,
    Path(session_id): Path<String>,
    Json(req): Json<AddAgentRelationRequest>,
) -> Result<Json<AddAgentRelationResponse>, (StatusCode, Json<ErrorResponse>)> {
    let project_index = get_or_load_project(&state, &req.project)
        .await
        .map_err(|e| internal_error(e.to_string()))?;

    // Parse stance
    let stance = avocado_core::Stance::from_str(&req.stance).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::with_code(e, "INVALID_STANCE")),
        )
    })?;

    let relation = project_index
        .database
        .add_agent_relation(
            &session_id,
            &req.message_id,
            &req.from_agent_id,
            &req.target_message_id,
            stance,
        )
        .map_err(|e| internal_error(e.to_string()))?;

    Ok(Json(AddAgentRelationResponse { relation }))
}

async fn get_agent_relations_handler(
    State(state): State<Arc<AppState>>,
    Path(session_id): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<GetAgentRelationsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let project = params.get("project").cloned();
    let project_index = get_or_load_project(&state, &project)
        .await
        .map_err(|e| internal_error(e.to_string()))?;

    let relations = project_index
        .database
        .get_agent_relations(&session_id)
        .map_err(|e| internal_error(e.to_string()))?;

    let agents = project_index
        .database
        .get_session_agents(&session_id)
        .map_err(|e| internal_error(e.to_string()))?;

    Ok(Json(GetAgentRelationsResponse { relations, agents }))
}

fn internal_error(msg: String) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse::with_code(msg, "INTERNAL_ERROR")),
    )
}

fn not_found_error(msg: String) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse::with_code(msg, "NOT_FOUND")),
    )
}
