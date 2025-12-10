//! Evaluation module for retrieval quality metrics
//!
//! Provides tools for evaluating retrieval quality using golden queries
//! with expected results. Calculates recall@k, precision@k, MRR, and latency.

use crate::compiler;
use crate::db::Database;
use crate::index::VectorIndex;
use crate::types::{CompilerConfig, EvalResult, EvalSummary, GoldenQuery, Result};
use std::collections::HashSet;

/// Evaluate a set of golden queries and compute metrics
///
/// # Arguments
///
/// * `queries` - Golden queries with expected results
/// * `db` - Database handle
/// * `index` - Vector index
/// * `config` - Compiler configuration
///
/// # Returns
///
/// Evaluation summary with aggregate metrics
pub async fn evaluate(
    queries: &[GoldenQuery],
    db: &Database,
    index: &VectorIndex,
    config: &CompilerConfig,
) -> Result<EvalSummary> {
    let mut results = Vec::with_capacity(queries.len());
    let mut latencies = Vec::with_capacity(queries.len());

    for query in queries {
        let start = std::time::Instant::now();

        // Compile context for this query
        let working_set = compiler::compile(&query.query, config.clone(), db, index, None).await?;

        let latency_ms = start.elapsed().as_millis() as u64;
        latencies.push(latency_ms);

        // Get artifact paths from results
        let result_paths: Vec<String> = working_set
            .citations
            .iter()
            .map(|c| c.artifact_path.clone())
            .collect();

        // Calculate metrics
        let (recall, precision, mrr) = calculate_metrics(&query.expected_paths, &result_paths, query.k);

        results.push(EvalResult {
            query: query.query.clone(),
            recall_at_k: recall,
            precision_at_k: precision,
            mrr,
            latency_ms,
        });
    }

    // Calculate aggregate metrics
    let mean_recall = results.iter().map(|r| r.recall_at_k).sum::<f32>() / results.len() as f32;
    let mean_precision = results.iter().map(|r| r.precision_at_k).sum::<f32>() / results.len() as f32;
    let mean_mrr = results.iter().map(|r| r.mrr).sum::<f32>() / results.len() as f32;

    // Calculate latency percentiles
    latencies.sort();
    let p50_idx = latencies.len() / 2;
    let p99_idx = (latencies.len() as f32 * 0.99) as usize;
    let p50_latency_ms = latencies.get(p50_idx).copied().unwrap_or(0);
    let p99_latency_ms = latencies.get(p99_idx.min(latencies.len().saturating_sub(1))).copied().unwrap_or(0);

    Ok(EvalSummary {
        mean_recall,
        mean_precision,
        mean_mrr,
        p50_latency_ms,
        p99_latency_ms,
        query_count: queries.len(),
        results,
    })
}

/// Calculate recall@k, precision@k, and MRR for a single query
///
/// # Arguments
///
/// * `expected` - Expected artifact paths
/// * `results` - Retrieved artifact paths (in order)
/// * `k` - Top k to consider
///
/// # Returns
///
/// (recall_at_k, precision_at_k, mrr)
fn calculate_metrics(expected: &[String], results: &[String], k: usize) -> (f32, f32, f32) {
    let expected_set: HashSet<&String> = expected.iter().collect();
    let top_k: Vec<&String> = results.iter().take(k).collect();
    let top_k_set: HashSet<&String> = top_k.iter().copied().collect();

    // Recall@k: What fraction of expected items appear in top k?
    let found = expected_set.intersection(&top_k_set).count();
    let recall = if expected.is_empty() {
        1.0 // No expected items means perfect recall
    } else {
        found as f32 / expected.len() as f32
    };

    // Precision@k: What fraction of top k items are expected?
    let precision = if top_k.is_empty() {
        0.0
    } else {
        found as f32 / top_k.len() as f32
    };

    // MRR: Reciprocal of rank of first relevant result
    let mrr = results
        .iter()
        .position(|r| expected_set.contains(r))
        .map(|pos| 1.0 / (pos + 1) as f32)
        .unwrap_or(0.0);

    (recall, precision, mrr)
}

/// Load golden queries from JSON file
///
/// # Arguments
///
/// * `path` - Path to JSON file
///
/// # Returns
///
/// Vector of golden queries
pub fn load_golden_queries(path: &std::path::Path) -> Result<Vec<GoldenQuery>> {
    let content = std::fs::read_to_string(path)?;
    let queries: Vec<GoldenQuery> = serde_json::from_str(&content)?;
    Ok(queries)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_metrics_perfect() {
        let expected = vec!["a.md".to_string(), "b.md".to_string()];
        let results = vec!["a.md".to_string(), "b.md".to_string(), "c.md".to_string()];

        let (recall, precision, mrr) = calculate_metrics(&expected, &results, 3);

        assert_eq!(recall, 1.0); // All expected found
        assert!((precision - 2.0 / 3.0).abs() < 0.001); // 2/3 precision
        assert_eq!(mrr, 1.0); // First result is relevant
    }

    #[test]
    fn test_calculate_metrics_partial() {
        let expected = vec!["a.md".to_string(), "b.md".to_string()];
        let results = vec!["c.md".to_string(), "a.md".to_string(), "d.md".to_string()];

        let (recall, precision, mrr) = calculate_metrics(&expected, &results, 3);

        assert_eq!(recall, 0.5); // 1/2 expected found
        assert!((precision - 1.0 / 3.0).abs() < 0.001); // 1/3 precision
        assert_eq!(mrr, 0.5); // First relevant at position 2
    }

    #[test]
    fn test_calculate_metrics_none_found() {
        let expected = vec!["a.md".to_string(), "b.md".to_string()];
        let results = vec!["c.md".to_string(), "d.md".to_string()];

        let (recall, precision, mrr) = calculate_metrics(&expected, &results, 3);

        assert_eq!(recall, 0.0);
        assert_eq!(precision, 0.0);
        assert_eq!(mrr, 0.0);
    }
}
