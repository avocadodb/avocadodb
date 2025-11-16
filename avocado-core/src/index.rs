//! In-memory vector index for similarity search
//!
//! Phase 1 uses brute-force cosine similarity, which is fine for <10K spans.
//! Phase 2 can add HNSW or other approximate methods if needed.

use crate::types::{Result, ScoredSpan, Span};

/// In-memory vector index for fast similarity search
pub struct VectorIndex {
    /// Spans with their embeddings
    spans: Vec<Span>,
}

impl VectorIndex {
    /// Build an index from spans
    ///
    /// # Arguments
    ///
    /// * `spans` - Vector of spans with embeddings
    ///
    /// # Returns
    ///
    /// A new VectorIndex ready for searching
    pub fn build(spans: Vec<Span>) -> Self {
        Self { spans }
    }

    /// Search for similar spans using cosine similarity
    ///
    /// # Arguments
    ///
    /// * `query_embedding` - The query vector
    /// * `k` - Number of results to return
    ///
    /// # Returns
    ///
    /// Vector of scored spans, sorted by relevance (highest score first)
    pub fn search(&self, query_embedding: &[f32], k: usize) -> Result<Vec<ScoredSpan>> {
        let mut scores: Vec<(usize, f32)> = self
            .spans
            .iter()
            .enumerate()
            .filter_map(|(idx, span)| {
                span.embedding.as_ref().map(|emb| {
                    let score = cosine_similarity(query_embedding, emb);
                    (idx, score)
                })
            })
            .collect();

        // Sort by score descending
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Take top k
        scores.truncate(k);

        // Convert to ScoredSpan
        let results = scores
            .into_iter()
            .map(|(idx, score)| ScoredSpan {
                span: self.spans[idx].clone(),
                score,
            })
            .collect();

        Ok(results)
    }

    /// Get the number of spans in the index
    pub fn len(&self) -> usize {
        self.spans.len()
    }

    /// Check if the index is empty
    pub fn is_empty(&self) -> bool {
        self.spans.is_empty()
    }

    /// Get all spans (for debugging)
    pub fn spans(&self) -> &[Span] {
        &self.spans
    }
}

/// Calculate cosine similarity between two vectors
///
/// # Arguments
///
/// * `a` - First vector
/// * `b` - Second vector
///
/// # Returns
///
/// Cosine similarity score (0.0 to 1.0, higher is more similar)
///
/// # Formula
///
/// ```text
/// cosine_sim(a, b) = (a · b) / (||a|| * ||b||)
/// ```
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }

    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }

    dot / (norm_a * norm_b)
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn create_test_span(embedding: Vec<f32>) -> Span {
        Span {
            id: Uuid::new_v4().to_string(),
            artifact_id: "test".to_string(),
            start_line: 1,
            end_line: 10,
            text: "test text".to_string(),
            embedding: Some(embedding),
            embedding_model: Some("test".to_string()),
            token_count: 10,
            metadata: None,
        }
    }

    #[test]
    fn test_cosine_similarity_identical() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        let sim = cosine_similarity(&a, &b);
        assert!((sim - 1.0).abs() < 0.0001);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        let sim = cosine_similarity(&a, &b);
        assert!(sim.abs() < 0.0001); // Should be ~0
    }

    #[test]
    fn test_cosine_similarity_opposite() {
        let a = vec![1.0, 0.0];
        let b = vec![-1.0, 0.0];
        let sim = cosine_similarity(&a, &b);
        assert!((sim + 1.0).abs() < 0.0001); // Should be ~-1
    }

    #[test]
    fn test_vector_index_search() {
        let spans = vec![
            create_test_span(vec![1.0, 0.0, 0.0]),
            create_test_span(vec![0.0, 1.0, 0.0]),
            create_test_span(vec![0.9, 0.1, 0.0]),
        ];

        let index = VectorIndex::build(spans);
        assert_eq!(index.len(), 3);

        let query = vec![1.0, 0.0, 0.0];
        let results = index.search(&query, 2).unwrap();

        assert_eq!(results.len(), 2);
        // First result should be most similar
        assert!(results[0].score > results[1].score);
    }

    #[test]
    fn test_empty_index() {
        let index = VectorIndex::build(vec![]);
        assert!(index.is_empty());

        let results = index.search(&[1.0, 0.0], 10).unwrap();
        assert_eq!(results.len(), 0);
    }
}
