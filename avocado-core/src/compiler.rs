//! Context compilation engine
//!
//! This is the heart of AvocadoDB - the deterministic context compiler.
//!
//! # Algorithm Overview
//!
//! 1. Embed the query
//! 2. Semantic search (vector similarity) - top 50
//! 3. Lexical search (keyword matching) - top 20
//! 4. Hybrid fusion (combine results with RRF)
//! 5. MMR diversification (reduce redundancy)
//! 6. Token budget packing (greedy selection)
//! 7. Deterministic sort (by artifact_id, start_line)
//! 8. Build WorkingSet with citations

use crate::db::Database;
use crate::embedding;
use crate::index::{cosine_similarity, VectorIndex};
use crate::types::{Citation, CompilerConfig, Result, ScoredSpan, Span, WorkingSet};
use std::collections::HashMap;

/// Compile a context working set for a query
///
/// # Arguments
///
/// * `query` - The search query
/// * `config` - Compiler configuration
/// * `db` - Database handle
/// * `index` - Vector index
/// * `api_key` - Optional OpenAI API key
///
/// # Returns
///
/// A deterministic WorkingSet with compiled context
pub async fn compile(
    query: &str,
    config: CompilerConfig,
    db: &Database,
    index: &VectorIndex,
    api_key: Option<&str>,
) -> Result<WorkingSet> {
    let start_time = std::time::Instant::now();
    let mut last_checkpoint = start_time;

    // Step 1: Embed query
    let query_embedding = embedding::embed_text(query, api_key).await?;
    log::debug!("Embed query: {}ms", last_checkpoint.elapsed().as_millis());
    last_checkpoint = std::time::Instant::now();

    // Step 2: Semantic search
    let semantic_results = index.search(&query_embedding, 50)?;
    log::debug!("Semantic search: {}ms", last_checkpoint.elapsed().as_millis());
    last_checkpoint = std::time::Instant::now();

    // Step 3: Lexical search
    let lexical_results = lexical_search(query, db, 20)?;
    log::debug!("Lexical search: {}ms", last_checkpoint.elapsed().as_millis());
    last_checkpoint = std::time::Instant::now();

    // Step 4: Hybrid fusion
    let mut candidates = hybrid_fusion(
        semantic_results,
        lexical_results,
        config.semantic_weight,
        config.lexical_weight,
    );
    log::debug!("Hybrid fusion: {}ms", last_checkpoint.elapsed().as_millis());
    last_checkpoint = std::time::Instant::now();

    // Step 5: MMR diversification (if enabled)
    if config.enable_mmr {
        candidates = apply_mmr(candidates, &query_embedding, config.mmr_lambda);
        log::debug!("MMR diversification: {}ms", last_checkpoint.elapsed().as_millis());
        last_checkpoint = std::time::Instant::now();
    }

    // Step 6: Pack into token budget
    let selected_spans = pack_token_budget(candidates, config.token_budget);
    log::debug!("Token packing: {}ms", last_checkpoint.elapsed().as_millis());
    last_checkpoint = std::time::Instant::now();

    // Step 7: Sort deterministically
    let sorted_spans = deterministic_sort(selected_spans);
    log::debug!("Deterministic sort: {}ms", last_checkpoint.elapsed().as_millis());
    last_checkpoint = std::time::Instant::now();

    // Step 8: Build context and citations
    let (context_text, citations) = build_context(&sorted_spans, db)?;
    log::debug!("Build context: {}ms", last_checkpoint.elapsed().as_millis());
    last_checkpoint = std::time::Instant::now();

    // Count tokens
    let tokens_used = count_tokens(&context_text);
    log::debug!("Count tokens: {}ms", last_checkpoint.elapsed().as_millis());

    let compilation_time_ms = start_time.elapsed().as_millis() as u64;
    log::info!("Total compilation time: {}ms", compilation_time_ms);

    Ok(WorkingSet {
        text: context_text.clone(),
        spans: sorted_spans,
        citations,
        tokens_used,
        query: query.to_string(),
        compilation_time_ms,
    })
}

/// Perform lexical (keyword) search
///
/// Simple keyword matching for Phase 1. Could be enhanced with BM25 later.
///
/// # Arguments
///
/// * `query` - The search query
/// * `db` - Database handle
/// * `limit` - Maximum number of results
///
/// # Returns
///
/// Vector of scored spans
fn lexical_search(query: &str, db: &Database, limit: usize) -> Result<Vec<ScoredSpan>> {
    let spans = db.search_spans(query, limit)?;

    // Simple scoring: count keyword matches
    let query_lower = query.to_lowercase();
    let keywords: Vec<&str> = query_lower.split_whitespace().collect();

    let scored: Vec<ScoredSpan> = spans
        .into_iter()
        .map(|span| {
            let text_lower = span.text.to_lowercase();
            let matches = keywords
                .iter()
                .filter(|kw| text_lower.contains(**kw))
                .count();

            ScoredSpan {
                span,
                score: matches as f32 / keywords.len().max(1) as f32,
            }
        })
        .collect();

    Ok(scored)
}

/// Hybrid fusion using Reciprocal Rank Fusion (RRF)
///
/// Combines semantic and lexical search results with weighted scores.
///
/// # Arguments
///
/// * `semantic` - Semantic search results
/// * `lexical` - Lexical search results
/// * `semantic_weight` - Weight for semantic results
/// * `lexical_weight` - Weight for lexical results
///
/// # Returns
///
/// Merged and sorted list of scored spans
fn hybrid_fusion(
    semantic: Vec<ScoredSpan>,
    lexical: Vec<ScoredSpan>,
    semantic_weight: f32,
    lexical_weight: f32,
) -> Vec<ScoredSpan> {
    let mut scores: HashMap<String, (Span, f32)> = HashMap::new();

    // Add semantic scores using RRF
    for (rank, scored) in semantic.into_iter().enumerate() {
        let rrf_score = semantic_weight / (60.0 + rank as f32);
        scores.insert(
            scored.span.id.clone(),
            (scored.span, rrf_score),
        );
    }

    // Add lexical scores using RRF
    for (rank, scored) in lexical.into_iter().enumerate() {
        let rrf_score = lexical_weight / (60.0 + rank as f32);
        scores
            .entry(scored.span.id.clone())
            .and_modify(|(_, score)| *score += rrf_score)
            .or_insert((scored.span, rrf_score));
    }

    // Convert back to sorted list
    let mut results: Vec<ScoredSpan> = scores
        .into_iter()
        .map(|(_, (span, score))| ScoredSpan { span, score })
        .collect();

    // Sort by score descending
    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

    results
}

/// Apply Maximal Marginal Relevance (MMR) for diversity
///
/// MMR balances relevance and diversity to avoid redundant content.
///
/// # Arguments
///
/// * `candidates` - Candidate spans sorted by relevance
/// * `query_embedding` - The query vector
/// * `lambda` - Balance parameter (0.0 = max diversity, 1.0 = max relevance)
///
/// # Returns
///
/// Diversified list of scored spans
///
/// TODO: Implement MMR algorithm
/// This is a key business logic decision that affects result quality.
/// Consider the trade-offs:
/// - Higher lambda = more relevant but potentially redundant
/// - Lower lambda = more diverse but potentially less relevant
/// - How to handle spans without embeddings?
fn apply_mmr(
    candidates: Vec<ScoredSpan>,
    _query_embedding: &[f32],
    lambda: f32,
) -> Vec<ScoredSpan> {
    if candidates.is_empty() {
        return vec![];
    }

    let mut selected = Vec::new();
    let mut remaining = candidates;

    // Select first span (highest relevance)
    if let Some(first) = remaining.first() {
        selected.push(first.clone());
        remaining.remove(0);
    }

    // Iteratively select diverse spans using MMR
    const TARGET_SPANS: usize = 30;

    while !remaining.is_empty() && selected.len() < TARGET_SPANS {
        let mut best_mmr_score = f32::NEG_INFINITY;
        let mut best_idx = 0;

        for (idx, candidate) in remaining.iter().enumerate() {
            // Relevance to query (using original score from hybrid fusion)
            let relevance = candidate.score;

            // Calculate maximum similarity to already selected spans
            let max_similarity = if let Some(ref candidate_emb) = candidate.span.embedding {
                selected
                    .iter()
                    .filter_map(|selected_span: &ScoredSpan| {
                        selected_span.span.embedding.as_ref().map(|selected_emb| {
                            cosine_similarity(candidate_emb, selected_emb)
                        })
                    })
                    .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                    .unwrap_or(0.0)
            } else {
                // If no embedding, treat as having zero similarity
                0.0
            };

            // MMR score: balance relevance and diversity
            // lambda = 1.0 means pure relevance (no diversity penalty)
            // lambda = 0.0 means pure diversity (no relevance bonus)
            let mmr_score = lambda * relevance - (1.0 - lambda) * max_similarity;

            if mmr_score > best_mmr_score {
                best_mmr_score = mmr_score;
                best_idx = idx;
            }
        }

        // Add the best span to selected and remove from remaining
        selected.push(remaining.remove(best_idx));
    }

    selected
}

/// Pack spans into token budget using greedy selection
///
/// # Arguments
///
/// * `candidates` - Scored spans sorted by relevance
/// * `budget` - Maximum number of tokens
///
/// # Returns
///
/// Selected spans that fit within budget
///
/// TODO: Implement token budget packing
/// Consider:
/// - Should we always take the highest scored spans?
/// - Or try to maximize token utilization with a knapsack approach?
/// - How to handle very large spans that might waste budget?
fn pack_token_budget(candidates: Vec<ScoredSpan>, budget: usize) -> Vec<ScoredSpan> {
    let mut selected = Vec::new();
    let mut total_tokens = 0;

    // First pass: greedy selection of high-value spans
    let mut remaining = Vec::new();

    for candidate in candidates {
        let span_tokens = candidate.span.token_count;

        // Skip spans that are too large and would waste budget
        // (e.g., a span that's >40% of budget might leave too much unused)
        let remaining_budget = budget - total_tokens;
        let waste_ratio = if remaining_budget > 0 {
            (span_tokens as f32 - remaining_budget as f32) / budget as f32
        } else {
            1.0
        };

        if total_tokens + span_tokens <= budget {
            total_tokens += span_tokens;
            selected.push(candidate);
        } else if waste_ratio < 0.4 && span_tokens < budget {
            // Might fit later, keep for second pass
            remaining.push(candidate);
        }
    }

    // Second pass: try to fill remaining budget with smaller spans
    // This maximizes token utilization
    let remaining_budget = budget.saturating_sub(total_tokens);

    if remaining_budget > 0 {
        // Sort remaining by size (smallest first) to fill gaps
        remaining.sort_by_key(|s| s.span.token_count);

        for candidate in remaining {
            if total_tokens + candidate.span.token_count <= budget {
                total_tokens += candidate.span.token_count;
                selected.push(candidate);
            }
        }
    }

    selected
}

/// Sort spans deterministically
///
/// Critical for ensuring same query → same result every time.
/// Sorts by (artifact_id, start_line) to create canonical ordering.
///
/// # Arguments
///
/// * `spans` - Spans to sort
///
/// # Returns
///
/// Deterministically sorted spans
fn deterministic_sort(mut spans: Vec<ScoredSpan>) -> Vec<Span> {
    // Sort by (artifact_id, start_line) for deterministic ordering
    spans.sort_by(|a, b| {
        a.span
            .artifact_id
            .cmp(&b.span.artifact_id)
            .then_with(|| a.span.start_line.cmp(&b.span.start_line))
    });

    spans.into_iter().map(|s| s.span).collect()
}

/// Build context text and citations from spans
///
/// # Arguments
///
/// * `spans` - Selected spans
/// * `db` - Database handle
///
/// # Returns
///
/// (context_text, citations)
fn build_context(spans: &[Span], db: &Database) -> Result<(String, Vec<Citation>)> {
    let mut context_parts = Vec::new();
    let mut citations = Vec::new();

    for (idx, span) in spans.iter().enumerate() {
        // Get artifact path for citation
        let artifact = db.get_artifact(&span.artifact_id)?;
        let artifact_path = artifact
            .as_ref()
            .map(|a| a.path.clone())
            .unwrap_or_else(|| "unknown".to_string());

        // Add citation marker
        let citation_marker = format!("[{}]", idx + 1);

        // Build context chunk with citation
        let chunk = format!(
            "{} {}\nLines {}-{}\n\n{}",
            citation_marker, artifact_path, span.start_line, span.end_line, span.text
        );

        context_parts.push(chunk);

        // Create citation
        citations.push(Citation {
            span_id: span.id.clone(),
            artifact_id: span.artifact_id.clone(),
            artifact_path,
            start_line: span.start_line,
            end_line: span.end_line,
            score: 0.0, // TODO: Preserve score from search
        });
    }

    let context_text = context_parts.join("\n\n---\n\n");

    Ok((context_text, citations))
}

use std::sync::OnceLock;

/// Cached tiktoken tokenizer for performance
static TOKENIZER: OnceLock<tiktoken_rs::CoreBPE> = OnceLock::new();

/// Count tokens in text using cached tiktoken-rs tokenizer
///
/// Note: If tiktoken fails to initialize, this will panic. In practice,
/// tiktoken should never fail to initialize unless there's a system issue.
fn count_tokens(text: &str) -> usize {
    // Use cached tiktoken tokenizer for accurate counting
    let tokenizer = TOKENIZER.get_or_init(|| {
        tiktoken_rs::cl100k_base().expect("Failed to initialize tiktoken tokenizer")
    });

    tokenizer.encode_with_special_tokens(text).len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deterministic_sort() {
        let spans = vec![
            ScoredSpan {
                span: Span {
                    id: "1".to_string(),
                    artifact_id: "b".to_string(),
                    start_line: 10,
                    end_line: 20,
                    text: "".to_string(),
                    embedding: None,
                    embedding_model: None,
                    token_count: 10,
                    metadata: None,
                },
                score: 0.9,
            },
            ScoredSpan {
                span: Span {
                    id: "2".to_string(),
                    artifact_id: "a".to_string(),
                    start_line: 5,
                    end_line: 15,
                    text: "".to_string(),
                    embedding: None,
                    embedding_model: None,
                    token_count: 10,
                    metadata: None,
                },
                score: 0.95,
            },
        ];

        let sorted = deterministic_sort(spans);

        // Should be sorted by artifact_id first ("a" before "b")
        assert_eq!(sorted[0].artifact_id, "a");
        assert_eq!(sorted[1].artifact_id, "b");
    }

    #[test]
    fn test_pack_token_budget() {
        let candidates = vec![
            ScoredSpan {
                span: Span {
                    id: "1".to_string(),
                    artifact_id: "a".to_string(),
                    start_line: 1,
                    end_line: 10,
                    text: "".to_string(),
                    embedding: None,
                    embedding_model: None,
                    token_count: 100,
                    metadata: None,
                },
                score: 1.0,
            },
            ScoredSpan {
                span: Span {
                    id: "2".to_string(),
                    artifact_id: "a".to_string(),
                    start_line: 11,
                    end_line: 20,
                    text: "".to_string(),
                    embedding: None,
                    embedding_model: None,
                    token_count: 150,
                    metadata: None,
                },
                score: 0.9,
            },
        ];

        let selected = pack_token_budget(candidates, 200);

        // Should select first span only (100 tokens)
        // Second span would exceed budget
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].span.id, "1");
    }
}
