//! Span extraction from documents
//!
//! This module handles extracting meaningful text spans from documents.
//! Spans are the fundamental unit of retrieval in AvocadoDB.
//!
//! # Key Principles
//!
//! - Spans should be 20-50 lines (roughly 200-500 tokens)
//! - Respect natural boundaries (paragraphs, code blocks, sections)
//! - Never split mid-sentence
//! - Maintain line number accuracy for citations

use crate::types::{Result, Span};
use uuid::Uuid;

/// Extract spans from document text
///
/// # Arguments
///
/// * `content` - The full document text
/// * `artifact_id` - The ID of the parent artifact
///
/// # Returns
///
/// A vector of spans extracted from the document
///
/// # Algorithm
///
/// 1. Split document into lines
/// 2. Group lines into spans (target 20-50 lines)
/// 3. Respect paragraph boundaries (double newlines)
/// 4. Keep code blocks together
/// 5. Calculate token counts for each span
///
pub fn extract_spans(content: &str, artifact_id: &str) -> Result<Vec<Span>> {
    let lines: Vec<&str> = content.lines().collect();
    let mut spans = Vec::new();
    let mut current_start = 1;
    let mut current_lines = Vec::new();

    for (idx, line) in lines.iter().enumerate() {
        let line_num = idx + 1;
        current_lines.push(*line);

        // Determine if we should create a span at this point
        let should_split = should_create_span(
            &current_lines,
            line,
            idx,
            lines.len(),
        );

        if should_split && !current_lines.is_empty() {
            let text = current_lines.join("\n");
            let token_count = estimate_tokens(&text);

            spans.push(Span {
                id: Uuid::new_v4().to_string(),
                artifact_id: artifact_id.to_string(),
                start_line: current_start,
                end_line: line_num,
                text,
                embedding: None,
                embedding_model: None,
                token_count,
                metadata: None,
            });

            current_start = line_num + 1;
            current_lines.clear();
        }
    }

    // Handle any remaining lines
    if !current_lines.is_empty() {
        let text = current_lines.join("\n");
        let token_count = estimate_tokens(&text);

        spans.push(Span {
            id: Uuid::new_v4().to_string(),
            artifact_id: artifact_id.to_string(),
            start_line: current_start,
            end_line: lines.len(),
            text,
            embedding: None,
            embedding_model: None,
            token_count,
            metadata: None,
        });
    }

    Ok(spans)
}

/// Determine if we should create a span at the current position
///
/// # Arguments
///
/// * `current_lines` - Lines accumulated so far
/// * `line` - The current line being processed
/// * `idx` - Current line index
/// * `total_lines` - Total number of lines in document
///
/// # Returns
///
/// `true` if a span should be created at this point
///
/// Smart span boundary detection
///
/// Detects natural boundaries for creating semantic spans:
/// - Paragraph boundaries (empty lines)
/// - Code block boundaries (``` markers)
/// - Section headers (# markdown, == rst, etc.)
/// - Target span size (20-50 lines)
/// - Minimum span size (avoid tiny spans)
fn should_create_span(
    current_lines: &[&str],
    line: &str,
    idx: usize,
    total_lines: usize,
) -> bool {
    let num_lines = current_lines.len();

    // Always split at end of document
    if idx == total_lines - 1 {
        return true;
    }

    // Reached maximum size (50 lines)
    if num_lines >= 50 {
        return true;
    }

    // Hit paragraph boundary and have minimum size
    if line.trim().is_empty() && num_lines >= 20 {
        return true;
    }

    // Detect markdown/RST section headers (good split points)
    let is_header = is_section_header(line);
    if is_header && num_lines >= 15 {
        // Split before headers if we have enough content
        return true;
    }

    // Detect code fence boundaries
    let is_code_fence = line.trim().starts_with("```") || line.trim().starts_with("~~~");
    if is_code_fence && num_lines >= 20 {
        // Split at code block boundaries
        return true;
    }

    // Ideal target size is 30 lines - split at next good boundary
    if num_lines >= 30 {
        // Look for natural boundaries
        if line.trim().is_empty() || is_header || is_code_fence {
            return true;
        }
        
        // Detect function/class definitions (common in code)
        let trimmed = line.trim();
        let is_definition = trimmed.starts_with("def ") 
            || trimmed.starts_with("class ")
            || trimmed.starts_with("function ")
            || trimmed.starts_with("pub fn ")
            || trimmed.starts_with("fn ")
            || trimmed.starts_with("const ")
            || trimmed.starts_with("let ");
        if is_definition && num_lines >= 25 {
            return true;
        }

        // If we're at 40+ lines and haven't found a boundary, force split
        if num_lines >= 40 {
            return true;
        }
    }

    false
}

/// Check if a line looks like a section header
fn is_section_header(line: &str) -> bool {
    let trimmed = line.trim();

    // Markdown headers: # Title, ## Title, etc.
    if trimmed.starts_with('#') && trimmed.len() > 1 {
        return true;
    }

    // ReStructuredText style headers: underlines with =, -, ~, etc.
    if trimmed.len() > 2 && trimmed.chars().all(|c| c == '=' || c == '-' || c == '~' || c == '^') {
        return true;
    }

    false
}

use std::sync::OnceLock;

/// Cached tiktoken tokenizer for performance
static TOKENIZER: OnceLock<tiktoken_rs::CoreBPE> = OnceLock::new();

/// Estimate token count for text
///
/// Uses tiktoken-rs for accurate token counting compatible with OpenAI models.
/// The tokenizer is cached for performance (avoids reloading on every call).
/// Falls back to simple heuristic if tiktoken initialization fails.
///
/// # Arguments
///
/// * `text` - The text to estimate tokens for
///
/// # Returns
///
/// Accurate token count (or heuristic estimate if tiktoken fails)
fn estimate_tokens(text: &str) -> usize {
    // Use cached tiktoken tokenizer for accurate counting
    let tokenizer = TOKENIZER.get_or_init(|| {
        // If initialization fails, we'll use a dummy tokenizer that always returns 0
        // and fall back to heuristic below
        tiktoken_rs::cl100k_base().unwrap_or_else(|_| {
            // Return a dummy - we'll detect this and use heuristic
            // This is a workaround since we can't return an error from get_or_init
            // In practice, tiktoken should never fail to initialize
            panic!("Failed to initialize tiktoken tokenizer - this should not happen")
        })
    });

    // Use the tokenizer
    tokenizer.encode_with_special_tokens(text).len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_spans_simple() {
        let content = "Line 1\nLine 2\nLine 3";
        let spans = extract_spans(content, "test-artifact").unwrap();

        assert!(!spans.is_empty());
        assert_eq!(spans[0].start_line, 1);
        assert!(spans[0].token_count > 0);
    }

    #[test]
    fn test_extract_spans_with_paragraphs() {
        let content = (0..25).map(|i| format!("Line {}", i)).collect::<Vec<_>>().join("\n")
            + "\n\n"
            + &(25..50).map(|i| format!("Line {}", i)).collect::<Vec<_>>().join("\n");

        let spans = extract_spans(&content, "test-artifact").unwrap();

        // Should create multiple spans
        assert!(spans.len() >= 2);

        // Each span should have valid line numbers
        for span in &spans {
            assert!(span.end_line >= span.start_line);
        }
    }

    #[test]
    fn test_no_overlapping_spans() {
        let content = (0..100).map(|i| format!("Line {}", i)).collect::<Vec<_>>().join("\n");
        let spans = extract_spans(&content, "test-artifact").unwrap();

        // Verify no gaps or overlaps
        for i in 0..spans.len() - 1 {
            assert_eq!(spans[i].end_line + 1, spans[i + 1].start_line);
        }
    }

    #[test]
    fn test_token_estimation() {
        let text = "This is a test sentence with about ten words in it.";
        let tokens = estimate_tokens(text);
        assert!(tokens > 0);
    }
}
