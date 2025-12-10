//! AvocadoDB Core Engine
//!
//! The deterministic context compiler that fixes RAG.
//!
//! AvocadoDB replaces traditional vector databases' chaotic "top-k" retrieval
//! with deterministic, citation-backed context generation using span-based
//! compilation.
//!
//! # Key Features
//!
//! - **100% Deterministic**: Same query → same context, every time
//! - **Citation-Backed**: Every piece of context has exact line number citations
//! - **Token Efficient**: 95%+ token budget utilization (vs 60-70% in RAG)
//! - **Fast**: < 500ms for 8K token context
//!
//! # Architecture
//!
//! ```text
//! Query → Embed → [Semantic Search + Lexical Search] → Hybrid Fusion
//!       → MMR Diversification → Token Packing → Deterministic Sort → WorkingSet
//! ```

#![warn(missing_docs)]

pub mod approx;
pub mod compiler;
pub mod db;
pub mod diff;
pub mod embedding;
pub mod eval;
pub mod index;
pub mod session;
pub mod span;
pub mod storage;
pub mod types;

// Re-export commonly used types
pub use types::{
    // Multi-agent orchestration types
    Agent,
    AgentMessageMeta,
    AgentRelation,
    AgentRelationEntry,
    AgentRelationSummary,
    Artifact,
    ChunkingParams,
    Citation,
    CompilerConfig,
    DiffEntry,
    Error,
    EvalResult,
    EvalSummary,
    ExplainCandidate,
    ExplainPlan,
    ExplainThresholds,
    ExplainTiming,
    GoldenQuery,
    IndexParams,
    IngestAction,
    // New types for enhanced determinism and explainability
    Manifest,
    Message,
    MessageRole,
    RerankEntry,
    Result,
    ScoredSpan,
    Session,
    SessionWorkingSet,
    Span,
    Stance,
    WorkingSet,
    WorkingSetDiff,
};

pub use session::{SessionManager, SessionManagerGeneric, SessionReplay, SessionTurn};

// Re-export compiler functions
pub use compiler::{compile, compile_with_backend, compile_with_backend_options};

// Re-export storage module types
pub use storage::{
    create_backend, create_backend_from_env, SqliteBackend, StorageBackend, StorageConfig,
    VectorSearchProvider, VectorSearchResult,
};

/// The version of AvocadoDB
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(clippy::const_is_empty)]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }

    #[test]
    fn test_working_set_hash_determinism() {
        let ws1 = WorkingSet {
            text: "Hello, world!".to_string(),
            spans: vec![],
            citations: vec![],
            tokens_used: 3,
            query: "test".to_string(),
            compilation_time_ms: 100,
            manifest: None,
            explain: None,
        };

        let ws2 = WorkingSet {
            text: "Hello, world!".to_string(),
            spans: vec![],
            citations: vec![],
            tokens_used: 3,
            query: "test".to_string(),
            compilation_time_ms: 200, // Different timing
            manifest: None,
            explain: None,
        };

        // Hash should be the same because it only depends on text
        assert_eq!(ws1.deterministic_hash(), ws2.deterministic_hash());
    }
}
