//! AvocadoDB Command Line Interface
//!
//! Simple CLI for interacting with AvocadoDB locally.

use anyhow::Result;
use avocado_core::{compiler, db::Database, embedding, index::VectorIndex, span, Artifact, CompilerConfig};
use clap::{Parser, Subcommand};
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Parser)]
#[command(name = "avocado")]
#[command(about = "AvocadoDB - Deterministic context compilation", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new AvocadoDB database
    Init {
        /// Database path (default: .avocado/db.sqlite)
        #[arg(short, long, default_value = ".avocado/db.sqlite")]
        path: PathBuf,
    },

    /// Ingest documents into the database
    Ingest {
        /// Path to file or directory
        path: PathBuf,

        /// Recursively ingest directories
        #[arg(short, long)]
        recursive: bool,

        /// Database path
        #[arg(short, long, default_value = ".avocado/db.sqlite")]
        db_path: PathBuf,
    },

    /// Compile a context working set for a query
    Compile {
        /// Search query
        query: String,

        /// Token budget
        #[arg(short, long, default_value = "8000")]
        budget: usize,

        /// Output as JSON
        #[arg(short, long)]
        json: bool,

        /// Database path
        #[arg(short, long, default_value = ".avocado/db.sqlite")]
        db_path: PathBuf,
    },

    /// Show database statistics
    Stats {
        /// Database path
        #[arg(short, long, default_value = ".avocado/db.sqlite")]
        db_path: PathBuf,
    },

    /// Clear all data
    Clear {
        /// Database path
        #[arg(short, long, default_value = ".avocado/db.sqlite")]
        db_path: PathBuf,

        /// Skip confirmation
        #[arg(short, long)]
        yes: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Init { path } => {
            // Create directory if it doesn't exist
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }

            // Initialize database
            let _db = Database::new(&path)?;
            println!("✓ Initialized AvocadoDB at {}", path.display());
        }

        Commands::Ingest {
            path,
            recursive,
            db_path,
        } => {
            let db = Database::new(&db_path)?;

            let files = if path.is_dir() {
                collect_files(&path, recursive)?
            } else {
                vec![path]
            };

            let mut total_spans = 0;
            for file_path in &files {
                let content = fs::read_to_string(file_path)?;

                // Create artifact
                let artifact_id = Uuid::new_v4().to_string();
                let content_hash = format!("{:x}", sha2::Sha256::digest(content.as_bytes()));

                let artifact = Artifact {
                    id: artifact_id.clone(),
                    path: file_path.display().to_string(),
                    content: content.clone(),
                    content_hash,
                    metadata: None,
                    created_at: chrono::Utc::now(),
                };

                db.insert_artifact(&artifact)?;

                // Extract and embed spans
                let mut spans = span::extract_spans(&content, &artifact_id)?;

                // Embed spans
                println!("Embedding {} spans from {}...", spans.len(), file_path.display());
                let texts: Vec<&str> = spans.iter().map(|s| s.text.as_str()).collect();
                let embeddings = embedding::embed_batch(texts, None).await?;

                for (span, emb) in spans.iter_mut().zip(embeddings.iter()) {
                    span.embedding = Some(emb.clone());
                    span.embedding_model = Some(embedding::embedding_model().to_string());
                }

                db.insert_spans(&spans)?;
                total_spans += spans.len();
            }

            println!(
                "✓ Indexed {} files, created {} spans",
                files.len(),
                total_spans
            );
        }

        Commands::Compile {
            query,
            budget,
            json,
            db_path,
        } => {
            let db = Database::new(&db_path)?;

            // Load all spans and build index
            let spans = db.get_all_spans()?;
            let index = VectorIndex::build(spans);

            // Compile context
            let config = CompilerConfig {
                token_budget: budget,
                ..Default::default()
            };

            let working_set = compiler::compile(&query, config, &db, &index, None).await?;

            if json {
                println!("{}", serde_json::to_string_pretty(&working_set)?);
            } else {
                println!("{}", working_set.text);
                println!("\n---");
                println!(
                    "Tokens: {}/{} | Time: {}ms | Citations: {}",
                    working_set.tokens_used,
                    budget,
                    working_set.compilation_time_ms,
                    working_set.citations.len()
                );
            }
        }

        Commands::Stats { db_path } => {
            let db = Database::new(&db_path)?;
            let (artifacts, spans, tokens) = db.get_stats()?;

            println!("AvocadoDB Statistics");
            println!("  Artifacts: {}", artifacts);
            println!("  Spans: {}", spans);
            println!("  Total tokens: {}", tokens);
        }

        Commands::Clear { db_path, yes } => {
            if !yes {
                print!("Are you sure? This will delete all data. (y/N): ");
                use std::io::{self, Write};
                io::stdout().flush()?;

                let mut input = String::new();
                io::stdin().read_line(&mut input)?;

                if !input.trim().eq_ignore_ascii_case("y") {
                    println!("Cancelled");
                    return Ok(());
                }
            }

            let db = Database::new(&db_path)?;
            db.clear()?;
            println!("✓ Cleared all data");
        }
    }

    Ok(())
}

/// Collect files from a directory
fn collect_files(dir: &PathBuf, recursive: bool) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            files.push(path);
        } else if path.is_dir() && recursive {
            files.extend(collect_files(&path, recursive)?);
        }
    }

    Ok(files)
}
