use avocado_core::{embedding, span, Span};
use avocado_core::approx::{ApproxIndex, HnswBackend, InstantBackend};
use serde_json::json;
use sha2::{Sha256, Digest};
use std::time::Instant;
use std::fs;
use std::path::Path;

fn build_corpus(num_docs: usize, lines_per_doc: usize) -> (Vec<Span>, Vec<Vec<f32>>, usize) {
    let mut spans_all = Vec::new();
    let mut total_lines = 0usize;
    for i in 0..num_docs {
        let mut content = String::new();
        for l in 0..lines_per_doc {
            content.push_str(&format!("Doc {} line {}: Deterministic RAG with MMR & hybrid search.\n", i, l));
        }
        total_lines += lines_per_doc;
        // Derive artifact id deterministically
        let mut hasher = Sha256::new();
        hasher.update(format!("artifact-{}", i));
        let artifact_id = format!("{:x}", hasher.finalize());
        let spans = span::extract_spans(&content, &artifact_id).expect("extract_spans");
        spans_all.extend(spans);
    }
    // Embed
    let texts: Vec<&str> = spans_all.iter().map(|s| s.text.as_str()).collect();
    let rt = tokio::runtime::Runtime::new().unwrap();
    let embeddings = rt.block_on(embedding::embed_batch(texts, None, None)).expect("embed");
    (spans_all, embeddings, total_lines)
}

fn main() {
    let num_docs: usize = std::env::var("DOCS").ok().and_then(|s| s.parse().ok()).unwrap_or(500);
    let lines_per_doc: usize = std::env::var("LINES").ok().and_then(|s| s.parse().ok()).unwrap_or(20);
    let k: usize = std::env::var("TOPK").ok().and_then(|s| s.parse().ok()).unwrap_or(50);
    let query = "What is AvocadoDB? Deterministic context compilation.";

    let (mut spans, embeddings, _total_lines) = build_corpus(num_docs, lines_per_doc);
    // Attach embeddings to spans
    for (s, e) in spans.iter_mut().zip(embeddings.iter()) {
        s.embedding = Some(e.clone());
        s.embedding_model = Some(embedding::embedding_model().to_string());
    }

    // Query embedding
    let rt = tokio::runtime::Runtime::new().unwrap();
    let q_emb = rt.block_on(embedding::embed_text(query, None, None)).expect("q_emb");

    // HNSW build/search/save/load (baseline)
    let t0 = Instant::now();
    let hnsw = HnswBackend::build(spans.clone());
    let hnsw_build_ms = t0.elapsed().as_millis();
    let hnsw_results = hnsw.search(&q_emb, k).expect("hnsw search");

    let cache_dir_root = std::env::temp_dir().join("avocado_ann_spike");
    let cache_dir_hnsw = cache_dir_root.join("hnsw");
    let cache_dir_inst = cache_dir_root.join("instant");
    let _ = std::fs::remove_dir_all(&cache_dir_root);
    std::fs::create_dir_all(&cache_dir_hnsw).unwrap();
    std::fs::create_dir_all(&cache_dir_inst).unwrap();
    hnsw.save_to_disk(&cache_dir_hnsw).expect("hnsw save");
    let t1 = Instant::now();
    let hnsw_loaded_res = HnswBackend::load_from_disk(&cache_dir_hnsw);
    let hnsw_load_ms = t1.elapsed().as_millis();
    let hnsw_loaded = match hnsw_loaded_res {
        Ok(opt) => opt.is_some(),
        Err(e) => {
            eprintln!("HNSW load failed: {}", e);
            false
        }
    };
    let hnsw_index_bytes = dir_size_bytes(&cache_dir_hnsw);

    // Instant build/search/save/load (spike)
    let t2 = Instant::now();
    let instant = InstantBackend::build(spans.clone());
    let instant_build_ms = t2.elapsed().as_millis();
    let inst_results = instant.search(&q_emb, k).expect("instant search");
    instant.save_to_disk(&cache_dir_inst).expect("instant save");
    let t3 = Instant::now();
    let instant_loaded = match InstantBackend::load_from_disk(&cache_dir_inst) {
        Ok(opt) => opt.is_some(),
        Err(e) => {
            eprintln!("Instant load failed: {}", e);
            false
        }
    };
    let instant_load_ms = t3.elapsed().as_millis();
    let instant_index_bytes = dir_size_bytes(&cache_dir_inst);

    // Determinism under light concurrency for instant backend
    use std::sync::atomic::{AtomicBool, Ordering};
    let inst_for_threads = InstantBackend::build(spans.clone());
    let base = inst_for_threads.search(&q_emb, k).unwrap();
    let base_ids: Vec<(String, usize)> = base.iter().map(|s| (s.span.id.clone(), s.span.start_line)).collect();
    let q_emb_arc = std::sync::Arc::new(q_emb.clone());
    let base_ids_arc = std::sync::Arc::new(base_ids.clone());
    let det_ok = std::sync::Arc::new(AtomicBool::new(true));
    std::thread::scope(|scope| {
        for _ in 0..4 {
            let backend_ref = &inst_for_threads;
            let q_clone = q_emb_arc.clone();
            let base_ids_clone = base_ids_arc.clone();
            let det_flag = det_ok.clone();
            scope.spawn(move || {
                let r = backend_ref.search(&q_clone, k).unwrap();
                let ids: Vec<(String, usize)> = r.iter().map(|s| (s.span.id.clone(), s.span.start_line)).collect();
                if ids != *base_ids_clone {
                    // Mark as non-deterministic (shared flag; best-effort)
                    // In a spike harness we just print
                    println!("non_deterministic_ordering");
                    det_flag.store(false, Ordering::Relaxed);
                }
            });
        }
    });

    // Compare top-k overlap/ordering between backends (informational)
    use std::collections::HashSet;
    let ids_hnsw: HashSet<String> = hnsw_results.iter().map(|s| s.span.id.clone()).collect();
    let ids_inst: HashSet<String> = inst_results.iter().map(|s| s.span.id.clone()).collect();
    let overlap = ids_hnsw.intersection(&ids_inst).count();

    // Output JSON summary
    let summary = json!({
        "docs": num_docs,
        "lines_per_doc": lines_per_doc,
        "k": k,
        "hnsw": {
            "build_ms": hnsw_build_ms,
            "load_ms": hnsw_load_ms,
            "loaded": hnsw_loaded,
            "index_bytes": hnsw_index_bytes,
        },
        "instant": {
            "build_ms": instant_build_ms,
            "load_ms": instant_load_ms,
            "loaded": instant_loaded,
            "index_bytes": instant_index_bytes,
        },
        "overlap_topk": overlap,
        "deterministic_instant_concurrent": det_ok.load(Ordering::Relaxed),
        "max_rss_kb": max_rss_kb(),
    });
    println!("{}", serde_json::to_string_pretty(&summary).unwrap());
}

#[allow(dead_code)]
fn dir_size_bytes(path: &Path) -> u64 {
    let mut total: u64 = 0;
    if let Ok(entries) = fs::read_dir(path) {
        for e in entries.flatten() {
            if let Ok(md) = e.metadata() {
                if md.is_file() {
                    total = total.saturating_add(md.len());
                }
            }
        }
    }
    total
}

#[allow(dead_code)]
fn max_rss_kb() -> u64 {
    // Use getrusage to get max resident set size in kilobytes
    unsafe {
        let mut usage: libc::rusage = std::mem::zeroed();
        if libc::getrusage(libc::RUSAGE_SELF, &mut usage) == 0 {
            #[cfg(target_os = "macos")]
            {
                // macOS returns bytes in ru_maxrss
                return (usage.ru_maxrss as u64) / 1024;
            }
            #[cfg(not(target_os = "macos"))]
            {
                // Linux returns kilobytes
                return usage.ru_maxrss as u64;
            }
        }
    }
    0
}

