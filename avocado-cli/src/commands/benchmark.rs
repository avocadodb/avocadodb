//! Benchmark command implementation
//!
//! Runs performance benchmarks and displays results in a user-friendly format.

use anyhow::Result;
use avocado_core::embedding;
use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Instant;

/// Benchmark results for display
#[allow(dead_code)]
pub struct BenchmarkResults {
    /// Time to generate a single embedding in milliseconds
    pub single_embedding_ms: f64,
    /// Time to generate 10 embeddings in milliseconds
    pub batch_10_ms: f64,
    /// Time to generate 50 embeddings in milliseconds
    pub batch_50_ms: f64,
    /// Time to generate 100 embeddings in milliseconds
    pub batch_100_ms: f64,
    /// Name of the embedding model used
    pub model_name: String,
    /// Dimension of the embedding vectors
    pub dimensions: usize,
    /// Hardware performance rating (e.g., "excellent", "good", "acceptable")
    pub hardware_rating: String,
}

/// Run embedding performance benchmarks
pub async fn run_benchmark(verbose: bool) -> Result<BenchmarkResults> {
    println!("\n{}", style("🥑 AvocadoDB Performance Benchmark").bold().green());
    println!("{}", style("─".repeat(60)).dim());

    let model_name = embedding::embedding_model().to_string();
    let dimensions = embedding::embedding_dimension();

    println!("\n{} {}", style("Model:").bold(), style(&model_name).cyan());
    println!("{} {} dimensions", style("Dimensions:").bold(), style(dimensions).cyan());
    println!();

    // Create progress spinner
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap()
    );

    // Benchmark 1: Single embedding
    spinner.set_message("Benchmarking single embedding...");
    spinner.enable_steady_tick(std::time::Duration::from_millis(100));

    let test_text = "This is a test query for embedding performance benchmarking";
    let mut durations = Vec::new();

    // Warmup
    for _ in 0..3 {
        embedding::embed_text(test_text, None, None).await?;
    }

    // Actual benchmark (10 runs)
    for _ in 0..10 {
        let start = Instant::now();
        embedding::embed_text(test_text, None, None).await?;
        durations.push(start.elapsed().as_secs_f64() * 1000.0);
    }

    let single_ms = median(&durations);
    spinner.finish_and_clear();

    println!("  {} {:.2}ms",
        style("✓ Single embedding:").green(),
        style(format!("{:.2}", single_ms)).cyan().bold()
    );

    // Benchmark 2: Batch of 10
    spinner.set_message("Benchmarking batch of 10...");
    spinner.enable_steady_tick(std::time::Duration::from_millis(100));

    let texts_10: Vec<&str> = (0..10).map(|_| test_text).collect();
    durations.clear();

    for _ in 0..5 {
        let start = Instant::now();
        embedding::embed_batch(texts_10.clone(), None, None).await?;
        durations.push(start.elapsed().as_secs_f64() * 1000.0);
    }

    let batch_10_ms = median(&durations);
    spinner.finish_and_clear();

    println!("  {} {:.2}ms ({:.2}ms per text)",
        style("✓ Batch of 10:").green(),
        style(format!("{:.2}", batch_10_ms)).cyan().bold(),
        style(format!("{:.2}", batch_10_ms / 10.0)).dim()
    );

    // Benchmark 3: Batch of 50
    spinner.set_message("Benchmarking batch of 50...");
    spinner.enable_steady_tick(std::time::Duration::from_millis(100));

    let texts_50: Vec<&str> = (0..50).map(|_| test_text).collect();
    durations.clear();

    for _ in 0..3 {
        let start = Instant::now();
        embedding::embed_batch(texts_50.clone(), None, None).await?;
        durations.push(start.elapsed().as_secs_f64() * 1000.0);
    }

    let batch_50_ms = median(&durations);
    spinner.finish_and_clear();

    println!("  {} {:.2}ms ({:.2}ms per text)",
        style("✓ Batch of 50:").green(),
        style(format!("{:.2}", batch_50_ms)).cyan().bold(),
        style(format!("{:.2}", batch_50_ms / 50.0)).dim()
    );

    // Benchmark 4: Batch of 100
    spinner.set_message("Benchmarking batch of 100...");
    spinner.enable_steady_tick(std::time::Duration::from_millis(100));

    let texts_100: Vec<&str> = (0..100).map(|_| test_text).collect();
    durations.clear();

    for _ in 0..3 {
        let start = Instant::now();
        embedding::embed_batch(texts_100.clone(), None, None).await?;
        durations.push(start.elapsed().as_secs_f64() * 1000.0);
    }

    let batch_100_ms = median(&durations);
    spinner.finish_and_clear();

    println!("  {} {:.2}ms ({:.2}ms per text)",
        style("✓ Batch of 100:").green(),
        style(format!("{:.2}", batch_100_ms)).cyan().bold(),
        style(format!("{:.2}", batch_100_ms / 100.0)).dim()
    );

    // Calculate hardware rating
    let hardware_rating = rate_hardware(single_ms, batch_100_ms);

    println!("\n{}", style("─".repeat(60)).dim());
    println!("{} {}", style("Hardware Rating:").bold(), style(&hardware_rating).yellow().bold());

    // Comparison with OpenAI
    print_comparison(single_ms);

    println!();

    if verbose {
        print_detailed_stats(&durations);
    }

    Ok(BenchmarkResults {
        single_embedding_ms: single_ms,
        batch_10_ms,
        batch_50_ms,
        batch_100_ms,
        model_name,
        dimensions,
        hardware_rating,
    })
}

/// Calculate median of a list of durations
fn median(durations: &[f64]) -> f64 {
    let mut sorted = durations.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let mid = sorted.len() / 2;
    if sorted.len() % 2 == 0 {
        (sorted[mid - 1] + sorted[mid]) / 2.0
    } else {
        sorted[mid]
    }
}

/// Rate hardware performance
fn rate_hardware(single_ms: f64, batch_100_ms: f64) -> String {
    // Rating based on single embedding performance
    // Excellent: <2ms, Good: <5ms, Fair: <10ms, Slow: >=10ms

    let throughput = 100.0 / batch_100_ms * 1000.0; // embeddings per second

    let rating = if single_ms < 2.0 && throughput > 1000.0 {
        "⭐⭐⭐⭐⭐ Excellent (High-end CPU/GPU)"
    } else if single_ms < 5.0 && throughput > 500.0 {
        "⭐⭐⭐⭐ Good (Modern CPU)"
    } else if single_ms < 10.0 && throughput > 200.0 {
        "⭐⭐⭐ Fair (Average CPU)"
    } else {
        "⭐⭐ Needs Optimization (Consider GPU or faster CPU)"
    };

    rating.to_string()
}

/// Print comparison with OpenAI
fn print_comparison(pure_rust_ms: f64) {
    println!("\n{}", style("Comparison with OpenAI:").bold());
    println!("{}", style("─".repeat(60)).dim());

    // OpenAI typical: 200-300ms for ada-002
    let openai_typical_ms = 250.0;
    let speedup = openai_typical_ms / pure_rust_ms;

    println!("  {} ~{:.0}ms (typical)",
        style("OpenAI ada-002:").dim(),
        openai_typical_ms
    );
    println!("  {} {:.2}ms",
        style("Pure Rust:").green().bold(),
        style(format!("{:.2}", pure_rust_ms)).cyan().bold()
    );
    println!("\n  {} {}x faster",
        style("Speedup:").bold(),
        style(format!("{:.1}", speedup)).green().bold()
    );

    // Cost comparison
    println!("\n{}", style("Cost:").bold());
    println!("  {} $0 (free)", style("Pure Rust:").green().bold());
    println!("  {} ~$0.0001 per 1K tokens", style("OpenAI:").dim());
}

/// Print detailed statistics
fn print_detailed_stats(durations: &[f64]) {
    println!("\n{}", style("Detailed Statistics:").bold());
    println!("{}", style("─".repeat(60)).dim());

    let min = durations.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = durations.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let avg = durations.iter().sum::<f64>() / durations.len() as f64;
    let median_val = median(durations);

    println!("  Min: {:.2}ms", min);
    println!("  Max: {:.2}ms", max);
    println!("  Avg: {:.2}ms", avg);
    println!("  Median: {:.2}ms", median_val);
}
