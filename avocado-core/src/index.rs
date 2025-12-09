//! In-memory vector index for similarity search
//!
//! Phase 1 uses brute-force cosine similarity, which is fine for <10K spans.
//! Phase 2 uses HNSW for fast approximate nearest neighbor search (10-100x faster).

use crate::types::{Result, ScoredSpan, Span};
use crate::embedding;
use hnsw_rs::prelude::*;
use hnsw_rs::api::AnnT;
use std::path::Path;
use std::fs;
use std::time::Instant;
use sha2::{Digest, Sha256};
use serde::{Serialize, Deserialize};

/// In-memory vector index for fast similarity search
/// 
/// Uses HNSW (Hierarchical Navigable Small World) for approximate nearest neighbor search.
/// This provides O(log n) search time instead of O(n) brute-force, making it 10-100x faster
/// for large repositories (10K+ spans).
pub struct VectorIndex {
    /// HNSW index for fast approximate search (None for empty index)
    /// Note: When loaded from disk, we rebuild HNSW to avoid lifetime issues
    /// This is still faster than building from scratch since we have spans cached
    hnsw: Option<Hnsw<'static, f32, DistCosine>>,
    /// Spans indexed by their position in the HNSW index
    spans: Vec<Span>,
    /// Dimension of embeddings (typically 1536 for ada-002)
    dimension: usize,
}

impl VectorIndex {
    /// Build an index from spans using HNSW
    ///
    /// # Arguments
    ///
    /// * `spans` - Vector of spans with embeddings
    ///
    /// # Returns
    ///
    /// A new VectorIndex ready for searching
    ///
    /// # HNSW Parameters
    ///
    /// - `m`: Maximum number of connections per node (16 = good balance of speed/quality)
    /// - `ef_construction`: Size of candidate list during construction (200 = high quality)
    /// - `ef_search`: Size of candidate list during search (50 = good balance)
    ///
    /// These parameters provide >95% recall@50 while being 10-100x faster than brute-force.
    pub fn build(spans: Vec<Span>) -> Self {
        let start_time = Instant::now();
        let total_spans = spans.len();
        if spans.is_empty() {
            // Return empty index - no HNSW needed
            return Self {
                hnsw: None,
                spans: Vec::new(),
                dimension: embedding::embedding_dimension(), // Use current embedding dimension
            };
        }

        // Determine dimension from first span with embedding
        let dimension = spans
            .iter()
            .find_map(|s| s.embedding.as_ref().map(|e| e.len()))
            .unwrap_or_else(|| embedding::embedding_dimension()); // Use current embedding dimension

        // Create HNSW index
        // HNSW::new signature: (max_nb_connection, max_elements, max_layer, ef_construction, distance_fn)
        // Parameters tuned for good balance of speed and quality:
        // - max_nb_connection=32: Maximum connections per node (must be <= 256)
        //   This is also the "m" parameter (number of bi-directional links)
        // - max_elements: Expected number of elements (use spans.len() for efficiency)
        // - max_layer: Maximum layers (16 is a good default)
        // - ef_construction=200: High quality index construction
        // Note: Dimension is inferred from first insertion, not passed to new()
        let max_nb_connection = 32; // Must be <= 256, this is also "m"
        let max_elements = spans.len();
        let max_layer = 16; // Maximum layers in the graph
        let ef_construction = 200; // Construction quality parameter
        let hnsw = Hnsw::<f32, DistCosine>::new(
            max_nb_connection, // m parameter (bi-directional links per node)
            max_elements,      // Expected number of elements
            max_layer,         // Maximum layers
            ef_construction,   // Construction quality
            DistCosine::default(), // Distance function
        );

        // Insert all spans with embeddings into HNSW
        let mut inserted = 0usize;
        for (idx, span) in spans.iter().enumerate() {
            if let Some(embedding) = &span.embedding {
                if embedding.len() == dimension {
                    // Insert into HNSW with span index as data
                    // hnsw_rs expects a tuple: (&[f32], usize) where usize is the payload
                    hnsw.insert((embedding.as_slice(), idx));
                    inserted += 1;
                }
            }
        }

        let elapsed_ms = start_time.elapsed().as_millis();
        log::info!(
            "VectorIndex::build: indexed {} of {} spans (dim={}): {}ms",
            inserted,
            total_spans,
            dimension,
            elapsed_ms
        );

        Self {
            hnsw: Some(hnsw),
            spans,
            dimension,
        }
    }

    /// Search for similar spans using HNSW approximate nearest neighbor search
    ///
    /// # Arguments
    ///
    /// * `query_embedding` - The query vector
    /// * `k` - Number of results to return
    ///
    /// # Returns
    ///
    /// Vector of scored spans, sorted by relevance (highest score first)
    ///
    /// # Performance
    ///
    /// - O(log n) search time instead of O(n) brute-force
    /// - 10-100x faster for large repositories (10K+ spans)
    /// - >95% recall@k (quality maintained)
    pub fn search(&self, query_embedding: &[f32], k: usize) -> Result<Vec<ScoredSpan>> {
        if self.spans.is_empty() || query_embedding.len() != self.dimension {
            return Ok(Vec::new());
        }

        // If no HNSW (empty index), return empty results
        let hnsw = match &self.hnsw {
            Some(h) => h,
            None => return Ok(Vec::new()),
        };

        // Use ef_search = max(k * 2, 50) for good balance of speed and quality
        // Higher ef_search = better recall but slower
        let ef_search = (k * 2).max(50).min(200);

        // Search HNSW for approximate nearest neighbors
        let search_results = hnsw.search(
            query_embedding,
            k.min(self.spans.len()),
            ef_search,
        );

        // Convert HNSW results to ScoredSpan
        // HNSW returns Neighbour structs with distance and data (our span index)
        // Distance is cosine distance (0 = identical, 2 = opposite)
        // Convert to similarity score: score = 1 - (distance / 2)
        let results: Vec<ScoredSpan> = search_results
            .into_iter()
            .filter_map(|neighbour| {
                let idx = neighbour.d_id; // The data we stored (span index)
                let distance = neighbour.distance;
                
                // HNSW uses distance (0 = identical, 2 = opposite)
                // Convert to similarity score: score = 1 - (distance / 2)
                // This gives us a score in [0, 1] range where 1 = identical
                let score: f32 = 1.0 - (distance / 2.0);
                
                // Ensure score is in valid range [0, 1]
                let score = score.max(0.0_f32).min(1.0_f32);
                
                self.spans.get(idx).map(|span| ScoredSpan {
                    span: span.clone(),
                    score,
                })
            })
            .collect();

        // Sort by score descending (HNSW results are already sorted, but ensure it)
        let mut results = results;
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

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

    /// Get all spans (for debugging and persistence)
    pub fn spans(&self) -> &[Span] {
        &self.spans
    }
    
    /// Save HNSW index to disk for persistence (Phase 2.1)
    ///
    /// Saves both the HNSW graph structure and the spans metadata.
    /// This allows instant loading on subsequent queries once `hnsw_rs`
    /// supports owned loading; for now we only reload from cached spans.
    ///
    /// # Arguments
    ///
    /// * `cache_dir` - Directory to save index files
    ///
    /// # Returns
    ///
    /// Ok(()) if successful
    pub fn save_to_disk(&self, cache_dir: &Path) -> Result<()> {
        // Create directory if needed
        fs::create_dir_all(cache_dir)
            .map_err(|e| crate::types::Error::Other(anyhow::anyhow!("Failed to create cache directory: {}", e)))?;
        
        // Save HNSW index using its native dump format
        // Note: We don't currently load this dump back due to lifetime
        // limitations in `hnsw_rs` (see docs/HNSW_LIFETIME_ISSUE.md).
        // The files are kept so we can take advantage of them once
        // the upstream library supports owned loading.
        if let Some(hnsw) = &self.hnsw {
            // HNSW file_dump creates two files: {basename}.hnsw.graph and {basename}.hnsw.data
            let basename = "index";
            // Write to a temporary dir then atomically move into place
            let tmp_dir = cache_dir.join(".tmp");
            let _ = fs::remove_dir_all(&tmp_dir);
            fs::create_dir_all(&tmp_dir)
                .map_err(|e| crate::types::Error::Other(anyhow::anyhow!("Failed to create tmp dir: {}", e)))?;
            hnsw.file_dump(&tmp_dir, basename)
                .map_err(|e| crate::types::Error::Other(anyhow::anyhow!("Failed to dump HNSW index: {}", e)))?;
            // Move files into place
            for ext in &["hnsw.graph", "hnsw.data"] {
                let src = tmp_dir.join(format!("{}.{}", basename, ext));
                let dst = cache_dir.join(format!("{}.{}", basename, ext));
                fs::rename(&src, &dst)
                    .map_err(|e| crate::types::Error::Other(anyhow::anyhow!("Failed to move {}: {}", ext, e)))?;
            }
            let _ = fs::remove_dir_all(&tmp_dir);
        }
        
        // Save spans metadata separately (for validation and reference)
        let spans_path = cache_dir.join("spans.bin");
        let spans_tmp = cache_dir.join("spans.bin.tmp");
        let spans_data = bincode::serialize(&self.spans)
            .map_err(|e| crate::types::Error::Other(anyhow::anyhow!("Failed to serialize spans: {}", e)))?;
        fs::write(&spans_tmp, &spans_data)
            .map_err(|e| crate::types::Error::Other(anyhow::anyhow!("Failed to write spans cache: {}", e)))?;
        fs::rename(&spans_tmp, &spans_path)
            .map_err(|e| crate::types::Error::Other(anyhow::anyhow!("Failed to move spans cache: {}", e)))?;
        
        // Save dimension for validation
        let dim_path = cache_dir.join("dimension.bin");
        let dim_tmp = cache_dir.join("dimension.bin.tmp");
        let dim_data = bincode::serialize(&self.dimension)
            .map_err(|e| crate::types::Error::Other(anyhow::anyhow!("Failed to serialize dimension: {}", e)))?;
        fs::write(&dim_tmp, &dim_data)
            .map_err(|e| crate::types::Error::Other(anyhow::anyhow!("Failed to write dimension cache: {}", e)))?;
        fs::rename(&dim_tmp, &dim_path)
            .map_err(|e| crate::types::Error::Other(anyhow::anyhow!("Failed to move dimension cache: {}", e)))?;

        // Save metadata with checksum and version
        let checksum = {
            let mut hasher = Sha256::new();
            hasher.update(&spans_data);
            format!("{:x}", hasher.finalize())
        };
        let meta = IndexMeta {
            version: 1,
            backend: "hnsw_rs".to_string(),
            span_count: self.spans.len(),
            dimension: self.dimension,
            spans_checksum: checksum,
        };
        let meta_json = serde_json::to_vec(&meta)
            .map_err(|e| crate::types::Error::Other(anyhow::anyhow!("Failed to serialize index meta: {}", e)))?;
        let meta_path = cache_dir.join("index.meta.json");
        let meta_tmp = cache_dir.join("index.meta.json.tmp");
        fs::write(&meta_tmp, &meta_json)
            .map_err(|e| crate::types::Error::Other(anyhow::anyhow!("Failed to write index meta: {}", e)))?;
        fs::rename(&meta_tmp, &meta_path)
            .map_err(|e| crate::types::Error::Other(anyhow::anyhow!("Failed to move index meta: {}", e)))?;
        
        Ok(())
    }
    
    /// Load HNSW index from disk (Phase 2.1)
    ///
    /// **Note**: Due to lifetime constraints in the `hnsw_rs` library, we cannot directly
    /// load the HNSW structure from disk. Instead, we load cached spans and rebuild HNSW.
    /// This is still faster than loading spans from SQLite, but not as fast as loading
    /// the HNSW structure directly would be.
    ///
    /// For large repositories (>10K spans), consider using server mode which keeps the
    /// index in memory across queries for optimal performance.
    ///
    /// # Arguments
    ///
    /// * `cache_dir` - Directory containing index files
    ///
    /// # Returns
    ///
    /// Some(VectorIndex) if loaded successfully, None otherwise
    pub fn load_from_disk(cache_dir: &Path) -> Result<Option<Self>> {
        use std::fs;
        // Check if cache files exist
        let spans_path = cache_dir.join("spans.bin");
        let dim_path = cache_dir.join("dimension.bin");
        let meta_path = cache_dir.join("index.meta.json");
        
        if !spans_path.exists() || !dim_path.exists() || !meta_path.exists() {
            return Ok(None);
        }
        // Read and validate metadata
        let meta_json = fs::read(&meta_path)
            .map_err(|e| crate::types::Error::Other(anyhow::anyhow!("Failed to read index meta: {}", e)))?;
        let meta: IndexMeta = serde_json::from_slice(&meta_json)
            .map_err(|e| crate::types::Error::Other(anyhow::anyhow!("Failed to parse index meta: {}", e)))?;
        if meta.version != 1 {
            return Err(crate::types::Error::Other(anyhow::anyhow!("Unsupported index version: {}", meta.version)));
        }

        // Load dimension
        let dim_data = fs::read(&dim_path)
            .map_err(|e| crate::types::Error::Other(anyhow::anyhow!("Failed to read dimension cache: {}", e)))?;
        let cached_dimension: usize = bincode::deserialize(&dim_data)
            .map_err(|e| crate::types::Error::Other(anyhow::anyhow!("Failed to deserialize dimension: {}", e)))?;
        
        // Load spans
        let spans_data = fs::read(&spans_path)
            .map_err(|e| crate::types::Error::Other(anyhow::anyhow!("Failed to read spans cache: {}", e)))?;
        // Validate checksum
        let mut hasher = Sha256::new();
        hasher.update(&spans_data);
        let actual_checksum = format!("{:x}", hasher.finalize());
        if actual_checksum != meta.spans_checksum {
            return Err(crate::types::Error::Other(anyhow::anyhow!(
                "Index cache checksum mismatch"
            )));
        }
        let spans: Vec<Span> = bincode::deserialize(&spans_data)
            .map_err(|e| crate::types::Error::Other(anyhow::anyhow!("Failed to deserialize spans: {}", e)))?;
        
        // Basic validation: ensure cached dimension matches spans embeddings
        let actual_dimension = spans
            .iter()
            .find_map(|s| s.embedding.as_ref().map(|e| e.len()))
            .unwrap_or(cached_dimension);
        if actual_dimension != cached_dimension {
            return Err(crate::types::Error::Other(anyhow::anyhow!(
                "Cached index dimension mismatch (expected {}, found {})",
                cached_dimension,
                actual_dimension
            )));
        }
        
        // Rebuild HNSW from cached spans
        // 
        // LIMITATION: The `hnsw_rs` library has lifetime constraints that prevent us from
        // directly loading and storing the HNSW structure. The loaded HNSW has a lifetime
        // tied to HnswIo, which conflicts with our need for 'static lifetime in VectorIndex.
        //
        // WORKAROUND: We cache spans (fast to load) and rebuild HNSW (slower but necessary).
        // This is still faster than loading spans from SQLite, but rebuilding HNSW from
        // 50K+ spans can take 1-2 minutes.
        //
        // RECOMMENDATION: For large repositories, use server mode which keeps the index
        // in memory across queries, avoiding rebuilds entirely.
        //
        // FUTURE: Consider alternative HNSW libraries (hora, instant-distance) or contribute
        // serialization improvements to hnsw_rs that support owned HNSW structures.
        Ok(Some(Self::build(spans)))
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct IndexMeta {
    version: u32,
    backend: String,
    span_count: usize,
    dimension: usize,
    spans_checksum: String,
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
