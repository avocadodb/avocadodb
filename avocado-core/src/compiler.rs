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
use crate::storage::StorageBackend;
use crate::types::{
    ChunkingParams, Citation, CompilerConfig, ExplainCandidate, ExplainPlan, ExplainThresholds,
    ExplainTiming, IndexParams, Manifest, Result, ScoredSpan, Span, WorkingSet,
};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

/// AvocadoDB version for manifest
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Compile a context working set using a StorageBackend
///
/// This is the backend-agnostic version of compile that works with any
/// StorageBackend implementation (SQLite, PostgreSQL, etc.)
///
/// # Arguments
///
/// * `query` - The search query
/// * `config` - Compiler configuration
/// * `backend` - Storage backend implementation
/// * `api_key` - Optional OpenAI API key
///
/// # Returns
///
/// A deterministic WorkingSet with compiled context
pub async fn compile_with_backend<B: StorageBackend>(
    query: &str,
    config: CompilerConfig,
    backend: &B,
    api_key: Option<&str>,
) -> Result<WorkingSet> {
    compile_with_backend_options(query, config, backend, api_key, false).await
}

/// Compile a context working set using a StorageBackend with explain option
///
/// # Arguments
///
/// * `query` - The search query
/// * `config` - Compiler configuration
/// * `backend` - Storage backend implementation
/// * `api_key` - Optional OpenAI API key
/// * `explain` - Whether to generate explain plan
///
/// # Returns
///
/// A deterministic WorkingSet with compiled context, optionally with explain plan
pub async fn compile_with_backend_options<B: StorageBackend>(
    query: &str,
    config: CompilerConfig,
    backend: &B,
    api_key: Option<&str>,
    explain: bool,
) -> Result<WorkingSet> {
    let start_time = std::time::Instant::now();
    let mut timing = ExplainTiming::default();

    // Step 1: Embed query
    let t0 = std::time::Instant::now();
    let query_embedding = embedding::embed_text(query, None, api_key).await?;
    timing.embed_query_ms = t0.elapsed().as_millis() as u64;

    // Hash the query embedding for explain plan
    let query_embedding_hash = if explain {
        let mut hasher = Sha256::new();
        for f in &query_embedding {
            hasher.update(f.to_le_bytes());
        }
        format!("{:x}", hasher.finalize())
    } else {
        String::new()
    };

    // Step 2: Semantic search using backend's vector search
    let t0 = std::time::Instant::now();
    let vector_search = backend.get_vector_search().await?;
    let semantic_results = vector_search.search(&query_embedding, 50).await?;
    timing.semantic_search_ms = t0.elapsed().as_millis() as u64;

    // Convert to ScoredSpan format
    let semantic_spans: Vec<ScoredSpan> = semantic_results
        .into_iter()
        .map(|r| ScoredSpan {
            span: r.span,
            score: r.score,
        })
        .collect();

    // Capture semantic candidates for explain
    let semantic_candidates = if explain {
        scored_spans_to_candidates_async(&semantic_spans, backend).await
    } else {
        vec![]
    };

    // Step 3: Lexical search using backend
    let t0 = std::time::Instant::now();
    let lexical_spans = backend.search_spans(query, 20).await?;
    timing.lexical_search_ms = t0.elapsed().as_millis() as u64;

    // Convert lexical to ScoredSpan (with decreasing scores by rank)
    let lexical_scored: Vec<ScoredSpan> = lexical_spans
        .into_iter()
        .enumerate()
        .map(|(i, span)| ScoredSpan {
            span,
            score: 1.0 - (i as f32 * 0.05),
        })
        .collect();

    let lexical_candidates = if explain {
        scored_spans_to_candidates_async(&lexical_scored, backend).await
    } else {
        vec![]
    };

    // Step 4: Hybrid fusion
    let t0 = std::time::Instant::now();
    let mut candidates = hybrid_fusion(
        semantic_spans,
        lexical_scored,
        config.semantic_weight,
        config.lexical_weight,
    );
    timing.fusion_ms = t0.elapsed().as_millis() as u64;

    let fused_candidates = if explain {
        scored_spans_to_candidates_async(&candidates, backend).await
    } else {
        vec![]
    };

    // Step 5: MMR diversification
    let t0 = std::time::Instant::now();
    if config.enable_mmr {
        candidates = apply_mmr(candidates, &query_embedding, config.mmr_lambda);
    }
    timing.mmr_ms = t0.elapsed().as_millis() as u64;

    let mmr_candidates = if explain {
        scored_spans_to_candidates_async(&candidates, backend).await
    } else {
        vec![]
    };

    // Step 6: Token budget packing
    let t0 = std::time::Instant::now();
    let selected_spans = pack_token_budget(candidates, config.token_budget);
    timing.packing_ms = t0.elapsed().as_millis() as u64;

    let packed_candidates = if explain {
        scored_spans_to_candidates_async(&selected_spans, backend).await
    } else {
        vec![]
    };

    // Step 7: Deterministic sort
    let sorted_scored_spans = deterministic_sort_with_scores(selected_spans);

    let final_candidates = if explain {
        scored_spans_to_candidates_async(&sorted_scored_spans, backend).await
    } else {
        vec![]
    };

    // Step 8: Build context with citations
    let t0 = std::time::Instant::now();
    let (context_text, citations, sorted_spans) =
        build_context_with_backend(&sorted_scored_spans, backend).await?;
    timing.build_context_ms = t0.elapsed().as_millis() as u64;

    let tokens_used = count_tokens(&context_text);
    let compilation_time_ms = start_time.elapsed().as_millis() as u64;
    timing.total_ms = compilation_time_ms;

    // Build manifest
    let context_hash = {
        let mut hasher = Sha256::new();
        hasher.update(context_text.as_bytes());
        format!("{:x}", hasher.finalize())
    };

    let embedding_model = sorted_spans
        .first()
        .and_then(|s| s.embedding_model.clone())
        .unwrap_or_else(|| "all-MiniLM-L6-v2".to_string());

    let embedding_dimension = sorted_spans
        .first()
        .and_then(|s| s.embedding.as_ref().map(|e| e.len()))
        .unwrap_or(384);

    let manifest = Manifest {
        avocado_version: VERSION.to_string(),
        tokenizer: "cl100k_base".to_string(),
        embedding_model,
        embedding_dimension,
        chunking: ChunkingParams::default(),
        index: IndexParams::default(),
        context_hash,
    };

    // Build explain plan if requested
    let explain_plan = if explain {
        Some(ExplainPlan {
            query: query.to_string(),
            query_embedding_hash,
            semantic_candidates,
            lexical_candidates,
            fused_candidates,
            mmr_candidates,
            packed_candidates,
            final_candidates,
            timing,
            thresholds: ExplainThresholds {
                semantic_k: 50,
                lexical_k: 20,
                semantic_weight: config.semantic_weight,
                lexical_weight: config.lexical_weight,
                mmr_lambda: config.mmr_lambda,
                mmr_enabled: config.enable_mmr,
                token_budget: config.token_budget,
            },
        })
    } else {
        None
    };

    Ok(WorkingSet {
        text: context_text,
        spans: sorted_spans,
        citations,
        tokens_used,
        query: query.to_string(),
        compilation_time_ms,
        manifest: Some(manifest),
        explain: explain_plan,
    })
}

/// Convert scored spans to explain candidates (async version for StorageBackend)
async fn scored_spans_to_candidates_async<B: StorageBackend>(
    spans: &[ScoredSpan],
    backend: &B,
) -> Vec<ExplainCandidate> {
    let mut candidates = Vec::with_capacity(spans.len());
    for (idx, scored) in spans.iter().enumerate() {
        let artifact_path = backend
            .get_artifact(&scored.span.artifact_id)
            .await
            .ok()
            .flatten()
            .map(|a| a.path)
            .unwrap_or_else(|| "unknown".to_string());

        candidates.push(ExplainCandidate {
            span_id: scored.span.id.clone(),
            artifact_path,
            lines: (scored.span.start_line, scored.span.end_line),
            score: scored.score,
            tokens: scored.span.token_count,
            rank: idx + 1,
        });
    }
    candidates
}

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
    compile_with_options(query, config, db, index, api_key, false).await
}

/// Compile a context working set for a query with explain option
///
/// # Arguments
///
/// * `query` - The search query
/// * `config` - Compiler configuration
/// * `db` - Database handle
/// * `index` - Vector index
/// * `api_key` - Optional OpenAI API key
/// * `explain` - Whether to generate explain plan
///
/// # Returns
///
/// A deterministic WorkingSet with compiled context, optionally with explain plan
pub async fn compile_with_options(
    query: &str,
    config: CompilerConfig,
    db: &Database,
    index: &VectorIndex,
    api_key: Option<&str>,
    explain: bool,
) -> Result<WorkingSet> {
    let start_time = std::time::Instant::now();
    let mut timing = ExplainTiming::default();

    // Step 1: Embed query (uses local embeddings by default, no API key needed)
    let t0 = std::time::Instant::now();
    let query_embedding = embedding::embed_text(query, None, api_key).await?;
    timing.embed_query_ms = t0.elapsed().as_millis() as u64;
    log::debug!("Embed query: {}ms", timing.embed_query_ms);

    // Hash the query embedding for explain plan
    let query_embedding_hash = if explain {
        let mut hasher = Sha256::new();
        for f in &query_embedding {
            hasher.update(f.to_le_bytes());
        }
        format!("{:x}", hasher.finalize())
    } else {
        String::new()
    };

    // Step 2: Semantic search
    let t0 = std::time::Instant::now();
    let semantic_results = index.search(&query_embedding, 50)?;
    timing.semantic_search_ms = t0.elapsed().as_millis() as u64;
    log::debug!("Semantic search: {}ms", timing.semantic_search_ms);

    // Capture semantic candidates for explain
    let semantic_candidates = if explain {
        scored_spans_to_candidates(&semantic_results, db)
    } else {
        vec![]
    };

    // Step 3: Lexical search
    let t0 = std::time::Instant::now();
    let lexical_results = lexical_search(query, db, 20)?;
    timing.lexical_search_ms = t0.elapsed().as_millis() as u64;
    log::debug!("Lexical search: {}ms", timing.lexical_search_ms);

    // Capture lexical candidates for explain
    let lexical_candidates = if explain {
        scored_spans_to_candidates(&lexical_results, db)
    } else {
        vec![]
    };

    // Step 4: Hybrid fusion
    let t0 = std::time::Instant::now();
    let mut candidates = hybrid_fusion(
        semantic_results,
        lexical_results,
        config.semantic_weight,
        config.lexical_weight,
    );
    timing.fusion_ms = t0.elapsed().as_millis() as u64;
    log::debug!("Hybrid fusion: {}ms", timing.fusion_ms);

    // Capture fused candidates for explain
    let fused_candidates = if explain {
        scored_spans_to_candidates(&candidates, db)
    } else {
        vec![]
    };

    // Step 5: MMR diversification (if enabled)
    let t0 = std::time::Instant::now();
    if config.enable_mmr {
        candidates = apply_mmr(candidates, &query_embedding, config.mmr_lambda);
    }
    timing.mmr_ms = t0.elapsed().as_millis() as u64;
    log::debug!("MMR diversification: {}ms", timing.mmr_ms);

    // Capture MMR candidates for explain
    let mmr_candidates = if explain {
        scored_spans_to_candidates(&candidates, db)
    } else {
        vec![]
    };

    // Step 6: Pack into token budget
    let t0 = std::time::Instant::now();
    let selected_spans = pack_token_budget(candidates, config.token_budget);
    timing.packing_ms = t0.elapsed().as_millis() as u64;
    log::debug!("Token packing: {}ms", timing.packing_ms);

    // Capture packed candidates for explain
    let packed_candidates = if explain {
        scored_spans_to_candidates(&selected_spans, db)
    } else {
        vec![]
    };

    // Step 7: Sort deterministically (but keep scores for citations)
    let sorted_scored_spans = deterministic_sort_with_scores(selected_spans);
    log::debug!("Deterministic sort: complete");

    // Capture final candidates for explain
    let final_candidates = if explain {
        scored_spans_to_candidates(&sorted_scored_spans, db)
    } else {
        vec![]
    };

    // Step 8: Build context and citations (preserve scores)
    let t0 = std::time::Instant::now();
    let (context_text, citations) = build_context(&sorted_scored_spans, db)?;

    // Extract spans for WorkingSet (without scores)
    let sorted_spans: Vec<Span> = sorted_scored_spans.iter().map(|s| s.span.clone()).collect();
    timing.build_context_ms = t0.elapsed().as_millis() as u64;
    log::debug!("Build context: {}ms", timing.build_context_ms);

    // Count tokens
    let tokens_used = count_tokens(&context_text);

    let compilation_time_ms = start_time.elapsed().as_millis() as u64;
    timing.total_ms = compilation_time_ms;
    log::info!("Total compilation time: {}ms", compilation_time_ms);

    // Generate manifest
    let context_hash = {
        let mut hasher = Sha256::new();
        hasher.update(context_text.as_bytes());
        format!("{:x}", hasher.finalize())
    };

    let embedding_model = sorted_spans
        .first()
        .and_then(|s| s.embedding_model.clone())
        .unwrap_or_else(|| "all-MiniLM-L6-v2".to_string());

    let embedding_dimension = sorted_spans
        .first()
        .and_then(|s| s.embedding.as_ref().map(|e| e.len()))
        .unwrap_or(384);

    let manifest = Manifest {
        avocado_version: VERSION.to_string(),
        tokenizer: "cl100k_base".to_string(),
        embedding_model,
        embedding_dimension,
        chunking: ChunkingParams::default(),
        index: IndexParams::default(),
        context_hash,
    };

    // Generate explain plan if requested
    let explain_plan = if explain {
        Some(ExplainPlan {
            query: query.to_string(),
            query_embedding_hash,
            semantic_candidates,
            lexical_candidates,
            fused_candidates,
            mmr_candidates,
            packed_candidates,
            final_candidates,
            timing,
            thresholds: ExplainThresholds {
                semantic_k: 50,
                lexical_k: 20,
                semantic_weight: config.semantic_weight,
                lexical_weight: config.lexical_weight,
                mmr_lambda: config.mmr_lambda,
                mmr_enabled: config.enable_mmr,
                token_budget: config.token_budget,
            },
        })
    } else {
        None
    };

    Ok(WorkingSet {
        text: context_text,
        spans: sorted_spans,
        citations,
        tokens_used,
        query: query.to_string(),
        compilation_time_ms,
        manifest: Some(manifest),
        explain: explain_plan,
    })
}

/// Convert scored spans to explain candidates
fn scored_spans_to_candidates(spans: &[ScoredSpan], db: &Database) -> Vec<ExplainCandidate> {
    spans
        .iter()
        .enumerate()
        .map(|(idx, scored)| {
            let artifact_path = db
                .get_artifact(&scored.span.artifact_id)
                .ok()
                .flatten()
                .map(|a| a.path)
                .unwrap_or_else(|| "unknown".to_string());

            ExplainCandidate {
                span_id: scored.span.id.clone(),
                artifact_path,
                lines: (scored.span.start_line, scored.span.end_line),
                score: scored.score,
                tokens: scored.span.token_count,
                rank: idx + 1,
            }
        })
        .collect()
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
        scores.insert(scored.span.id.clone(), (scored.span, rrf_score));
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

    // Sort by score descending with deterministic tiebreakers:
    // then by (artifact_id, start_line) to ensure canonical order on equal scores
    results.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.span.artifact_id.cmp(&b.span.artifact_id))
            .then_with(|| a.span.start_line.cmp(&b.span.start_line))
    });

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
/// Implements Maximal Marginal Relevance (MMR) algorithm to balance
/// relevance and diversity. Higher lambda = more relevant but potentially
/// redundant. Lower lambda = more diverse but potentially less relevant.
/// Spans without embeddings are handled by treating similarity as 0.0.
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
                        selected_span
                            .span
                            .embedding
                            .as_ref()
                            .map(|selected_emb| cosine_similarity(candidate_emb, selected_emb))
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

/// Pack spans into token budget using optimized greedy selection
///
/// Uses a two-pass greedy algorithm:
/// 1. First pass: Select high-value spans (score/token ratio) that fit
/// 2. Second pass: Fill remaining budget with smaller spans
///
/// This balances relevance (via scores) with token utilization efficiency.
/// A full knapsack DP would be O(n*budget) which is too slow for large datasets.
///
/// # Arguments
///
/// * `candidates` - Scored spans sorted by relevance
/// * `budget` - Maximum number of tokens
///
/// # Returns
///
/// Selected spans that fit within budget, optimized for score/token ratio
fn pack_token_budget(candidates: Vec<ScoredSpan>, budget: usize) -> Vec<ScoredSpan> {
    if candidates.is_empty() || budget == 0 {
        return vec![];
    }

    let mut selected = Vec::new();
    let mut total_tokens = 0;

    // Calculate value density (score per token) for each candidate
    // This helps prioritize high-value spans that use budget efficiently
    let mut candidates_with_density: Vec<(ScoredSpan, f32)> = candidates
        .into_iter()
        .map(|c| {
            let density = if c.span.token_count > 0 {
                c.score / c.span.token_count as f32
            } else {
                // Spans with 0 tokens get infinite density (shouldn't happen, but handle it)
                f32::INFINITY
            };
            (c, density)
        })
        .collect();

    // Sort by density (highest first), then by score as tiebreaker
    candidates_with_density.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                b.0.score
                    .partial_cmp(&a.0.score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    });

    // First pass: greedy selection of high-value spans
    let mut remaining = Vec::new();

    for (candidate, _density) in candidates_with_density {
        let span_tokens = candidate.span.token_count;

        // Skip spans that are too large (would waste too much budget)
        // Reject spans >50% of total budget to avoid poor utilization
        if span_tokens > budget / 2 {
            continue;
        }

        if total_tokens + span_tokens <= budget {
            total_tokens += span_tokens;
            selected.push(candidate);
        } else {
            // Might fit later, keep for second pass
            remaining.push(candidate);
        }
    }

    // Second pass: try to fill remaining budget with smaller spans
    // Sort by size (smallest first) to maximize utilization
    let remaining_budget = budget.saturating_sub(total_tokens);

    if remaining_budget > 0 && !remaining.is_empty() {
        // Sort by token count (ascending) to fill gaps efficiently
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

/// Deterministically sort spans (preserving scores)
///
/// Critical for ensuring same query → same result every time.
/// Sorts by (artifact_id, start_line) to create canonical ordering.
///
/// # Arguments
///
/// * `spans` - Scored spans to sort
///
/// # Returns
///
/// Deterministically sorted scored spans
fn deterministic_sort_with_scores(mut spans: Vec<ScoredSpan>) -> Vec<ScoredSpan> {
    // Sort by (artifact_id, start_line) for deterministic ordering
    spans.sort_by(|a, b| {
        a.span
            .artifact_id
            .cmp(&b.span.artifact_id)
            .then_with(|| a.span.start_line.cmp(&b.span.start_line))
    });

    spans
}

/// Sort spans deterministically (legacy, returns spans without scores)
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
#[allow(dead_code)]
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

/// Build context text and citations from scored spans
///
/// # Arguments
///
/// * `scored_spans` - Selected spans with scores
/// * `db` - Database handle
///
/// # Returns
///
/// (context_text, citations)
fn build_context(scored_spans: &[ScoredSpan], db: &Database) -> Result<(String, Vec<Citation>)> {
    let mut context_parts = Vec::new();
    let mut citations = Vec::new();

    for (idx, scored_span) in scored_spans.iter().enumerate() {
        let span = &scored_span.span;

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

        // Create citation (preserve score from ScoredSpan)
        citations.push(Citation {
            span_id: span.id.clone(),
            artifact_id: span.artifact_id.clone(),
            artifact_path,
            start_line: span.start_line,
            end_line: span.end_line,
            score: scored_span.score, // Preserve score from search
        });
    }

    let context_text = context_parts.join("\n\n---\n\n");

    Ok((context_text, citations))
}

/// Build context text and citations from scored spans using a StorageBackend
///
/// # Arguments
///
/// * `scored_spans` - Selected spans with scores
/// * `backend` - Storage backend implementation
///
/// # Returns
///
/// (context_text, citations, spans)
async fn build_context_with_backend<B: StorageBackend>(
    scored_spans: &[ScoredSpan],
    backend: &B,
) -> Result<(String, Vec<Citation>, Vec<Span>)> {
    let mut context_parts = Vec::new();
    let mut citations = Vec::new();
    let mut spans = Vec::new();

    for (idx, scored_span) in scored_spans.iter().enumerate() {
        let span = &scored_span.span;

        // Get artifact path for citation
        let artifact = backend.get_artifact(&span.artifact_id).await?;
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

        // Create citation (preserve score from ScoredSpan)
        citations.push(Citation {
            span_id: span.id.clone(),
            artifact_id: span.artifact_id.clone(),
            artifact_path,
            start_line: span.start_line,
            end_line: span.end_line,
            score: scored_span.score,
        });

        spans.push(span.clone());
    }

    let context_text = context_parts.join("\n\n---\n\n");

    Ok((context_text, citations, spans))
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
