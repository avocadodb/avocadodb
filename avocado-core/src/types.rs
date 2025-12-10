//! Core data types for AvocadoDB
//!
//! This module defines the fundamental data structures used throughout AvocadoDB:
//! - Artifact: A complete ingested document
//! - Span: A fragment of a document with embeddings and citations
//! - WorkingSet: The compiled context result
//! - CompilerConfig: Configuration for context compilation
//! - Manifest: Version and parameter manifest for reproducibility
//! - ExplainPlan: Detailed retrieval explanation for debugging

use serde::{Deserialize, Serialize};

/// An ingested document stored in the database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    /// Unique identifier (UUID v4)
    pub id: String,
    /// File path or unique identifier
    pub path: String,
    /// Full document content
    pub content: String,
    /// SHA256 hash of content for deduplication
    pub content_hash: String,
    /// Optional metadata (arbitrary JSON)
    pub metadata: Option<serde_json::Value>,
    /// Timestamp when artifact was created
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// A span of text from a document with embeddings and line numbers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Span {
    /// Unique identifier (UUID v4)
    pub id: String,
    /// Parent artifact ID
    pub artifact_id: String,
    /// Starting line number (1-indexed)
    pub start_line: usize,
    /// Ending line number (inclusive)
    pub end_line: usize,
    /// The actual text content of this span
    pub text: String,
    /// Vector embedding (1536 dimensions for ada-002)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding: Option<Vec<f32>>,
    /// Model used to generate embedding
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding_model: Option<String>,
    /// Estimated token count
    pub token_count: usize,
    /// Optional metadata (arbitrary JSON)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// A span with its relevance score (used during retrieval)
#[derive(Debug, Clone)]
pub struct ScoredSpan {
    /// The span
    pub span: Span,
    /// Relevance score (higher is better)
    pub score: f32,
}

/// Citation information for a span in a working set
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Citation {
    /// Span ID
    pub span_id: String,
    /// Artifact ID
    pub artifact_id: String,
    /// Artifact path for display
    pub artifact_path: String,
    /// Starting line number
    pub start_line: usize,
    /// Ending line number
    pub end_line: usize,
    /// Relevance score
    pub score: f32,
}

/// The compiled context result - deterministic every time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkingSet {
    /// The compiled text ready for LLM consumption
    pub text: String,
    /// Spans included in this working set (sorted deterministically)
    pub spans: Vec<Span>,
    /// Citation information for each span
    pub citations: Vec<Citation>,
    /// Total tokens used
    pub tokens_used: usize,
    /// Original query
    pub query: String,
    /// Time taken to compile (milliseconds)
    pub compilation_time_ms: u64,
    /// Version manifest for reproducibility (populated when requested)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manifest: Option<Manifest>,
    /// Explain plan for debugging (populated when explain=true)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub explain: Option<ExplainPlan>,
}

impl WorkingSet {
    /// Calculate a deterministic hash of this working set
    /// Used for testing determinism - same query should always produce same hash
    pub fn deterministic_hash(&self) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(self.text.as_bytes());
        format!("{:x}", hasher.finalize())
    }
}

/// Configuration for the context compiler
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompilerConfig {
    /// Maximum number of tokens to include in working set
    pub token_budget: usize,
    /// Whether to apply MMR (Maximal Marginal Relevance) for diversity
    pub enable_mmr: bool,
    /// MMR lambda parameter (0.0 = max diversity, 1.0 = max relevance)
    pub mmr_lambda: f32,
    /// Weight for semantic search results (0.0 to 1.0)
    pub semantic_weight: f32,
    /// Weight for lexical/keyword search results (0.0 to 1.0)
    pub lexical_weight: f32,
}

impl Default for CompilerConfig {
    fn default() -> Self {
        Self {
            token_budget: 8000,
            enable_mmr: true,
            mmr_lambda: 0.5,
            semantic_weight: 0.7,
            lexical_weight: 0.3,
        }
    }
}

/// A conversation session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Unique identifier (UUID v4)
    pub id: String,
    /// Optional user identifier
    pub user_id: Option<String>,
    /// Optional session title
    pub title: Option<String>,
    /// Optional metadata (arbitrary JSON)
    pub metadata: Option<serde_json::Value>,
    /// When session was created
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// When session was last updated
    pub updated_at: chrono::DateTime<chrono::Utc>,
    /// When last message was added
    pub last_message_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// A message in a conversation session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Unique identifier (UUID v4)
    pub id: String,
    /// Session this message belongs to
    pub session_id: String,
    /// Message role: 'user', 'assistant', 'system', 'tool'
    pub role: MessageRole,
    /// Message content
    pub content: String,
    /// Optional metadata (tool calls, citations, etc.)
    pub metadata: Option<serde_json::Value>,
    /// Sequence number within session (0-indexed)
    pub sequence_number: usize,
    /// When message was created
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Role of a message in a conversation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    /// Message from the user
    User,
    /// Message from the AI assistant
    Assistant,
    /// System message (instructions, context)
    System,
    /// Message from a tool or function
    Tool,
}

impl MessageRole {
    /// Convert MessageRole to string for database storage
    pub fn as_str(&self) -> &str {
        match self {
            MessageRole::User => "user",
            MessageRole::Assistant => "assistant",
            MessageRole::System => "system",
            MessageRole::Tool => "tool",
        }
    }

    /// Parse MessageRole from string
    pub fn from_str(s: &str) -> std::result::Result<Self, String> {
        match s {
            "user" => Ok(MessageRole::User),
            "assistant" => Ok(MessageRole::Assistant),
            "system" => Ok(MessageRole::System),
            "tool" => Ok(MessageRole::Tool),
            _ => Err(format!("Invalid message role: {}", s)),
        }
    }
}

/// Association between a session and a working set
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionWorkingSet {
    /// Unique identifier (UUID v4)
    pub id: String,
    /// Session ID
    pub session_id: String,
    /// Optional message ID that triggered this compilation
    pub message_id: Option<String>,
    /// The working set (stored as JSON)
    pub working_set: WorkingSet,
    /// Query that generated this working set
    pub query: String,
    /// Configuration used for compilation
    pub config: CompilerConfig,
    /// When this was created
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Session with its messages (for retrieval)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionWithMessages {
    /// The session
    pub session: Session,
    /// Messages in chronological order
    pub messages: Vec<Message>,
    /// Associated working sets
    pub working_sets: Vec<SessionWorkingSet>,
}

// ============================================================================
// Version Manifest - for full reproducibility
// ============================================================================

/// Version and parameter manifest for reproducibility
///
/// This captures all parameters that affect compilation output,
/// allowing exact reproduction of results and debugging.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    /// AvocadoDB version
    pub avocado_version: String,
    /// Tokenizer used (e.g., "cl100k_base")
    pub tokenizer: String,
    /// Embedding model name
    pub embedding_model: String,
    /// Embedding dimension
    pub embedding_dimension: usize,
    /// Chunking parameters used
    pub chunking: ChunkingParams,
    /// Index parameters used
    pub index: IndexParams,
    /// SHA256 hash of final compiled context
    pub context_hash: String,
}

/// Parameters controlling document chunking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkingParams {
    /// Minimum lines per span
    pub min_lines: usize,
    /// Maximum lines per span
    pub max_lines: usize,
    /// Target lines per span
    pub target_lines: usize,
}

impl Default for ChunkingParams {
    fn default() -> Self {
        Self {
            min_lines: 20,
            max_lines: 50,
            target_lines: 30,
        }
    }
}

/// Parameters controlling the HNSW index
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexParams {
    /// Max connections per node
    pub hnsw_m: usize,
    /// Construction quality parameter
    pub hnsw_ef_construction: usize,
    /// Search quality parameter
    pub hnsw_ef_search: usize,
}

impl Default for IndexParams {
    fn default() -> Self {
        Self {
            hnsw_m: 32,
            hnsw_ef_construction: 200,
            hnsw_ef_search: 50,
        }
    }
}

// ============================================================================
// Explain Plan - for debugging retrieval
// ============================================================================

/// Detailed explanation of the retrieval pipeline
///
/// When `explain=true`, this captures candidates at each stage
/// so you can debug why specific spans were or weren't included.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplainPlan {
    /// Original query
    pub query: String,
    /// Hash of query embedding (for verification)
    pub query_embedding_hash: String,
    /// Candidates from semantic (HNSW) search
    pub semantic_candidates: Vec<ExplainCandidate>,
    /// Candidates from lexical (keyword) search
    pub lexical_candidates: Vec<ExplainCandidate>,
    /// Candidates after hybrid fusion (RRF)
    pub fused_candidates: Vec<ExplainCandidate>,
    /// Candidates after MMR diversification
    pub mmr_candidates: Vec<ExplainCandidate>,
    /// Spans after token budget packing
    pub packed_candidates: Vec<ExplainCandidate>,
    /// Final order after deterministic sort
    pub final_candidates: Vec<ExplainCandidate>,
    /// Per-stage timing information
    pub timing: ExplainTiming,
    /// Thresholds and parameters used
    pub thresholds: ExplainThresholds,
}

/// A candidate span at a specific pipeline stage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplainCandidate {
    /// Span ID
    pub span_id: String,
    /// Artifact path for display
    pub artifact_path: String,
    /// Line range (start, end)
    pub lines: (usize, usize),
    /// Score at this stage
    pub score: f32,
    /// Token count
    pub tokens: usize,
    /// Rank at this stage (1-indexed)
    pub rank: usize,
}

/// Timing breakdown for each pipeline stage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplainTiming {
    /// Time to embed query (ms)
    pub embed_query_ms: u64,
    /// Time for semantic search (ms)
    pub semantic_search_ms: u64,
    /// Time for lexical search (ms)
    pub lexical_search_ms: u64,
    /// Time for hybrid fusion (ms)
    pub fusion_ms: u64,
    /// Time for MMR diversification (ms)
    pub mmr_ms: u64,
    /// Time for token packing (ms)
    pub packing_ms: u64,
    /// Time to build context (ms)
    pub build_context_ms: u64,
    /// Total compilation time (ms)
    pub total_ms: u64,
}

impl Default for ExplainTiming {
    fn default() -> Self {
        Self {
            embed_query_ms: 0,
            semantic_search_ms: 0,
            lexical_search_ms: 0,
            fusion_ms: 0,
            mmr_ms: 0,
            packing_ms: 0,
            build_context_ms: 0,
            total_ms: 0,
        }
    }
}

/// Thresholds and parameters used during compilation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplainThresholds {
    /// Number of semantic candidates retrieved
    pub semantic_k: usize,
    /// Number of lexical candidates retrieved
    pub lexical_k: usize,
    /// Semantic weight in fusion
    pub semantic_weight: f32,
    /// Lexical weight in fusion
    pub lexical_weight: f32,
    /// MMR lambda (relevance vs diversity)
    pub mmr_lambda: f32,
    /// Whether MMR was enabled
    pub mmr_enabled: bool,
    /// Token budget
    pub token_budget: usize,
}

impl Default for ExplainThresholds {
    fn default() -> Self {
        Self {
            semantic_k: 50,
            lexical_k: 20,
            semantic_weight: 0.7,
            lexical_weight: 0.3,
            mmr_lambda: 0.5,
            mmr_enabled: true,
            token_budget: 8000,
        }
    }
}

// ============================================================================
// Incremental Rebuild - for efficient re-ingestion
// ============================================================================

/// Action to take when ingesting a document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IngestAction {
    /// Skip - content unchanged
    Skip {
        /// Existing artifact ID
        artifact_id: String,
        /// Reason for skipping
        reason: String,
    },
    /// Update - content changed, re-index
    Update {
        /// Existing artifact ID to update
        artifact_id: String,
    },
    /// Create - new document
    Create,
}

// ============================================================================
// Evaluation - for quality metrics
// ============================================================================

/// A golden query for evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoldenQuery {
    /// The query string
    pub query: String,
    /// Expected artifact paths in results
    pub expected_paths: Vec<String>,
    /// Check within top k results
    pub k: usize,
}

/// Result of evaluating a single query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalResult {
    /// The query evaluated
    pub query: String,
    /// Recall at k (% of expected in top k)
    pub recall_at_k: f32,
    /// Precision at k (% of top k that are expected)
    pub precision_at_k: f32,
    /// Mean Reciprocal Rank
    pub mrr: f32,
    /// Latency in milliseconds
    pub latency_ms: u64,
}

/// Summary of evaluation across multiple queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalSummary {
    /// Mean recall across queries
    pub mean_recall: f32,
    /// Mean precision across queries
    pub mean_precision: f32,
    /// Mean MRR across queries
    pub mean_mrr: f32,
    /// Median latency (p50)
    pub p50_latency_ms: u64,
    /// 99th percentile latency
    pub p99_latency_ms: u64,
    /// Number of queries evaluated
    pub query_count: usize,
    /// Individual results
    pub results: Vec<EvalResult>,
}

// ============================================================================
// Diff - for comparing working sets
// ============================================================================

/// Diff between two working sets
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkingSetDiff {
    /// Query used for both compilations
    pub query: String,
    /// Hash of the "before" working set
    pub before_hash: String,
    /// Hash of the "after" working set
    pub after_hash: String,
    /// Spans added in "after" (not in "before")
    pub added: Vec<DiffEntry>,
    /// Spans removed in "after" (were in "before")
    pub removed: Vec<DiffEntry>,
    /// Spans with changed rank/score
    pub reranked: Vec<RerankEntry>,
}

/// Entry for an added or removed span
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffEntry {
    /// Span ID
    pub span_id: String,
    /// Artifact path
    pub artifact_path: String,
    /// Line range (start, end)
    pub lines: (usize, usize),
    /// Score
    pub score: f32,
    /// Rank in results
    pub rank: usize,
}

/// Entry for a span that changed rank
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RerankEntry {
    /// Span ID
    pub span_id: String,
    /// Artifact path
    pub artifact_path: String,
    /// Old rank
    pub old_rank: usize,
    /// New rank
    pub new_rank: usize,
    /// Old score
    pub old_score: f32,
    /// New score
    pub new_score: f32,
}

// ============================================================================
// Errors and Result type
// ============================================================================

/// Result type alias for AvocadoDB operations
pub type Result<T> = std::result::Result<T, Error>;

/// Error types for AvocadoDB
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("Embedding error: {0}")]
    Embedding(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Other error: {0}")]
    Other(#[from] anyhow::Error),
}
