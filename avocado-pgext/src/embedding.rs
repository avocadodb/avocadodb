//! Synchronous embedding adapter for PostgreSQL extension
//!
//! PostgreSQL extensions run in a single-threaded context and cannot use
//! async/tokio. This module provides synchronous embedding generation
//! with multiple provider options:
//!
//! - **fastembed** (default): Pure Rust, ONNX-based, works offline
//! - **ollama**: Local LLM server with embedding models (e.g., bge-m3, nomic-embed)
//! - **openai**: OpenAI API (requires API key)
//!
//! Configure via PostgreSQL settings or environment variables.

use crate::error::{AvocadoError, Result};
use pgrx::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::{Mutex, OnceLock, RwLock};

// ============================================================================
// Configuration
// ============================================================================

/// Embedding provider selection
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EmbeddingProvider {
    /// Pure Rust fastembed (default, offline)
    Fastembed,
    /// Local Ollama server
    Ollama,
    /// OpenAI API
    #[cfg(feature = "openai")]
    OpenAI,
}

impl Default for EmbeddingProvider {
    fn default() -> Self {
        EmbeddingProvider::Fastembed
    }
}

/// Global embedding configuration
#[derive(Debug, Clone)]
pub struct EmbeddingConfig {
    pub provider: EmbeddingProvider,
    pub ollama_url: String,
    pub ollama_model: String,
    #[cfg(feature = "openai")]
    pub openai_api_key: Option<String>,
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            provider: EmbeddingProvider::Fastembed,
            ollama_url: "http://localhost:11434".to_string(),
            ollama_model: "bge-m3".to_string(),
            #[cfg(feature = "openai")]
            openai_api_key: None,
        }
    }
}

/// Global configuration (can be updated at runtime)
static EMBEDDING_CONFIG: OnceLock<RwLock<EmbeddingConfig>> = OnceLock::new();

fn get_config() -> &'static RwLock<EmbeddingConfig> {
    EMBEDDING_CONFIG.get_or_init(|| {
        // Try to read from environment
        let provider = std::env::var("AVOCADO_EMBEDDING_PROVIDER")
            .map(|p| match p.to_lowercase().as_str() {
                "ollama" => EmbeddingProvider::Ollama,
                #[cfg(feature = "openai")]
                "openai" => EmbeddingProvider::OpenAI,
                _ => EmbeddingProvider::Fastembed,
            })
            .unwrap_or_default();

        let ollama_url = std::env::var("AVOCADO_OLLAMA_URL")
            .unwrap_or_else(|_| "http://localhost:11434".to_string());

        let ollama_model = std::env::var("AVOCADO_OLLAMA_MODEL")
            .unwrap_or_else(|_| "bge-m3".to_string());

        RwLock::new(EmbeddingConfig {
            provider,
            ollama_url,
            ollama_model,
            #[cfg(feature = "openai")]
            openai_api_key: std::env::var("OPENAI_API_KEY").ok(),
        })
    })
}

// ============================================================================
// Fastembed Provider (Default)
// ============================================================================

mod fastembed_provider {
    use super::*;
    use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};

    static FASTEMBED_MODEL: OnceLock<Mutex<TextEmbedding>> = OnceLock::new();

    /// Embedding dimension for all-MiniLM-L6-v2
    pub const DIMENSION: usize = 384;

    fn get_model() -> Result<&'static Mutex<TextEmbedding>> {
        Ok(FASTEMBED_MODEL.get_or_init(|| {
            let model = TextEmbedding::try_new(InitOptions::new(EmbeddingModel::AllMiniLML6V2))
                .expect("Failed to initialize fastembed model");
            Mutex::new(model)
        }))
    }

    pub fn embed_batch(texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        let model_mutex = get_model()?;
        let texts_owned: Vec<String> = texts.iter().map(|s| s.to_string()).collect();

        let guard = model_mutex
            .lock()
            .map_err(|_| AvocadoError::Embedding("Fastembed model lock poisoned".to_string()))?;

        guard
            .embed(texts_owned, None)
            .map_err(|e| AvocadoError::Embedding(format!("Fastembed error: {}", e)))
    }

    pub fn model_name() -> &'static str {
        "all-MiniLM-L6-v2"
    }
}

// ============================================================================
// Ollama Provider
// ============================================================================

pub mod ollama {
    //! Ollama embedding support for local LLM servers
    //!
    //! Ollama provides a local API compatible with various embedding models:
    //! - bge-m3 (1024 dimensions, multilingual)
    //! - nomic-embed-text (768 dimensions)
    //! - mxbai-embed-large (1024 dimensions)
    //! - all-minilm (384 dimensions)

    use super::*;
    use reqwest::blocking::Client;
    use std::time::Duration;

    #[derive(Serialize)]
    struct OllamaEmbedRequest<'a> {
        model: &'a str,
        input: Vec<&'a str>,
    }

    #[derive(Deserialize)]
    struct OllamaEmbedResponse {
        embeddings: Vec<Vec<f32>>,
    }

    // Legacy single-text endpoint response
    #[derive(Deserialize)]
    struct OllamaEmbeddingResponse {
        embedding: Vec<f32>,
    }

    /// Generate embeddings using Ollama API (synchronous/blocking)
    ///
    /// # Arguments
    /// * `texts` - Texts to embed
    /// * `base_url` - Ollama server URL (e.g., "http://localhost:11434")
    /// * `model` - Model name (e.g., "bge-m3", "nomic-embed-text")
    pub fn embed_ollama_sync(texts: &[&str], base_url: &str, model: &str) -> Result<Vec<Vec<f32>>> {
        let client = Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .map_err(|e| AvocadoError::Embedding(format!("HTTP client error: {}", e)))?;

        // Try the batch endpoint first (Ollama 0.4.0+)
        let url = format!("{}/api/embed", base_url);

        let request = OllamaEmbedRequest {
            model,
            input: texts.to_vec(),
        };

        let response = client
            .post(&url)
            .json(&request)
            .send()
            .map_err(|e| AvocadoError::Embedding(format!("Ollama request failed: {}", e)))?;

        if response.status().is_success() {
            let result: OllamaEmbedResponse = response
                .json()
                .map_err(|e| AvocadoError::Embedding(format!("Failed to parse Ollama response: {}", e)))?;
            return Ok(result.embeddings);
        }

        // Fall back to single-text endpoint for older Ollama versions
        let url = format!("{}/api/embeddings", base_url);
        let mut embeddings = Vec::with_capacity(texts.len());

        for text in texts {
            let response = client
                .post(&url)
                .json(&serde_json::json!({
                    "model": model,
                    "prompt": text,
                }))
                .send()
                .map_err(|e| AvocadoError::Embedding(format!("Ollama request failed: {}", e)))?;

            if !response.status().is_success() {
                let status = response.status();
                let body = response.text().unwrap_or_default();
                return Err(AvocadoError::Embedding(format!(
                    "Ollama API error {}: {}",
                    status, body
                )));
            }

            let result: OllamaEmbeddingResponse = response
                .json()
                .map_err(|e| AvocadoError::Embedding(format!("Failed to parse response: {}", e)))?;

            embeddings.push(result.embedding);
        }

        Ok(embeddings)
    }

    /// Get embedding dimension for common Ollama models
    pub fn get_dimension(model: &str) -> usize {
        match model {
            m if m.contains("bge-m3") => 1024,
            m if m.contains("bge-large") => 1024,
            m if m.contains("nomic") => 768,
            m if m.contains("mxbai") => 1024,
            m if m.contains("minilm") || m.contains("all-minilm") => 384,
            m if m.contains("snowflake") => 1024,
            _ => 1024, // Default assumption
        }
    }
}

// ============================================================================
// Public API
// ============================================================================

/// Embedding dimension (depends on configured provider/model)
pub fn embedding_dimension() -> usize {
    let config = get_config().read().unwrap();
    match &config.provider {
        EmbeddingProvider::Fastembed => fastembed_provider::DIMENSION,
        EmbeddingProvider::Ollama => ollama::get_dimension(&config.ollama_model),
        #[cfg(feature = "openai")]
        EmbeddingProvider::OpenAI => 1536, // text-embedding-3-small
    }
}

/// Legacy constant for backward compatibility (fastembed dimension)
pub const EMBEDDING_DIMENSION: usize = 384;

/// Generate embeddings synchronously for a batch of texts
///
/// Uses the configured provider (fastembed by default, or ollama/openai if set).
pub fn embed_batch_sync(texts: &[&str]) -> Result<Vec<Vec<f32>>> {
    let config = get_config().read().unwrap();

    match &config.provider {
        EmbeddingProvider::Fastembed => fastembed_provider::embed_batch(texts),
        EmbeddingProvider::Ollama => {
            ollama::embed_ollama_sync(texts, &config.ollama_url, &config.ollama_model)
        }
        #[cfg(feature = "openai")]
        EmbeddingProvider::OpenAI => {
            let api_key = config
                .openai_api_key
                .as_ref()
                .ok_or_else(|| AvocadoError::Embedding("OPENAI_API_KEY not set".to_string()))?;
            openai::embed_openai_sync(texts, api_key)
        }
    }
}

/// Generate embedding for a single text
pub fn embed_sync(text: &str) -> Result<Vec<f32>> {
    let embeddings = embed_batch_sync(&[text])?;
    embeddings
        .into_iter()
        .next()
        .ok_or_else(|| AvocadoError::Embedding("No embedding returned".to_string()))
}

/// Get the current embedding model name
pub fn model_name() -> String {
    let config = get_config().read().unwrap();
    match &config.provider {
        EmbeddingProvider::Fastembed => fastembed_provider::model_name().to_string(),
        EmbeddingProvider::Ollama => config.ollama_model.clone(),
        #[cfg(feature = "openai")]
        EmbeddingProvider::OpenAI => "text-embedding-3-small".to_string(),
    }
}

/// Get current provider name
pub fn provider_name() -> &'static str {
    let config = get_config().read().unwrap();
    match &config.provider {
        EmbeddingProvider::Fastembed => "fastembed",
        EmbeddingProvider::Ollama => "ollama",
        #[cfg(feature = "openai")]
        EmbeddingProvider::OpenAI => "openai",
    }
}

// ============================================================================
// SQL Functions for Configuration
// ============================================================================

/// Set the embedding provider
///
/// # Example
/// ```sql
/// SELECT avocado_set_embedding_provider('ollama');
/// SELECT avocado_set_embedding_provider('fastembed');
/// ```
#[pg_extern]
fn avocado_set_embedding_provider(provider: &str) -> &'static str {
    let new_provider = match provider.to_lowercase().as_str() {
        "ollama" => EmbeddingProvider::Ollama,
        "fastembed" | "local" | "default" => EmbeddingProvider::Fastembed,
        #[cfg(feature = "openai")]
        "openai" => EmbeddingProvider::OpenAI,
        _ => {
            pgrx::warning!("Unknown provider '{}', using fastembed", provider);
            EmbeddingProvider::Fastembed
        }
    };

    if let Ok(mut config) = get_config().write() {
        config.provider = new_provider;
    }

    provider_name()
}

/// Configure Ollama settings
///
/// # Example
/// ```sql
/// SELECT avocado_set_ollama_config('http://localhost:11434', 'bge-m3');
/// SELECT avocado_set_ollama_config('http://gpu-server:11434', 'nomic-embed-text');
/// ```
#[pg_extern]
fn avocado_set_ollama_config(url: &str, model: &str) -> String {
    if let Ok(mut config) = get_config().write() {
        config.ollama_url = url.to_string();
        config.ollama_model = model.to_string();
    }

    format!("Ollama configured: {} with model {}", url, model)
}

/// Get current embedding configuration
///
/// # Example
/// ```sql
/// SELECT avocado_embedding_config();
/// ```
#[pg_extern]
fn avocado_embedding_config() -> JsonB {
    let config = get_config().read().unwrap();

    JsonB(serde_json::json!({
        "provider": provider_name(),
        "model": model_name(),
        "dimension": embedding_dimension(),
        "ollama_url": config.ollama_url,
        "ollama_model": config.ollama_model,
    }))
}

/// Test embedding generation with current configuration
///
/// # Example
/// ```sql
/// SELECT avocado_test_embedding('Hello world');
/// ```
#[pg_extern]
fn avocado_test_embedding(text: &str) -> JsonB {
    match embed_sync(text) {
        Ok(embedding) => JsonB(serde_json::json!({
            "success": true,
            "provider": provider_name(),
            "model": model_name(),
            "dimension": embedding.len(),
            "sample": &embedding[..5.min(embedding.len())],
        })),
        Err(e) => JsonB(serde_json::json!({
            "success": false,
            "error": e.to_string(),
            "provider": provider_name(),
        })),
    }
}

#[cfg(feature = "openai")]
pub mod openai {
    //! OpenAI embedding support (optional, requires 'openai' feature)

    use crate::error::{AvocadoError, Result};
    use reqwest::blocking::Client;
    use serde::{Deserialize, Serialize};

    const OPENAI_EMBEDDING_URL: &str = "https://api.openai.com/v1/embeddings";

    #[derive(Serialize)]
    struct EmbeddingRequest<'a> {
        model: &'a str,
        input: Vec<&'a str>,
    }

    #[derive(Deserialize)]
    struct EmbeddingResponse {
        data: Vec<EmbeddingData>,
    }

    #[derive(Deserialize)]
    struct EmbeddingData {
        embedding: Vec<f32>,
    }

    /// Generate embeddings using OpenAI API (synchronous/blocking)
    pub fn embed_openai_sync(texts: &[&str], api_key: &str) -> Result<Vec<Vec<f32>>> {
        let client = Client::new();

        let request = EmbeddingRequest {
            model: "text-embedding-3-small",
            input: texts.to_vec(),
        };

        let response = client
            .post(OPENAI_EMBEDDING_URL)
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&request)
            .send()
            .map_err(|e| AvocadoError::Embedding(format!("HTTP request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            return Err(AvocadoError::Embedding(format!(
                "OpenAI API error {}: {}",
                status, body
            )));
        }

        let result: EmbeddingResponse = response
            .json()
            .map_err(|e| AvocadoError::Embedding(format!("Failed to parse response: {}", e)))?;

        Ok(result.data.into_iter().map(|d| d.embedding).collect())
    }
}
