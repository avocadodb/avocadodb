//! Vector search abstraction
//!
//! Provides a unified interface for vector similarity search
//! across different backends (HNSW for SQLite, pgvector for PostgreSQL).

use async_trait::async_trait;
use crate::types::{Result, ScoredSpan, Span};

/// Result from vector similarity search
#[derive(Debug, Clone)]
pub struct VectorSearchResult {
    /// The matching span
    pub span: Span,
    /// Similarity score (higher is more similar)
    pub score: f32,
}

impl From<VectorSearchResult> for ScoredSpan {
    fn from(r: VectorSearchResult) -> Self {
        ScoredSpan {
            span: r.span,
            score: r.score,
        }
    }
}

impl From<ScoredSpan> for VectorSearchResult {
    fn from(s: ScoredSpan) -> Self {
        VectorSearchResult {
            span: s.span,
            score: s.score,
        }
    }
}

/// Vector search provider trait
///
/// Implementations provide vector similarity search capabilities.
/// This abstracts over HNSW (for SQLite) and pgvector (for PostgreSQL).
#[async_trait]
pub trait VectorSearchProvider: Send + Sync {
    /// Search for similar spans
    ///
    /// # Arguments
    /// * `query_embedding` - Query vector
    /// * `k` - Number of results to return
    ///
    /// # Returns
    /// Vector of scored spans, sorted by relevance (highest score first)
    async fn search(&self, query_embedding: &[f32], k: usize) -> Result<Vec<VectorSearchResult>>;

    /// Get number of indexed spans
    fn len(&self) -> usize;

    /// Check if index is empty
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get embedding dimension
    fn dimension(&self) -> usize;
}
