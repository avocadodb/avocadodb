//! Working set diff module
//!
//! Compares two working sets to identify added, removed, and reranked spans.
//! Useful for auditing corpus changes and understanding retrieval drift.

use crate::types::{DiffEntry, RerankEntry, WorkingSet, WorkingSetDiff};
use std::collections::HashMap;

/// Type alias for span info: (rank, score, artifact_path, (start_line, end_line))
type SpanInfo<'a> = (usize, f32, &'a str, (usize, usize));

/// Compute diff between two working sets
///
/// # Arguments
///
/// * `before` - The "before" working set
/// * `after` - The "after" working set
///
/// # Returns
///
/// Diff showing added, removed, and reranked spans
pub fn diff_working_sets(before: &WorkingSet, after: &WorkingSet) -> WorkingSetDiff {
    // Build maps of span_id -> (rank, score, path, lines)
    let before_map: HashMap<&str, SpanInfo> = before
        .citations
        .iter()
        .enumerate()
        .map(|(rank, c)| {
            (
                c.span_id.as_str(),
                (rank + 1, c.score, c.artifact_path.as_str(), (c.start_line, c.end_line)),
            )
        })
        .collect();

    let after_map: HashMap<&str, SpanInfo> = after
        .citations
        .iter()
        .enumerate()
        .map(|(rank, c)| {
            (
                c.span_id.as_str(),
                (rank + 1, c.score, c.artifact_path.as_str(), (c.start_line, c.end_line)),
            )
        })
        .collect();

    let mut added = Vec::new();
    let mut removed = Vec::new();
    let mut reranked = Vec::new();

    // Find added spans (in after but not in before)
    for (span_id, (rank, score, path, lines)) in &after_map {
        if !before_map.contains_key(*span_id) {
            added.push(DiffEntry {
                span_id: span_id.to_string(),
                artifact_path: path.to_string(),
                lines: *lines,
                score: *score,
                rank: *rank,
            });
        }
    }

    // Find removed spans (in before but not in after)
    for (span_id, (rank, score, path, lines)) in &before_map {
        if !after_map.contains_key(*span_id) {
            removed.push(DiffEntry {
                span_id: span_id.to_string(),
                artifact_path: path.to_string(),
                lines: *lines,
                score: *score,
                rank: *rank,
            });
        }
    }

    // Find reranked spans (in both but different rank/score)
    for (span_id, (old_rank, old_score, path, _)) in &before_map {
        if let Some((new_rank, new_score, _, _)) = after_map.get(*span_id) {
            if old_rank != new_rank || (old_score - new_score).abs() > 0.001 {
                reranked.push(RerankEntry {
                    span_id: span_id.to_string(),
                    artifact_path: path.to_string(),
                    old_rank: *old_rank,
                    new_rank: *new_rank,
                    old_score: *old_score,
                    new_score: *new_score,
                });
            }
        }
    }

    // Sort for deterministic output
    added.sort_by_key(|e| e.rank);
    removed.sort_by_key(|e| e.rank);
    reranked.sort_by_key(|e| e.new_rank);

    WorkingSetDiff {
        query: after.query.clone(),
        before_hash: before.deterministic_hash(),
        after_hash: after.deterministic_hash(),
        added,
        removed,
        reranked,
    }
}

/// Check if two working sets are identical
pub fn working_sets_identical(before: &WorkingSet, after: &WorkingSet) -> bool {
    before.deterministic_hash() == after.deterministic_hash()
}

/// Summarize diff as human-readable string
pub fn summarize_diff(diff: &WorkingSetDiff) -> String {
    let mut parts = Vec::new();

    if diff.added.is_empty() && diff.removed.is_empty() && diff.reranked.is_empty() {
        return "No changes".to_string();
    }

    if !diff.added.is_empty() {
        parts.push(format!("{} added", diff.added.len()));
    }
    if !diff.removed.is_empty() {
        parts.push(format!("{} removed", diff.removed.len()));
    }
    if !diff.reranked.is_empty() {
        parts.push(format!("{} reranked", diff.reranked.len()));
    }

    parts.join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Citation;

    fn make_working_set(citations: Vec<(&str, &str, f32)>) -> WorkingSet {
        let cites: Vec<Citation> = citations
            .into_iter()
            .map(|(id, path, score)| Citation {
                span_id: id.to_string(),
                artifact_id: "art".to_string(),
                artifact_path: path.to_string(),
                start_line: 1,
                end_line: 10,
                score,
            })
            .collect();

        WorkingSet {
            text: cites.iter().map(|c| c.artifact_path.as_str()).collect::<Vec<_>>().join(","),
            spans: vec![],
            citations: cites,
            tokens_used: 100,
            query: "test".to_string(),
            compilation_time_ms: 50,
            manifest: None,
            explain: None,
        }
    }

    #[test]
    fn test_diff_identical() {
        let ws = make_working_set(vec![("1", "a.md", 0.9), ("2", "b.md", 0.8)]);
        let diff = diff_working_sets(&ws, &ws);

        assert!(diff.added.is_empty());
        assert!(diff.removed.is_empty());
        assert!(diff.reranked.is_empty());
    }

    #[test]
    fn test_diff_added() {
        let before = make_working_set(vec![("1", "a.md", 0.9)]);
        let after = make_working_set(vec![("1", "a.md", 0.9), ("2", "b.md", 0.8)]);
        let diff = diff_working_sets(&before, &after);

        assert_eq!(diff.added.len(), 1);
        assert_eq!(diff.added[0].span_id, "2");
        assert!(diff.removed.is_empty());
    }

    #[test]
    fn test_diff_removed() {
        let before = make_working_set(vec![("1", "a.md", 0.9), ("2", "b.md", 0.8)]);
        let after = make_working_set(vec![("1", "a.md", 0.9)]);
        let diff = diff_working_sets(&before, &after);

        assert!(diff.added.is_empty());
        assert_eq!(diff.removed.len(), 1);
        assert_eq!(diff.removed[0].span_id, "2");
    }

    #[test]
    fn test_diff_reranked() {
        let before = make_working_set(vec![("1", "a.md", 0.9), ("2", "b.md", 0.8)]);
        let after = make_working_set(vec![("2", "b.md", 0.95), ("1", "a.md", 0.85)]);
        let diff = diff_working_sets(&before, &after);

        assert!(diff.added.is_empty());
        assert!(diff.removed.is_empty());
        assert_eq!(diff.reranked.len(), 2);
    }
}
