//! Context compilation functions for AvocadoDB extension
//!
//! The main `avocado_compile` function performs deterministic context
//! compilation using the avocado pipeline:
//! Query -> Embed -> Vector Search -> MMR -> Token Packing -> Working Set

use crate::embedding::embed_sync;
use crate::error::AvocadoError;
use crate::spi::{self, SpanWithScore};
use pgrx::datum::JsonB;
use pgrx::prelude::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashSet;

/// Default configuration for context compilation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompilerConfig {
    /// Maximum tokens to include in context
    #[serde(default = "default_token_budget")]
    pub token_budget: i32,

    /// Enable Maximal Marginal Relevance diversification
    #[serde(default = "default_true")]
    pub enable_mmr: bool,

    /// MMR lambda: 0.0 = diversity only, 1.0 = relevance only
    #[serde(default = "default_mmr_lambda")]
    pub mmr_lambda: f64,

    /// Number of candidates to retrieve for initial search
    #[serde(default = "default_candidate_limit")]
    pub candidate_limit: i32,
}

fn default_token_budget() -> i32 {
    8000
}
fn default_true() -> bool {
    true
}
fn default_mmr_lambda() -> f64 {
    0.5
}
fn default_candidate_limit() -> i32 {
    50
}

impl Default for CompilerConfig {
    fn default() -> Self {
        Self {
            token_budget: default_token_budget(),
            enable_mmr: default_true(),
            mmr_lambda: default_mmr_lambda(),
            candidate_limit: default_candidate_limit(),
        }
    }
}

/// Working set output
#[derive(Debug, Serialize)]
struct WorkingSet {
    /// Compiled context text
    text: String,
    /// Included spans
    spans: Vec<SpanInfo>,
    /// Citations for attribution
    citations: Vec<Citation>,
    /// Total tokens used
    tokens_used: i32,
    /// Original query
    query: String,
    /// Context hash (for determinism verification)
    context_hash: String,
    /// Compilation time in milliseconds
    compilation_time_ms: u64,
}

#[derive(Debug, Serialize)]
struct SpanInfo {
    span_id: String,
    artifact_path: String,
    lines: (i32, i32),
    score: f64,
    tokens: i32,
}

#[derive(Debug, Serialize)]
struct Citation {
    span_id: String,
    artifact_path: String,
    lines: (i32, i32),
    score: f64,
}

/// Compile context for a query
///
/// This is the main entry point for deterministic context compilation.
/// It performs:
/// 1. Query embedding
/// 2. Vector similarity search via pgvector
/// 3. Optional MMR diversification
/// 4. Token budget packing
/// 5. Deterministic ordering
///
/// # Arguments
/// * `query` - The user query to compile context for
/// * `config` - Optional JSONB configuration (token_budget, enable_mmr, etc.)
///
/// # Returns
/// JSONB working set with text, spans, citations, and metadata
///
/// # Example
/// ```sql
/// SELECT avocado_compile(
///     'How does authentication work?',
///     '{"token_budget": 4000}'::jsonb
/// );
/// ```
#[pg_extern]
fn avocado_compile(query: &str, config: default!(JsonB, "'{}'")) -> JsonB {
    let start = std::time::Instant::now();

    match compile_impl(query, &config.0, start) {
        Ok(working_set) => JsonB(serde_json::to_value(&working_set).unwrap()),
        Err(e) => e.report(),
    }
}

fn compile_impl(
    query: &str,
    config_json: &serde_json::Value,
    start: std::time::Instant,
) -> Result<WorkingSet, AvocadoError> {
    // Parse configuration
    let config: CompilerConfig =
        serde_json::from_value(config_json.clone()).unwrap_or_default();

    // 1. Embed the query
    let query_embedding = embed_sync(query)?;

    // 2. Vector search via pgvector
    let candidates = spi::search_similar_spans(&query_embedding, config.candidate_limit)?;

    if candidates.is_empty() {
        return Ok(WorkingSet {
            text: String::new(),
            spans: vec![],
            citations: vec![],
            tokens_used: 0,
            query: query.to_string(),
            context_hash: compute_hash(""),
            compilation_time_ms: start.elapsed().as_millis() as u64,
        });
    }

    // 3. Apply MMR if enabled
    let selected = if config.enable_mmr {
        apply_mmr(candidates, &query_embedding, config.mmr_lambda)
    } else {
        candidates
    };

    // 4. Pack within token budget
    let packed = pack_token_budget(selected, config.token_budget);

    // 5. Deterministic sort by artifact path + line number
    let mut final_spans = packed;
    final_spans.sort_by(|a, b| {
        a.artifact_path
            .cmp(&b.artifact_path)
            .then(a.span.start_line.cmp(&b.span.start_line))
    });

    // 6. Build output
    let mut text_parts = Vec::new();
    let mut spans = Vec::new();
    let mut citations = Vec::new();
    let mut tokens_used = 0;

    for span in &final_spans {
        // Add to text with source header
        text_parts.push(format!(
            "--- {} (lines {}-{}) ---\n{}",
            span.artifact_path, span.span.start_line, span.span.end_line, span.span.text
        ));

        let token_count = span.span.token_count.unwrap_or(0);
        tokens_used += token_count;

        spans.push(SpanInfo {
            span_id: span.span.id.clone(),
            artifact_path: span.artifact_path.clone(),
            lines: (span.span.start_line, span.span.end_line),
            score: span.score,
            tokens: token_count,
        });

        citations.push(Citation {
            span_id: span.span.id.clone(),
            artifact_path: span.artifact_path.clone(),
            lines: (span.span.start_line, span.span.end_line),
            score: span.score,
        });
    }

    let text = text_parts.join("\n\n");
    let context_hash = compute_hash(&text);

    Ok(WorkingSet {
        text,
        spans,
        citations,
        tokens_used,
        query: query.to_string(),
        context_hash,
        compilation_time_ms: start.elapsed().as_millis() as u64,
    })
}

/// Apply Maximal Marginal Relevance for diversification
fn apply_mmr(
    candidates: Vec<SpanWithScore>,
    _query_embedding: &[f32],
    lambda: f64,
) -> Vec<SpanWithScore> {
    if candidates.is_empty() {
        return candidates;
    }

    let mut selected: Vec<SpanWithScore> = Vec::new();
    let mut remaining: Vec<SpanWithScore> = candidates;
    let mut selected_texts: HashSet<String> = HashSet::new();

    // Greedy MMR selection
    while !remaining.is_empty() && selected.len() < 20 {
        let mut best_idx = 0;
        let mut best_mmr_score = f64::NEG_INFINITY;

        for (idx, candidate) in remaining.iter().enumerate() {
            // Relevance score (from vector search)
            let relevance = candidate.score;

            // Diversity score (max similarity to already selected)
            let diversity = if selected.is_empty() {
                0.0
            } else {
                // Use text overlap as a proxy for embedding similarity
                // (since we don't have embeddings loaded in memory)
                let candidate_words: HashSet<&str> =
                    candidate.span.text.split_whitespace().collect();

                selected
                    .iter()
                    .map(|s| {
                        let selected_words: HashSet<&str> =
                            s.span.text.split_whitespace().collect();
                        let intersection = candidate_words.intersection(&selected_words).count();
                        let union = candidate_words.union(&selected_words).count();
                        if union > 0 {
                            intersection as f64 / union as f64
                        } else {
                            0.0
                        }
                    })
                    .fold(0.0_f64, |a, b| a.max(b))
            };

            // MMR score: lambda * relevance - (1 - lambda) * max_similarity
            let mmr_score = lambda * relevance - (1.0 - lambda) * diversity;

            if mmr_score > best_mmr_score {
                best_mmr_score = mmr_score;
                best_idx = idx;
            }
        }

        let chosen = remaining.remove(best_idx);

        // Skip if we've already included very similar text
        let text_key = format!(
            "{}:{}",
            chosen.artifact_path,
            chosen.span.start_line / 10
        );
        if !selected_texts.contains(&text_key) {
            selected_texts.insert(text_key);
            selected.push(chosen);
        }
    }

    selected
}

/// Pack spans within token budget
fn pack_token_budget(spans: Vec<SpanWithScore>, budget: i32) -> Vec<SpanWithScore> {
    let mut packed = Vec::new();
    let mut tokens_used = 0;

    for span in spans {
        let span_tokens = span.span.token_count.unwrap_or(0);
        if tokens_used + span_tokens <= budget {
            tokens_used += span_tokens;
            packed.push(span);
        }
    }

    packed
}

/// Compute deterministic hash of text
fn compute_hash(text: &str) -> String {
    let hash = Sha256::digest(text.as_bytes());
    format!("{:x}", hash)[..24].to_string()
}
