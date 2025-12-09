//! AvocadoDB Command Line Interface
//!
//! Simple CLI for interacting with AvocadoDB locally.

mod commands;

use anyhow::Result;
use avocado_core::{compiler, db::Database, embedding, span, Artifact, CompilerConfig};
use clap::{Parser, Subcommand};
use console::style;
use indicatif::{ProgressBar, ProgressStyle, MultiProgress};
use sha2::Digest;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
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

        /// Backend identifier to use (e.g., bge-large-1024, openai-1536). Defaults to server active.
        #[arg(long)]
        backend: Option<String>,

        /// AvocadoDB server URL (daemon mode). Use --local to force local DB mode.
        #[arg(long, default_value = "http://localhost:8765")]
        url: String,

        /// Use local database instead of daemon
        #[arg(long, default_value_t = false)]
        local: bool,

        /// Token budget
        #[arg(short, long, default_value = "8000")]
        budget: usize,

        /// Output as JSON
        #[arg(short, long)]
        json: bool,

        /// Database path (local mode only)
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

    /// Ask a question and get a natural language answer (v2.0)
    Ask {
        /// The question to ask
        query: String,

        /// LLM mode: auto (try local, fallback), local (require), none (just context)
        #[arg(long, default_value = "auto")]
        llm: String,

        /// Token budget for context compilation
        #[arg(short, long, default_value = "8000")]
        budget: usize,

        /// Maximum tokens for answer generation
        #[arg(long, default_value = "150")]
        max_tokens: usize,

        /// AvocadoDB server URL
        #[arg(long, default_value = "http://localhost:8765")]
        url: String,

        /// Output as JSON
        #[arg(short, long)]
        json: bool,
    },

    /// Run performance benchmarks
    Benchmark {
        /// Show detailed statistics
        #[arg(short, long)]
        verbose: bool,
    },

    /// Recommend optimal embedding model
    Recommend {
        /// Corpus size (number of documents)
        #[arg(long)]
        corpus_size: Option<usize>,

        /// Use case (e.g., "production", "legal", "code-search")
        #[arg(long)]
        use_case: Option<String>,
    },

    /// Start the AvocadoDB daemon with either GPU (remote) or CPU (local) embeddings
    Serve {
        /// Use GPU (remote) embeddings via an HTTP endpoint (e.g., Modal). If false, use local CPU.
        #[arg(long, default_value_t = false)]
        gpu: bool,

        /// Remote embedding endpoint URL (required for --gpu), e.g. https://.../embed
        #[arg(long)]
        embed_url: Option<String>,

        /// Embedding model name for remote mode
        #[arg(long, default_value = "BAAI/bge-large-en-v1.5")]
        model: String,

        /// Embedding vector dimension for remote mode
        #[arg(long, default_value_t = 1024)]
        dim: usize,

        /// Avocado server URL to wait on (health check)
        #[arg(long, default_value = "http://127.0.0.1:8765")]
        url: String,

        /// Prewarm embeddings (make one small call) to reduce first-call latency
        #[arg(long, default_value_t = true)]
        prewarm: bool,
    },

    /// Session management commands
    Session {
        #[command(subcommand)]
        command: commands::SessionCommands,
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

            println!(
                "{} Ingesting {} {}...\n",
                style("🥑").green(),
                files.len(),
                if files.len() == 1 { "file" } else { "files" }
            );

            // Create multi-progress for overall + individual file progress
            let multi = MultiProgress::new();

            let overall_pb = multi.add(ProgressBar::new(files.len() as u64));
            overall_pb.set_style(
                ProgressStyle::default_bar()
                    .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} files ({msg})")
                    .unwrap()
                    .progress_chars("#>-")
            );
            overall_pb.set_message("starting");

            let mut total_spans = 0;
            let mut successful = 0;
            let mut failed = 0;

            for (_idx, file_path) in files.iter().enumerate() {
                // Wrap each file in error handling so we continue even if one fails
                let file_result: Result<usize, anyhow::Error> = async {
                    // Try to read file as text, skip if it fails (binary files, etc.)
                    let content = match fs::read_to_string(file_path) {
                        Ok(c) => c,
                        Err(_e) => {
                            // Skip binary files or unreadable files
                            return Ok(0);
                        }
                    };

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

                    // Try to insert artifact (might fail if duplicate)
                    if let Err(_e) = db.insert_artifact(&artifact) {
                        // Skip if already exists or other error
                        return Ok(0);
                    }

                    // Extract spans
                    let mut spans = match span::extract_spans(&content, &artifact_id) {
                        Ok(s) => s,
                        Err(_e) => {
                            // Skip if span extraction fails
                            return Ok(0);
                        }
                    };

                    // Show file progress
                    let file_name = file_path.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown");

                    overall_pb.set_message(format!("{} ({} spans)", file_name, spans.len()));

                    // Create progress bar for embedding
                    let embed_pb = multi.add(ProgressBar::new(spans.len() as u64));
                    embed_pb.set_style(
                        ProgressStyle::default_bar()
                            .template("  {spinner:.green} Embedding: [{bar:30.cyan/blue}] {pos}/{len} spans")
                            .unwrap()
                            .progress_chars("=>-")
                    );

                    // Embed spans in batches for efficiency
                    // OpenAI allows up to 2048 inputs per request, but we use 100 for:
                    // - Better progress visibility
                    // - Lower risk of hitting token limits per request
                    // - Still 10x faster than batch_size=10
                    let batch_size = 100;
                    let mut all_embeddings = Vec::new();

                    // Collect texts and embed in batches
                    {
                        let texts: Vec<&str> = spans.iter().map(|s| s.text.as_str()).collect();

                        for text_batch in texts.chunks(batch_size) {
                            let embeddings = match embedding::embed_batch(text_batch.to_vec(), None, None).await {
                                Ok(e) => e,
                                Err(_e) => {
                                    // Skip if embedding fails (API error, etc.)
                                    embed_pb.finish_and_clear();
                                    return Ok(0);
                                }
                            };
                            all_embeddings.extend(embeddings);
                            embed_pb.inc(text_batch.len() as u64);
                        }
                    }

                    // Now update spans with embeddings
                    for (span, emb) in spans.iter_mut().zip(all_embeddings.iter()) {
                        span.embedding = Some(emb.clone());
                        span.embedding_model = Some(embedding::embedding_model().to_string());
                    }

                    embed_pb.finish_and_clear();

                    // Insert spans (might fail, but continue anyway)
                    if let Err(_e) = db.insert_spans(&spans) {
                        return Ok(0);
                    }

                    Ok(spans.len())
                }.await;

                match file_result {
                    Ok(span_count) => {
                        if span_count > 0 {
                            total_spans += span_count;
                            successful += 1;
                        } else {
                            failed += 1;
                        }
                    }
                    Err(_e) => {
                        failed += 1;
                    }
                }

                overall_pb.inc(1);
            }

            overall_pb.finish_with_message(format!("{} spans created", total_spans));

            println!(
                "\n{} Indexed {} files → {} spans ({} successful, {} failed/skipped)",
                style("✓").green().bold(),
                style(successful).cyan().bold(),
                style(total_spans).cyan().bold(),
                style(successful).green(),
                style(failed).yellow()
            );
        }

        Commands::Compile {
            query,
            url,
            local,
            budget,
            json,
            db_path,
            backend,
        } => {
            if !json {
                println!(
                    "{} Compiling context for: {}\n",
                    style("🥑").green(),
                    style(&query).cyan().bold()
                );
            }

            // Prefer daemon mode unless --local is set
            if !local {
                let project = std::env::current_dir()
                    .unwrap_or_else(|_| PathBuf::from("."))
                    .canonicalize()
                    .unwrap_or_else(|_| PathBuf::from("."));

                let client = reqwest::Client::new();
                let resp = client
                    .post(format!("{}/compile", url.trim_end_matches('/')))
                    .json(&serde_json::json!({
                        "query": query,
                        "token_budget": budget,
                        "project": project.to_string_lossy(),
                        "backend": backend,
                    }))
                    .send()
                    .await?;

                if !resp.status().is_success() {
                    let status = resp.status();
                    let text = resp.text().await.unwrap_or_default();
                    anyhow::bail!("Server error {}: {}", status, text);
                }

                let v: serde_json::Value = resp.json().await?;
                let ws = v.get("working_set").cloned().unwrap_or(v);

                if json {
                    println!("{}", serde_json::to_string_pretty(&ws)?);
                } else {
                    // Print context text and brief stats
                    if let Some(text) = ws.get("text").and_then(|x| x.as_str()) {
                        println!("{}", text);
                        println!("\n{}", style("─".repeat(60)).dim());
                    }
                    let tokens = ws.get("tokens_used").and_then(|x| x.as_u64()).unwrap_or(0);
                    let spans = ws.get("citations").and_then(|c| c.as_array()).map(|a| a.len()).unwrap_or(0);
                    let utilization = if budget > 0 { ((tokens as f32 / budget as f32) * 100.0) as usize } else { 0 };
                    println!("{}  {} / {} ({}%)", style("Tokens:   ").bold(), style(tokens).cyan().bold(), style(budget).dim(), utilization);
                    println!("{}  {} spans", style("Compiled: ").bold(), style(spans).cyan().bold());
                }
                return Ok(());
            }

            // Local mode
            let db = Database::new(&db_path)?;

    // Show loading spinner
    let spinner = if !json {
        let sp = ProgressBar::new_spinner();
        sp.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} {msg}")
                .unwrap()
        );
        sp.set_message("Loading vector index...");
        sp.enable_steady_tick(std::time::Duration::from_millis(100));
        Some(sp)
    } else {
        None
    };

    // Get cached vector index (rebuilds only if data changed)
    // Note: For large repos, this can take 1-2 minutes on first query
    // Subsequent queries in the same session are fast (index stays in memory)
    // For best performance with large repos, use server mode
    if let Some(sp) = &spinner {
        sp.set_message("Building/loading vector index (this may take 1-2 min for large repos)...");
    }

    let index = db.get_vector_index()?;
    
    if let Some(sp) = &spinner {
        sp.set_message("Compiling context...");
    }

            // Compile context
            let config = CompilerConfig {
                token_budget: budget,
                ..Default::default()
            };

            if let Some(sp) = &spinner {
                sp.set_message("Compiling context (embedding query + hybrid search)...");
            }

            let working_set = compiler::compile(&query, config, &db, index.as_ref(), None).await?;

            if let Some(sp) = spinner {
                sp.finish_and_clear();
            }

            if json {
                println!("{}", serde_json::to_string_pretty(&working_set)?);
            } else {
                // Print context
                println!("{}", working_set.text);
                println!("\n{}", style("─".repeat(60)).dim());

                // Calculate utilization percentage
                let utilization = (working_set.tokens_used as f32 / budget as f32 * 100.0) as usize;

                // Print stats with colors
                println!(
                    "{}  {} / {} ({utilization}%)",
                    style("Tokens:   ").bold(),
                    style(working_set.tokens_used).cyan().bold(),
                    style(budget).dim(),
                );
                println!(
                    "{}  {} spans",
                    style("Compiled: ").bold(),
                    style(working_set.citations.len()).cyan().bold()
                );
                println!(
                    "{}  {}ms {}",
                    style("Time:     ").bold(),
                    style(working_set.compilation_time_ms).cyan().bold(),
                    if working_set.compilation_time_ms < 500 {
                        style("✓").green()
                    } else {
                        style("⚠").yellow()
                    }
                );
                println!(
                    "{}  {}",
                    style("Hash:     ").bold(),
                    style(&working_set.deterministic_hash()[..16]).dim()
                );
                println!();
            }
        }

        Commands::Stats { db_path } => {
            let db = Database::new(&db_path)?;
            let (artifacts, spans, tokens) = db.get_stats()?;

            // Calculate averages
            let avg_tokens_per_span = if spans > 0 {
                tokens / spans
            } else {
                0
            };

            let avg_spans_per_artifact = if artifacts > 0 {
                spans / artifacts
            } else {
                0
            };

            // Print header
            println!("\n{}", style("╔══════════════════════════════════════════════════════════════╗").cyan());
            println!("{}", style("║         AvocadoDB Database Statistics                       ║").cyan());
            println!("{}", style("╚══════════════════════════════════════════════════════════════╝").cyan());
            println!();

            // Main stats
            println!("  {} {}",
                style("Artifacts:").bold(),
                style(format!("{}", artifacts)).cyan().bold()
            );
            println!("  {} {}",
                style("Spans:    ").bold(),
                style(format!("{}", spans)).cyan().bold()
            );
            println!("  {} {}",
                style("Tokens:   ").bold(),
                style(format!("{}", tokens)).cyan().bold()
            );
            println!();

            // Averages
            println!("  {} {}",
                style("Avg tokens/span:   ").dim(),
                style(format!("{}", avg_tokens_per_span)).yellow()
            );
            println!("  {} {}",
                style("Avg spans/artifact:").dim(),
                style(format!("{}", avg_spans_per_artifact)).yellow()
            );
            println!();

            // Visual bar chart for token distribution
            if spans > 0 {
                println!("{}", style("  Token Distribution:").bold());
                println!();

                // Get individual span token counts for visualization
                let all_spans = db.get_all_spans()?;

                // Calculate buckets
                let mut buckets = vec![0; 5];
                let bucket_size = 200; // 0-200, 200-400, 400-600, 600-800, 800+

                for span in &all_spans {
                    let bucket_idx = (span.token_count / bucket_size).min(4);
                    buckets[bucket_idx] += 1;
                }

                let max_count = *buckets.iter().max().unwrap_or(&1);

                for (i, &count) in buckets.iter().enumerate() {
                    let range = if i == 4 {
                        format!("{}+", i * bucket_size)
                    } else {
                        format!("{}-{}", i * bucket_size, (i + 1) * bucket_size)
                    };

                    let bar_length = if max_count > 0 {
                        (count as f32 / max_count as f32 * 40.0) as usize
                    } else {
                        0
                    };

                    let bar = "█".repeat(bar_length);
                    let percentage = if spans > 0 {
                        (count as f32 / spans as f32 * 100.0) as usize
                    } else {
                        0
                    };

                    println!(
                        "    {:>8} tokens │{:<40}│ {} ({}%)",
                        style(range).dim(),
                        style(bar).cyan(),
                        style(format!("{:>4}", count)).yellow(),
                        style(format!("{:>2}", percentage)).dim()
                    );
                }
                println!();
            }

            // Status indicator
            if spans < 10 {
                println!("  {} Small corpus - good for testing", style("ℹ").blue());
            } else if spans < 1000 {
                println!("  {} Optimal size for Phase 1", style("✓").green());
            } else if spans < 10000 {
                println!("  {} Large corpus - consider monitoring performance", style("⚠").yellow());
            } else {
                println!("  {} Very large corpus - Phase 2 HNSW recommended", style("⚠").yellow());
            }
            println!();
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

        Commands::Ask {
            query,
            llm,
            budget,
            max_tokens,
            url,
            json,
        } => {
            // Find the ask.py script
            // Try multiple locations:
            // 1. Relative to current directory (development)
            // 2. Relative to executable (installed)
            // 3. In cargo target directory (during build)
            let mut script_paths = vec![
                PathBuf::from("avocado-cli/scripts/ask.py"),
                PathBuf::from("../avocado-cli/scripts/ask.py"),
            ];

            // Try relative to executable
            if let Ok(exe_path) = std::env::current_exe() {
                if let Some(exe_dir) = exe_path.parent() {
                    script_paths.push(exe_dir.join("scripts").join("ask.py"));
                    script_paths.push(exe_dir.parent().unwrap_or(exe_dir).join("avocado-cli").join("scripts").join("ask.py"));
                }
            }

            // Find first existing script
            let script = script_paths
                .iter()
                .find(|p| p.exists())
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "Could not find ask.py script. Tried: {}. \
                        Please ensure the script exists in avocado-cli/scripts/ask.py",
                        script_paths.iter()
                            .map(|p| p.display().to_string())
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                })?;

            // Build command
            let mut cmd = Command::new("python3");
            cmd.arg(&script);
            cmd.arg(&query);
            cmd.arg("--url").arg(&url);
            cmd.arg("--budget").arg(budget.to_string());
            cmd.arg("--llm").arg(&llm);
            cmd.arg("--max-tokens").arg(max_tokens.to_string());
            if json {
                cmd.arg("--json");
            }

            // Run and capture output
            let output = cmd.output()?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                eprintln!("{}", stderr);
                return Err(anyhow::anyhow!("Ask command failed"));
            }

            // Print output
            let stdout = String::from_utf8_lossy(&output.stdout);
            print!("{}", stdout);
        }

        Commands::Benchmark { verbose } => {
            commands::run_benchmark(verbose).await?;
        }

        Commands::Recommend {
            corpus_size,
            use_case,
        } => {
            commands::recommend_model(corpus_size, use_case.as_deref())?;
        }

        Commands::Serve {
            gpu,
            embed_url,
            model,
            dim,
            url,
            prewarm,
        } => {
            // Build environment for the server process
            let mut server_cmd = Command::new("avocado-server");
            server_cmd.env("RUST_LOG", "info");

            if gpu {
                let remote = embed_url.clone().unwrap_or_default();
                if remote.is_empty() {
                    anyhow::bail!("--embed-url is required when using --gpu");
                }
                server_cmd
                    .env("AVOCADODB_EMBEDDING_PROVIDER", "remote")
                    .env("AVOCADODB_EMBEDDING_MODEL", &model)
                    .env("AVOCADODB_EMBEDDING_DIM", dim.to_string())
                    .env("AVOCADODB_EMBEDDING_URL", &remote)
                    .env("AVOCADODB_FORBID_FALLBACKS", "1");
            } else {
                server_cmd
                    .env("AVOCADODB_EMBEDDING_PROVIDER", "local")
                    .env("AVOCADODB_FORBID_FALLBACKS", "1");
            }

            // Spawn the server process
            let mut child = match server_cmd.spawn() {
                Ok(c) => c,
                Err(e) => {
                    // Fallback to running from local target if not in PATH
                    let mut alt = Command::new(
                        std::env::current_exe()
                            .ok()
                            .and_then(|p| p.parent().map(|d| d.to_path_buf()))
                            .unwrap_or_else(|| PathBuf::from(".")) // best effort
                            .join("avocado-server"),
                    );
                    alt.envs(server_cmd.get_envs().filter_map(|(k, v)| {
                        v.map(|vv| (k.to_os_string(), vv.to_os_string()))
                    }));
                    alt.env("RUST_LOG", "info");
                    alt.spawn().map_err(|_| e)?
                }
            };

            // Wait for health
            let client = reqwest::blocking::Client::new();
            let start = std::time::Instant::now();
            loop {
                // Check if server died
                if let Ok(Some(status)) = child.try_wait() {
                    anyhow::bail!("avocado-server exited prematurely with status {}", status);
                }
                match client.get(format!("{}/health", url.trim_end_matches('/'))).send() {
                    Ok(resp) if resp.status().is_success() => break,
                    _ => std::thread::sleep(std::time::Duration::from_millis(300)),
                }
                if start.elapsed() > std::time::Duration::from_secs(30) {
                    anyhow::bail!("timeout waiting for server health at {}/health", url);
                }
            }

            // Optional pre-warm for GPU endpoint
            if gpu && prewarm {
                if let Some(remote) = embed_url {
                    let _ = client
                        .post(remote)
                        .json(&serde_json::json!({"inputs": ["warmup 1","warmup 2","warmup 3"]}))
                        .send();
                }
            }

            println!(
                "✓ Avocado server ready at {} ({})",
                url,
                if gpu { "remote embeddings (GPU)" } else { "local CPU embeddings" }
            );
        }

        Commands::Session { command } => {
            commands::handle_session_command(command).await?;
        }
    }

    Ok(())
}

/// Collect files from a directory
fn collect_files(dir: &PathBuf, recursive: bool) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    
    // Directories to skip (common build artifacts, dependencies, etc.)
    let skip_dirs: &[&str] = &[
        ".git", ".svn", ".hg",
        "node_modules", ".node_modules",
        "venv", ".venv", "env", ".env",
        "__pycache__", ".pytest_cache",
        "target", "build", "dist", "out",
        ".next", ".cache", ".tox",
        "vendor", ".bundle",
        ".idea", ".vscode", ".vs",
        ".avocado",  // Skip AvocadoDB's own database directory
    ];

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        
        // Skip if it's a skip directory
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if skip_dirs.contains(&name) {
                continue;
            }
        }

        if path.is_file() {
            files.push(path);
        } else if path.is_dir() && recursive {
            files.extend(collect_files(&path, recursive)?);
        }
    }

    Ok(files)
}
