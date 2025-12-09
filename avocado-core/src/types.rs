//! Core data types for AvocadoDB
//!
//! This module defines the fundamental data structures used throughout AvocadoDB:
//! - Artifact: A complete ingested document
//! - Span: A fragment of a document with embeddings and citations
//! - WorkingSet: The compiled context result
//! - CompilerConfig: Configuration for context compilation

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
