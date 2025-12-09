use crate::{Result, ScoredSpan, Span};
use crate::index::cosine_similarity;
use serde::{Serialize, Deserialize};
use std::path::Path;

/// Minimal ANN abstraction for build/search/save/load
pub trait ApproxIndex: Sized {
    /// Build from fully-embedded spans (embedding must be present)
    fn build(spans: Vec<Span>) -> Self;

    /// Search top-k results for given query embedding
    fn search(&self, query_embedding: &[f32], k: usize) -> Result<Vec<ScoredSpan>>;

    /// Save owned index to disk
    fn save_to_disk(&self, dir: &Path) -> Result<()>;

    /// Load owned index from disk
    fn load_from_disk(dir: &Path) -> Result<Option<Self>>;

    /// Return referenced spans
    fn spans(&self) -> &[Span];
}

/// HNSW adapter delegating to existing `VectorIndex`
pub struct HnswBackend(pub crate::index::VectorIndex);

impl ApproxIndex for HnswBackend {
    fn build(spans: Vec<Span>) -> Self {
        Self(crate::index::VectorIndex::build(spans))
    }

    fn search(&self, query_embedding: &[f32], k: usize) -> Result<Vec<ScoredSpan>> {
        self.0.search(query_embedding, k)
    }

    fn save_to_disk(&self, dir: &Path) -> Result<()> {
        self.0.save_to_disk(dir)
    }

    fn load_from_disk(dir: &Path) -> Result<Option<Self>> {
        Ok(crate::index::VectorIndex::load_from_disk(dir)?.map(HnswBackend))
    }

    fn spans(&self) -> &[Span] {
        self.0.spans()
    }
}

/// Instant backend (spike): owned vectors + brute-force search (placeholder)
///
/// Note: This is a spike scaffold to validate owned save/load and determinism,
/// not a performance implementation. It can be swapped with a real
/// instant-distance index while keeping the ApproxIndex API intact.
#[derive(Serialize, Deserialize)]
pub struct InstantBackend {
    dimension: usize,
    spans: Vec<Span>,
    embeddings: Vec<Vec<f32>>,
}

#[derive(Serialize, Deserialize)]
struct InstantBackendOnDisk {
    version: u32,
    dimension: usize,
    spans: Vec<SpanLite>,
    embeddings: Vec<Vec<f32>>,
}

#[derive(Serialize, Deserialize, Clone)]
struct SpanLite {
    id: String,
    artifact_id: String,
    start_line: usize,
    end_line: usize,
    text: String,
    token_count: usize,
    embedding_model: Option<String>,
}

impl From<&Span> for SpanLite {
    fn from(s: &Span) -> Self {
        SpanLite {
            id: s.id.clone(),
            artifact_id: s.artifact_id.clone(),
            start_line: s.start_line,
            end_line: s.end_line,
            text: s.text.clone(),
            token_count: s.token_count,
            embedding_model: s.embedding_model.clone(),
        }
    }
}

impl From<SpanLite> for Span {
    fn from(s: SpanLite) -> Self {
        Span {
            id: s.id,
            artifact_id: s.artifact_id,
            start_line: s.start_line,
            end_line: s.end_line,
            text: s.text,
            embedding: None, // set separately from embeddings vec
            embedding_model: s.embedding_model,
            token_count: s.token_count,
            metadata: None,
        }
    }
}

impl ApproxIndex for InstantBackend {
    fn build(spans: Vec<Span>) -> Self {
        // Collect embeddings; assume all spans have embeddings with same dimension
        let embeddings: Vec<Vec<f32>> = spans
            .iter()
            .map(|s| s.embedding.clone().unwrap_or_default())
            .collect();
        let dimension = embeddings.first().map(|e| e.len()).unwrap_or(0);
        Self {
            dimension,
            spans,
            embeddings,
        }
    }

    fn search(&self, query_embedding: &[f32], k: usize) -> Result<Vec<ScoredSpan>> {
        if query_embedding.len() != self.dimension || self.dimension == 0 {
            return Ok(Vec::new());
        }
        // Brute force cosine similarity (deterministic)
        let mut scored: Vec<ScoredSpan> = self
            .spans
            .iter()
            .zip(self.embeddings.iter())
            .map(|(s, emb)| {
                let score = if emb.len() == self.dimension {
                    cosine_similarity(query_embedding, emb)
                } else {
                    0.0
                };
                ScoredSpan {
                    span: s.clone(),
                    score,
                }
            })
            .collect();
        // Sort by score desc, then (artifact_id, start_line) for determinism
        scored.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.span.artifact_id.cmp(&b.span.artifact_id))
                .then_with(|| a.span.start_line.cmp(&b.span.start_line))
        });
        scored.truncate(k.min(scored.len()));
        Ok(scored)
    }

    fn save_to_disk(&self, dir: &Path) -> Result<()> {
        use std::fs;
        fs::create_dir_all(dir)?;
        // Store a reduced on-disk format that avoids serde_json dependency
        let slim: Vec<SpanLite> = self.spans.iter().map(SpanLite::from).collect();
        let on_disk = InstantBackendOnDisk {
            version: 1,
            dimension: self.dimension,
            spans: slim,
            embeddings: self.embeddings.clone(),
        };
        let data = bincode::serialize(&on_disk)
            .map_err(|e| crate::types::Error::Other(anyhow::anyhow!("serialize instant index: {}", e)))?;
        let tmp = dir.join("instant.idx.tmp");
        let dst = dir.join("instant.idx");
        fs::write(&tmp, data)?;
        fs::rename(tmp, dst)?;
        Ok(())
    }

    fn load_from_disk(dir: &Path) -> Result<Option<Self>> {
        use std::fs;
        let path = dir.join("instant.idx");
        if !path.exists() {
            return Ok(None);
        }
        let bytes = fs::read(path)?;
        let on_disk: InstantBackendOnDisk = bincode::deserialize(&bytes)
            .map_err(|e| crate::types::Error::Other(anyhow::anyhow!("deserialize instant index: {}", e)))?;
        if on_disk.version != 1 {
            return Ok(None);
        }
        // Reconstruct spans without embeddings; embeddings stored separately
        let spans: Vec<Span> = on_disk.spans.into_iter().map(Span::from).collect();
        Ok(Some(InstantBackend {
            dimension: on_disk.dimension,
            spans,
            embeddings: on_disk.embeddings,
        }))
    }

    fn spans(&self) -> &[Span] {
        &self.spans
    }
}

