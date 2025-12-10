//! Embedding generation with local and OpenAI support
//!
//! By default, uses local embeddings (all-MiniLM-L6-v2 via ONNX) for self-sufficiency.
//! OpenAI embeddings are optional and can be enabled via OPENAI_API_KEY.

use crate::types::{Error, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;
use std::sync::OnceLock;
use std::sync::{Mutex, Once};
use tokio::io::AsyncWriteExt;
use tokio::process::Command as AsyncCommand;

// OpenAI constants
const OPENAI_API_URL: &str = "https://api.openai.com/v1/embeddings";
const OPENAI_MODEL: &str = "text-embedding-ada-002";
const OPENAI_DIMENSION: usize = 1536;

// Local embedding model configuration
// Default: all-MiniLM-L6-v2 (384 dimensions) - fast and efficient
// Can be overridden via AVOCADODB_EMBEDDING_MODEL environment variable
// Available models and their dimensions:
//   - AllMiniLML6V2: 384 dims (default, fastest)
//   - AllMiniLML12V2: 384 dims (slightly better quality)
//   - BGESmallENV15: 384 dims (good for English)
//   - BGELargeENV15: 1024 dims (higher quality, slower)
//   - NomicEmbedTextV1: 768 dims (good balance)
//   - NomicEmbedTextV15: 768 dims (improved version)
const DEFAULT_LOCAL_MODEL: &str = "sentence-transformers/all-MiniLM-L6-v2";
const DEFAULT_LOCAL_DIMENSION: usize = 384;

/// Get the local embedding model enum based on environment variable
fn get_local_embedding_model() -> fastembed::EmbeddingModel {
    use fastembed::EmbeddingModel;

    if let Ok(model_str) = env::var("AVOCADODB_EMBEDDING_MODEL") {
        match model_str.to_lowercase().as_str() {
            "allminilml6v2" | "all-minilm-l6-v2" | "minilm6" => EmbeddingModel::AllMiniLML6V2,
            "allminilml12v2" | "all-minilm-l12-v2" | "minilm12" => EmbeddingModel::AllMiniLML12V2,
            "bgesmallen" | "bge-small-en-v1.5" | "bgesmall" => EmbeddingModel::BGESmallENV15,
            "bgelargeen" | "bge-large-en-v1.5" | "bgelarge" => EmbeddingModel::BGELargeENV15,
            "nomicv1" | "nomic-embed-text-v1" => EmbeddingModel::NomicEmbedTextV1,
            "nomicv15" | "nomic-embed-text-v1.5" | "nomic" => EmbeddingModel::NomicEmbedTextV15,
            _ => {
                log::warn!(
                    "Unknown embedding model '{}', using default AllMiniLML6V2",
                    model_str
                );
                EmbeddingModel::AllMiniLML6V2
            }
        }
    } else {
        EmbeddingModel::AllMiniLML6V2
    }
}

/// Get the dimension for the selected local embedding model
fn get_local_embedding_dimension() -> usize {
    use fastembed::EmbeddingModel;

    match get_local_embedding_model() {
        EmbeddingModel::AllMiniLML6V2 => 384,
        EmbeddingModel::AllMiniLML12V2 => 384,
        EmbeddingModel::BGESmallENV15 => 384,
        EmbeddingModel::BGELargeENV15 => 1024,
        EmbeddingModel::NomicEmbedTextV1 => 768,
        EmbeddingModel::NomicEmbedTextV15 => 768,
        _ => DEFAULT_LOCAL_DIMENSION, // Fallback
    }
}

/// Get the model name string for the selected local embedding model
fn get_local_model_name() -> &'static str {
    use fastembed::EmbeddingModel;

    match get_local_embedding_model() {
        EmbeddingModel::AllMiniLML6V2 => "sentence-transformers/all-MiniLM-L6-v2",
        EmbeddingModel::AllMiniLML12V2 => "sentence-transformers/all-MiniLM-L12-v2",
        EmbeddingModel::BGESmallENV15 => "BAAI/bge-small-en-v1.5",
        EmbeddingModel::BGELargeENV15 => "BAAI/bge-large-en-v1.5",
        EmbeddingModel::NomicEmbedTextV1 => "nomic-ai/nomic-embed-text-v1",
        EmbeddingModel::NomicEmbedTextV15 => "nomic-ai/nomic-embed-text-v1.5",
        _ => DEFAULT_LOCAL_MODEL, // Fallback
    }
}

/// Embedding provider type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmbeddingProvider {
    /// Local embeddings using candle (default, no API required)
    Local,
    /// OpenAI embeddings (requires OPENAI_API_KEY)
    OpenAI,
    /// Remote HTTP embeddings (GPU sandbox or custom service)
    Remote,
    /// Ollama local server (e.g., bge-m3, nomic-embed-text)
    Ollama,
}

impl Default for EmbeddingProvider {
    fn default() -> Self {
        // Default to local for self-sufficiency
        EmbeddingProvider::Local
    }
}

impl EmbeddingProvider {
    /// Detect provider from environment or use default
    pub fn from_env() -> Self {
        // If OPENAI_API_KEY is set, allow OpenAI as option
        // But default to local for self-sufficiency
        if env::var("AVOCADODB_EMBEDDING_PROVIDER").is_ok() {
            match env::var("AVOCADODB_EMBEDDING_PROVIDER")
                .unwrap()
                .to_lowercase()
                .as_str()
            {
                "openai" => EmbeddingProvider::OpenAI,
                "local" | "fastembed" => EmbeddingProvider::Local,
                "remote" => EmbeddingProvider::Remote,
                "ollama" => EmbeddingProvider::Ollama,
                _ => EmbeddingProvider::Local,
            }
        } else {
            EmbeddingProvider::Local
        }
    }

    /// Get the embedding dimension for this provider
    pub fn dimension(&self) -> usize {
        match self {
            EmbeddingProvider::Local => get_local_embedding_dimension(),
            EmbeddingProvider::OpenAI => OPENAI_DIMENSION,
            EmbeddingProvider::Ollama => get_ollama_embedding_dimension(),
            EmbeddingProvider::Remote => {
                // Allow overriding remote dimension via env; default to local dimension for compatibility
                env::var("AVOCADODB_EMBEDDING_DIM")
                    .ok()
                    .and_then(|s| s.parse::<usize>().ok())
                    .unwrap_or_else(get_local_embedding_dimension)
            }
        }
    }

    /// Get the model name for this provider
    pub fn model_name(&self) -> &'static str {
        match self {
            EmbeddingProvider::Local => get_local_model_name(),
            EmbeddingProvider::OpenAI => OPENAI_MODEL,
            EmbeddingProvider::Ollama => get_ollama_model_name(),
            // Remote model name is not fixed; callers can optionally set AVOCADODB_EMBEDDING_MODEL
            EmbeddingProvider::Remote => DEFAULT_LOCAL_MODEL,
        }
    }
}

// Ollama configuration
const DEFAULT_OLLAMA_URL: &str = "http://localhost:11434";
const DEFAULT_OLLAMA_MODEL: &str = "bge-m3";

/// Get the Ollama model from environment
fn get_ollama_model_name() -> &'static str {
    // Note: We leak the string to get a 'static lifetime, which is fine since
    // this is only called when using Ollama and the string is cached
    static OLLAMA_MODEL: OnceLock<String> = OnceLock::new();
    let model = OLLAMA_MODEL.get_or_init(|| {
        env::var("AVOCADODB_OLLAMA_MODEL").unwrap_or_else(|_| DEFAULT_OLLAMA_MODEL.to_string())
    });
    // Safe: OnceLock guarantees this string lives for the program's lifetime
    unsafe { std::mem::transmute::<&str, &'static str>(model.as_str()) }
}

/// Get the Ollama embedding dimension based on model name
fn get_ollama_embedding_dimension() -> usize {
    let model = get_ollama_model_name();
    match model {
        m if m.contains("bge-m3") => 1024,
        m if m.contains("bge-large") => 1024,
        m if m.contains("nomic") => 768,
        m if m.contains("mxbai") => 1024,
        m if m.contains("minilm") || m.contains("all-minilm") => 384,
        m if m.contains("snowflake") => 1024,
        _ => {
            // Allow explicit dimension override
            env::var("AVOCADODB_EMBEDDING_DIM")
                .ok()
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(1024) // Default to 1024 for unknown models
        }
    }
}

// OpenAI API request/response types
#[derive(Debug, Serialize)]
struct EmbeddingRequest {
    model: String,
    input: Vec<String>,
}

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
/// Uses local embeddings by default (no API required).
/// Set AVOCADODB_EMBEDDING_PROVIDER=openai to use OpenAI.
///
/// # Arguments
///
/// * `text` - The text to embed
/// * `provider` - Embedding provider (defaults to Local)
/// * `api_key` - OpenAI API key (only used if provider is OpenAI)
///
/// # Returns
///
/// A vector of floats representing the embedding (384 for local, 1536 for OpenAI)
pub async fn embed_text(
    text: &str,
    provider: Option<EmbeddingProvider>,
    api_key: Option<&str>,
) -> Result<Vec<f32>> {
    let results = embed_batch(vec![text], provider, api_key).await?;
    results
        .into_iter()
        .next()
        .ok_or_else(|| Error::Embedding("No embedding returned".to_string()))
}

/// Embed multiple text strings
///
/// Uses local embeddings by default (no API required).
/// Set AVOCADODB_EMBEDDING_PROVIDER=openai to use OpenAI.
///
/// # Arguments
///
/// * `texts` - Vector of text strings to embed
/// * `provider` - Embedding provider (defaults to Local)
/// * `api_key` - OpenAI API key (only used if provider is OpenAI)
///
/// # Returns
///
/// A vector of embeddings, in the same order as the input texts
pub async fn embed_batch(
    texts: Vec<&str>,
    provider: Option<EmbeddingProvider>,
    api_key: Option<&str>,
) -> Result<Vec<Vec<f32>>> {
    let provider = provider.unwrap_or_else(EmbeddingProvider::from_env);

    if texts.is_empty() {
        return Ok(vec![]);
    }

    match provider {
        EmbeddingProvider::Local => embed_batch_local(texts).await,
        EmbeddingProvider::OpenAI => embed_batch_openai(texts, api_key).await,
        EmbeddingProvider::Remote => embed_batch_remote(texts).await,
        EmbeddingProvider::Ollama => embed_batch_ollama(texts).await,
    }
}

/// Local embedding generation with multiple strategies
///
/// Pure Rust implementation using fastembed (ONNX-based) for semantic embeddings.
/// Falls back to Python subprocess, then hash-based if needed.
///
/// Strategy priority:
/// 1. Pure Rust with fastembed (semantic, good quality, no Python required)
///    - Uses all-MiniLM-L6-v2 model (384 dimensions)
///    - ONNX-based, fast and efficient
///    - Model cached after first download (~90MB)
/// 2. Python subprocess with sentence-transformers (fallback if fastembed fails)
///    - Requires: `pip install sentence-transformers`
/// 3. Hash-based fallback (deterministic, but NOT semantic)
///    - Works without dependencies
///    - Poor semantic quality (similar texts don't cluster)
async fn embed_batch_local(texts: Vec<&str>) -> Result<Vec<Vec<f32>>> {
    // Try pure Rust fastembed first (best performance, no Python required)
    if let Ok(embeddings) = embed_batch_local_rust(texts.clone()).await {
        return Ok(embeddings);
    }

    // Respect hard-fail configuration to forbid non-semantic fallbacks in production
    if matches!(
        std::env::var("AVOCADODB_FORBID_FALLBACKS").ok().as_deref(),
        Some("1" | "true" | "TRUE" | "yes" | "YES")
    ) {
        return Err(Error::Embedding(
            "Local fastembed failed and fallbacks are disabled (AVOCADODB_FORBID_FALLBACKS=1)"
                .to_string(),
        ));
    }

    // Fallback to Python sentence-transformers (if available)
    static PY_WARN_ONCE: Once = Once::new();
    PY_WARN_ONCE.call_once(|| {
        log::warn!("Falling back to Python sentence-transformers for embeddings. Install Rust fastembed for best performance.");
    });
    if let Ok(embeddings) = embed_batch_local_python(texts.clone()).await {
        return Ok(embeddings);
    }

    // Final fallback to hash-based embeddings (works always, but not semantic)
    static HASH_WARN_ONCE: Once = Once::new();
    HASH_WARN_ONCE.call_once(|| {
        log::error!("Falling back to HASH-BASED embeddings (NOT SEMANTIC). This mode is for emergencies only.");
    });
    embed_batch_local_hash(texts).await
}

/// Pure Rust embeddings using fastembed (ONNX-based, no Python required)
///
/// Uses fastembed crate with all-MiniLM-L6-v2 model for semantic embeddings.
/// This is the preferred method as it's pure Rust, fast, and doesn't require Python.
///
/// Model is downloaded from HuggingFace on first use and cached locally.
/// fastembed handles model caching internally, so initialization is fast after first use.
async fn embed_batch_local_rust(texts: Vec<&str>) -> Result<Vec<Vec<f32>>> {
    use fastembed::{InitOptions, TextEmbedding};
    use tokio::task;

    if texts.is_empty() {
        return Ok(vec![]);
    }

    // Convert &str to String for the blocking task
    let texts_owned: Vec<String> = texts.iter().map(|s| s.to_string()).collect();

    // Cache the TextEmbedding model instance across the process to avoid repeated initialization
    static FASTEMBED_MODEL: OnceLock<Mutex<TextEmbedding>> = OnceLock::new();

    // fastembed is synchronous, so we run it in a blocking task
    // Note: fastembed handles model caching internally, so initialization is fast
    let embeddings = task::spawn_blocking(move || -> Result<Vec<Vec<f32>>> {
        // Initialize or reuse cached model (downloads on first use, then caches)
        let model_mutex = FASTEMBED_MODEL.get_or_init(|| {
            let embedding_model = get_local_embedding_model();
            let model = TextEmbedding::try_new(
                InitOptions::new(embedding_model).with_show_download_progress(false),
            )
            .expect("Failed to initialize fastembed model");
            Mutex::new(model)
        });

        // Generate embeddings (fastembed handles normalization)
        let embeddings = model_mutex
            .lock()
            .map_err(|_| Error::Embedding("Failed to lock fastembed model".to_string()))?
            .embed(texts_owned, None)
            .map_err(|e| Error::Embedding(format!("Failed to generate embeddings: {}", e)))?;

        // Verify dimensions (get expected dimension for selected model)
        let expected_dim = get_local_embedding_dimension();
        for emb in &embeddings {
            if emb.len() != expected_dim {
                return Err(Error::Embedding(format!(
                    "Unexpected embedding dimension: {} (expected {})",
                    emb.len(),
                    expected_dim
                )));
            }
        }

        Ok(embeddings)
    })
    .await
    .map_err(|e| Error::Embedding(format!("Task join error: {}", e)))??;

    Ok(embeddings)
}

/// Local embeddings using Python sentence-transformers (semantic, good quality)
///
/// Uses a Python subprocess to call sentence-transformers with all-MiniLM-L6-v2.
/// This provides semantic embeddings without API keys.
///
/// Requires: Python with sentence-transformers installed
///   pip install sentence-transformers
async fn embed_batch_local_python(texts: Vec<&str>) -> Result<Vec<Vec<f32>>> {
    // Check if Python is available
    let python = which_python()?;

    // Create a Python script to generate embeddings
    let script = format!(
        r#"
import sys
import json

try:
    from sentence_transformers import SentenceTransformer
    import numpy as np
    
    # Load model (cached after first use)
    model = SentenceTransformer('all-MiniLM-L6-v2')
    
    # Read texts from stdin (one per line)
    texts = []
    for line in sys.stdin:
        texts.append(line.strip())
    
    # Generate embeddings
    embeddings = model.encode(texts, normalize_embeddings=True)
    
    # Output as JSON array
    result = [emb.tolist() for emb in embeddings]
    print(json.dumps(result))
    sys.exit(0)
except ImportError:
    print(json.dumps({{"error": "sentence-transformers not installed. Install with: pip install sentence-transformers"}}), file=sys.stderr)
    sys.exit(1)
except Exception as e:
    print(json.dumps({{"error": str(e)}}), file=sys.stderr)
    sys.exit(1)
"#
    );

    // Run Python script
    let mut child = AsyncCommand::new(&python)
        .arg("-c")
        .arg(&script)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| Error::Embedding(format!("Failed to spawn Python process: {}", e)))?;

    // Write texts to stdin
    if let Some(mut stdin) = child.stdin.take() {
        for text in &texts {
            stdin
                .write_all(text.as_bytes())
                .await
                .map_err(|e| Error::Embedding(format!("Failed to write to Python stdin: {}", e)))?;
            stdin
                .write_all(b"\n")
                .await
                .map_err(|e| Error::Embedding(format!("Failed to write to Python stdin: {}", e)))?;
        }
        stdin
            .shutdown()
            .await
            .map_err(|e| Error::Embedding(format!("Failed to close Python stdin: {}", e)))?;
    }

    // Wait for output
    let output = child
        .wait_with_output()
        .await
        .map_err(|e| Error::Embedding(format!("Failed to wait for Python process: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::Embedding(format!(
            "Python embedding failed: {}",
            stderr
        )));
    }

    // Parse JSON output
    let stdout = String::from_utf8_lossy(&output.stdout);
    let embeddings: Vec<Vec<f32>> = serde_json::from_str(&stdout)
        .map_err(|e| Error::Embedding(format!("Failed to parse Python output: {}", e)))?;

    // Verify dimensions (use default for Python fallback)
    let expected_dim = get_local_embedding_dimension();
    for emb in &embeddings {
        if emb.len() != expected_dim {
            return Err(Error::Embedding(format!(
                "Unexpected embedding dimension: {} (expected {})",
                emb.len(),
                expected_dim
            )));
        }
    }

    if embeddings.len() != texts.len() {
        return Err(Error::Embedding(format!(
            "Mismatched embedding count: {} embeddings for {} texts",
            embeddings.len(),
            texts.len()
        )));
    }

    Ok(embeddings)
}

/// Find Python executable (python3 or python)
fn which_python() -> Result<String> {
    // Try python3 first, then python
    for cmd in &["python3", "python"] {
        if std::process::Command::new(cmd)
            .arg("--version")
            .output()
            .is_ok()
        {
            return Ok(cmd.to_string());
        }
    }
    Err(Error::Embedding(
        "Python not found. Install Python 3 to use local embeddings.".to_string(),
    ))
}

/// Hash-based embeddings (fallback, NOT semantic)
///
/// Deterministic but not semantic - similar texts won't have similar embeddings.
/// Used as fallback when Python/sentence-transformers is unavailable.
async fn embed_batch_local_hash(texts: Vec<&str>) -> Result<Vec<Vec<f32>>> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let embeddings: Vec<Vec<f32>> = texts
        .iter()
        .map(|text| {
            let mut hasher = DefaultHasher::new();
            text.hash(&mut hasher);
            let hash = hasher.finish();

            let dim = get_local_embedding_dimension();
            let mut embedding = vec![0.0f32; dim];
            for i in 0..dim {
                let seed = hash.wrapping_add(i as u64);
                embedding[i] = ((seed % 2000) as f32 - 1000.0) / 1000.0;
            }

            let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
            if norm > 0.0 {
                for x in &mut embedding {
                    *x /= norm;
                }
            }

            embedding
        })
        .collect();

    Ok(embeddings)
}

/// OpenAI embedding generation
async fn embed_batch_openai(texts: Vec<&str>, api_key: Option<&str>) -> Result<Vec<Vec<f32>>> {
    let api_key = api_key
        .map(|s| s.to_string())
        .or_else(|| env::var("OPENAI_API_KEY").ok())
        .ok_or_else(|| {
            Error::Embedding(
                "OPENAI_API_KEY environment variable not set and no API key provided".to_string(),
            )
        })?;

    // OpenAI limit is 2048 inputs per request
    if texts.len() > 2048 {
        return Err(Error::InvalidInput(format!(
            "Too many texts to embed at once: {} (max 2048)",
            texts.len()
        )));
    }

    let client = Client::new();

    let request = EmbeddingRequest {
        model: OPENAI_MODEL.to_string(),
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
        if emb.len() != OPENAI_DIMENSION {
            return Err(Error::Embedding(format!(
                "Unexpected embedding dimension: {} (expected {})",
                emb.len(),
                OPENAI_DIMENSION
            )));
        }
    }

    Ok(embeddings)
}

/// Remote HTTP embedding generation
///
/// The remote service is configured via:
/// - AVOCADODB_EMBEDDING_URL: required, e.g. https://your-modal-fn.modal.run/embed
/// - AVOCADODB_EMBEDDING_API_KEY: optional, sent as Bearer token
/// - AVOCADODB_EMBEDDING_MODEL: optional, forwarded to remote
/// - AVOCADODB_EMBEDDING_DIM: optional, expected dimension (defaults to local dim)
///
/// Expected request body:
///   { "inputs": ["text1", "text2"], "model": "BAAI/bge-small-en-v1.5" }
///
/// Expected response body (either of the following):
///   { "embeddings": [[..],[..]], "dimension": 384 }
///   [[..],[..]]
async fn embed_batch_remote(texts: Vec<&str>) -> Result<Vec<Vec<f32>>> {
    use serde_json::json;

    let url = env::var("AVOCADODB_EMBEDDING_URL").map_err(|_| {
        Error::Embedding("AVOCADODB_EMBEDDING_URL not set for remote provider".to_string())
    })?;
    if texts.is_empty() {
        return Ok(vec![]);
    }

    let client = Client::new();
    let mut req = client.post(&url).header("Content-Type", "application/json");

    if let Ok(api_key) = env::var("AVOCADODB_EMBEDDING_API_KEY") {
        if !api_key.is_empty() {
            req = req.header("Authorization", format!("Bearer {}", api_key));
        }
    }

    let model = env::var("AVOCADODB_EMBEDDING_MODEL").ok();
    let body = if let Some(model_name) = model {
        json!({ "inputs": texts, "model": model_name })
    } else {
        json!({ "inputs": texts })
    };

    let resp = req
        .json(&body)
        .send()
        .await
        .map_err(|e| Error::Embedding(format!("Remote request failed: {}", e)))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(Error::Embedding(format!(
            "Remote returned error {}: {}",
            status, text
        )));
    }

    // Try to parse as { embeddings: [...], dimension?: N }
    let expected_dim = EmbeddingProvider::Remote.dimension();
    let text_body = resp
        .text()
        .await
        .map_err(|e| Error::Embedding(format!("Failed reading remote body: {}", e)))?;

    // First attempt: object with embeddings
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text_body) {
        if let Some(arr) = v.get("embeddings").and_then(|x| x.as_array()) {
            let mut embeddings: Vec<Vec<f32>> = Vec::with_capacity(arr.len());
            for item in arr {
                let vec_opt = item.as_array().map(|nums| {
                    nums.iter()
                        .filter_map(|n| n.as_f64().map(|f| f as f32))
                        .collect::<Vec<f32>>()
                });
                let vec = vec_opt.ok_or_else(|| {
                    Error::Embedding("Invalid embeddings array format".to_string())
                })?;
                if !vec.is_empty() && vec.len() != expected_dim {
                    // Allow remote to communicate dimension if provided
                    if let Some(dim) = v
                        .get("dimension")
                        .and_then(|d| d.as_u64())
                        .map(|d| d as usize)
                    {
                        if vec.len() != dim {
                            return Err(Error::Embedding(format!(
                                "Unexpected embedding dimension: {} (expected {})",
                                vec.len(),
                                expected_dim
                            )));
                        }
                    } else {
                        return Err(Error::Embedding(format!(
                            "Unexpected embedding dimension: {} (expected {})",
                            vec.len(),
                            expected_dim
                        )));
                    }
                }
                embeddings.push(vec);
            }
            if embeddings.len() != texts.len() {
                return Err(Error::Embedding(format!(
                    "Mismatched embedding count: got {}, expected {}",
                    embeddings.len(),
                    texts.len()
                )));
            }
            return Ok(embeddings);
        }

        // Second attempt: top-level array
        if let Some(arr) = v.as_array() {
            let mut embeddings: Vec<Vec<f32>> = Vec::with_capacity(arr.len());
            for item in arr {
                let vec_opt = item.as_array().map(|nums| {
                    nums.iter()
                        .filter_map(|n| n.as_f64().map(|f| f as f32))
                        .collect::<Vec<f32>>()
                });
                let vec = vec_opt.ok_or_else(|| {
                    Error::Embedding("Invalid embeddings array format".to_string())
                })?;
                if !vec.is_empty() && vec.len() != expected_dim {
                    return Err(Error::Embedding(format!(
                        "Unexpected embedding dimension: {} (expected {})",
                        vec.len(),
                        expected_dim
                    )));
                }
                embeddings.push(vec);
            }
            if embeddings.len() != texts.len() {
                return Err(Error::Embedding(format!(
                    "Mismatched embedding count: got {}, expected {}",
                    embeddings.len(),
                    texts.len()
                )));
            }
            return Ok(embeddings);
        }
    }

    Err(Error::Embedding(
        "Failed to parse remote embedding response".to_string(),
    ))
}

/// Ollama embedding generation
///
/// Uses local Ollama server with configurable model.
/// Configure via:
/// - AVOCADODB_OLLAMA_URL: Ollama server URL (default: http://localhost:11434)
/// - AVOCADODB_OLLAMA_MODEL: Model name (default: bge-m3)
///
/// Supports models like:
/// - bge-m3 (1024 dimensions, multilingual)
/// - nomic-embed-text (768 dimensions)
/// - mxbai-embed-large (1024 dimensions)
/// - all-minilm (384 dimensions)
async fn embed_batch_ollama(texts: Vec<&str>) -> Result<Vec<Vec<f32>>> {
    use serde_json::json;

    let base_url =
        env::var("AVOCADODB_OLLAMA_URL").unwrap_or_else(|_| DEFAULT_OLLAMA_URL.to_string());
    let model = get_ollama_model_name();
    let expected_dim = get_ollama_embedding_dimension();

    if texts.is_empty() {
        return Ok(vec![]);
    }

    let client = Client::new();

    // Try batch endpoint first (Ollama 0.4.0+)
    let url = format!("{}/api/embed", base_url);
    let body = json!({
        "model": model,
        "input": texts,
    });

    let resp = client
        .post(&url)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| Error::Embedding(format!("Ollama request failed: {}", e)))?;

    if resp.status().is_success() {
        let text_body = resp
            .text()
            .await
            .map_err(|e| Error::Embedding(format!("Failed reading Ollama response: {}", e)))?;

        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text_body) {
            if let Some(arr) = v.get("embeddings").and_then(|x| x.as_array()) {
                let mut embeddings: Vec<Vec<f32>> = Vec::with_capacity(arr.len());
                for item in arr {
                    let vec: Vec<f32> = item
                        .as_array()
                        .map(|nums| {
                            nums.iter()
                                .filter_map(|n| n.as_f64().map(|f| f as f32))
                                .collect()
                        })
                        .ok_or_else(|| Error::Embedding("Invalid embedding array".to_string()))?;
                    embeddings.push(vec);
                }
                if embeddings.len() != texts.len() {
                    return Err(Error::Embedding(format!(
                        "Mismatched embedding count: got {}, expected {}",
                        embeddings.len(),
                        texts.len()
                    )));
                }
                return Ok(embeddings);
            }
        }
    }

    // Fall back to single-text endpoint for older Ollama versions
    let url = format!("{}/api/embeddings", base_url);
    let mut embeddings = Vec::with_capacity(texts.len());

    for text in texts {
        let body = json!({
            "model": model,
            "prompt": text,
        });

        let resp = client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| Error::Embedding(format!("Ollama request failed: {}", e)))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(Error::Embedding(format!(
                "Ollama API error {}: {}",
                status, body
            )));
        }

        let text_body = resp
            .text()
            .await
            .map_err(|e| Error::Embedding(format!("Failed reading Ollama response: {}", e)))?;

        let v: serde_json::Value = serde_json::from_str(&text_body)
            .map_err(|e| Error::Embedding(format!("Failed parsing Ollama response: {}", e)))?;

        let embedding: Vec<f32> = v
            .get("embedding")
            .and_then(|e| e.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|n| n.as_f64().map(|f| f as f32))
                    .collect()
            })
            .ok_or_else(|| Error::Embedding("No embedding in Ollama response".to_string()))?;

        if embedding.len() != expected_dim {
            return Err(Error::Embedding(format!(
                "Unexpected embedding dimension: {} (expected {})",
                embedding.len(),
                expected_dim
            )));
        }

        embeddings.push(embedding);
    }

    Ok(embeddings)
}

/// Get the embedding model name (based on current provider)
pub fn embedding_model() -> &'static str {
    EmbeddingProvider::from_env().model_name()
}

/// Get the embedding dimension (based on current provider)
pub fn embedding_dimension() -> usize {
    EmbeddingProvider::from_env().dimension()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedding_provider_default() {
        // Default should be local
        let provider = EmbeddingProvider::default();
        assert_eq!(provider, EmbeddingProvider::Local);
        assert_eq!(provider.dimension(), get_local_embedding_dimension());
    }

    #[test]
    fn test_embedding_dimensions() {
        // Default model is AllMiniLML6V2 with 384 dimensions
        assert_eq!(
            EmbeddingProvider::Local.dimension(),
            get_local_embedding_dimension()
        );
        assert_eq!(EmbeddingProvider::OpenAI.dimension(), 1536);
    }

    #[tokio::test]
    async fn test_embed_batch_local() {
        // Test local embeddings (should work without API key)
        let texts = vec!["Hello", "World", "Test"];
        let result = embed_batch_local(texts).await;

        assert!(result.is_ok());
        let embeddings = result.unwrap();
        assert_eq!(embeddings.len(), 3);
        for emb in embeddings {
            assert_eq!(emb.len(), get_local_embedding_dimension());
        }
    }

    #[tokio::test]
    #[ignore] // Only run when OPENAI_API_KEY is set
    async fn test_embed_text_openai() {
        let result = embed_text("Hello, world!", Some(EmbeddingProvider::OpenAI), None).await;
        if env::var("OPENAI_API_KEY").is_ok() {
            let embedding = result.unwrap();
            assert_eq!(embedding.len(), OPENAI_DIMENSION);
        } else {
            assert!(result.is_err());
        }
    }
}
