//! Embedding generation using OpenAI
//!
//! This module handles generating vector embeddings for text using OpenAI's API.
//! For Phase 1, we use text-embedding-ada-002 (1536 dimensions).

use crate::types::{Error, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;

const OPENAI_API_URL: &str = "https://api.openai.com/v1/embeddings";
const MODEL: &str = "text-embedding-ada-002";
const DIMENSION: usize = 1536;

/// OpenAI API request format
#[derive(Debug, Serialize)]
struct EmbeddingRequest {
    model: String,
    input: Vec<String>,
}

/// OpenAI API response format
#[derive(Debug, Deserialize)]
struct EmbeddingResponse {
    data: Vec<EmbeddingData>,
}

#[derive(Debug, Deserialize)]
struct EmbeddingData {
    embedding: Vec<f32>,
    index: usize,
}

/// Embed a single text string
///
/// # Arguments
///
/// * `text` - The text to embed
/// * `api_key` - OpenAI API key (optional, uses OPENAI_API_KEY env var if not provided)
///
/// # Returns
///
/// A vector of 1536 floats representing the embedding
pub async fn embed_text(text: &str, api_key: Option<&str>) -> Result<Vec<f32>> {
    let results = embed_batch(vec![text], api_key).await?;
    results.into_iter().next().ok_or_else(|| {
        Error::Embedding("No embedding returned from API".to_string())
    })
}

/// Embed multiple text strings in a single API call
///
/// OpenAI allows up to 2048 inputs per request, so this is much more efficient
/// than calling embed_text repeatedly.
///
/// # Arguments
///
/// * `texts` - Vector of text strings to embed
/// * `api_key` - OpenAI API key (optional, uses OPENAI_API_KEY env var if not provided)
///
/// # Returns
///
/// A vector of embeddings, in the same order as the input texts
pub async fn embed_batch(texts: Vec<&str>, api_key: Option<&str>) -> Result<Vec<Vec<f32>>> {
    let api_key = api_key
        .map(|s| s.to_string())
        .or_else(|| env::var("OPENAI_API_KEY").ok())
        .ok_or_else(|| {
            Error::Embedding(
                "OPENAI_API_KEY environment variable not set and no API key provided".to_string(),
            )
        })?;

    if texts.is_empty() {
        return Ok(vec![]);
    }

    // OpenAI limit is 2048 inputs per request
    if texts.len() > 2048 {
        return Err(Error::InvalidInput(format!(
            "Too many texts to embed at once: {} (max 2048)",
            texts.len()
        )));
    }

    let client = Client::new();

    let request = EmbeddingRequest {
        model: MODEL.to_string(),
        input: texts.iter().map(|s| s.to_string()).collect(),
    };

    let response = client
        .post(OPENAI_API_URL)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&request)
        .send()
        .await
        .map_err(|e| Error::Embedding(format!("API request failed: {}", e)))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(Error::Embedding(format!(
            "API returned error {}: {}",
            status, body
        )));
    }

    let embedding_response: EmbeddingResponse = response
        .json()
        .await
        .map_err(|e| Error::Embedding(format!("Failed to parse response: {}", e)))?;

    // Sort by index to ensure correct ordering
    let mut data = embedding_response.data;
    data.sort_by_key(|d| d.index);

    let embeddings: Vec<Vec<f32>> = data.into_iter().map(|d| d.embedding).collect();

    // Verify all embeddings have correct dimension
    for emb in &embeddings {
        if emb.len() != DIMENSION {
            return Err(Error::Embedding(format!(
                "Unexpected embedding dimension: {} (expected {})",
                emb.len(),
                DIMENSION
            )));
        }
    }

    Ok(embeddings)
}

/// Get the embedding model name
pub fn embedding_model() -> &'static str {
    MODEL
}

/// Get the embedding dimension
pub fn embedding_dimension() -> usize {
    DIMENSION
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedding_constants() {
        assert_eq!(embedding_model(), "text-embedding-ada-002");
        assert_eq!(embedding_dimension(), 1536);
    }

    #[tokio::test]
    #[ignore] // Only run when OPENAI_API_KEY is set
    async fn test_embed_text() {
        let result = embed_text("Hello, world!", None).await;
        if env::var("OPENAI_API_KEY").is_ok() {
            let embedding = result.unwrap();
            assert_eq!(embedding.len(), 1536);
        } else {
            assert!(result.is_err());
        }
    }

    #[tokio::test]
    #[ignore] // Only run when OPENAI_API_KEY is set
    async fn test_embed_batch() {
        let texts = vec!["Hello", "World", "Test"];
        let result = embed_batch(texts, None).await;

        if env::var("OPENAI_API_KEY").is_ok() {
            let embeddings = result.unwrap();
            assert_eq!(embeddings.len(), 3);
            for emb in embeddings {
                assert_eq!(emb.len(), 1536);
            }
        } else {
            assert!(result.is_err());
        }
    }
}
