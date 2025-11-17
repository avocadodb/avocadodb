// Example demonstrating the HNSW lifetime issue
// This file is for documentation purposes only

use hnsw_rs::prelude::*;
use hnsw_rs::hnswio::HnswIo;
use std::path::Path;

// ============================================================================
// THE PROBLEM: What we want to do (but can't)
// ============================================================================

struct VectorIndex {
    // We want to store HNSW with 'static lifetime so it can be:
    // - Stored in Arc (requires 'static)
    // - Shared across threads
    // - Returned from functions without lifetime parameters
    hnsw: Option<Hnsw<'static, f32, DistCosine>>,
    spans: Vec<Span>,
}

fn load_index_broken(cache_dir: &Path) -> Result<VectorIndex> {
    let mut hnsw_io = HnswIo::new(cache_dir, "index");
    
    // PROBLEM 1: Lifetime inference fails
    // The compiler can't figure out what lifetime 'b should be
    let hnsw = hnsw_io.load_hnsw()?;
    //           ^^^^^^^^^^^^^^^^^^
    // Error: cannot infer an appropriate lifetime for lifetime parameter 'b
    
    // PROBLEM 2: Even if we specify the lifetime explicitly:
    let hnsw: Hnsw<'static, f32, DistCosine> = hnsw_io.load_hnsw()?;
    //           ^^^^^^^^
    // Error: lifetime may not live long enough
    //        `hnsw_io` has lifetime 'a, but Hnsw needs 'static
    
    // PROBLEM 3: We can't return it
    Ok(VectorIndex {
        hnsw: Some(hnsw),  // Error: `hnsw` does not live long enough
        spans: vec![],
    })
    // hnsw_io is dropped here, invalidating hnsw
}

// ============================================================================
// ATTEMPT 1: Keep HnswIo alive (doesn't work)
// ============================================================================

struct VectorIndexWithIo<'a> {
    // Now VectorIndex has a lifetime parameter
    hnsw: Option<Hnsw<'a, f32, DistCosine>>,
    hnsw_io: Option<HnswIo>,  // Keep it alive
    spans: Vec<Span>,
}

fn load_index_with_io<'a>(cache_dir: &Path) -> Result<VectorIndexWithIo<'a>> {
    let mut hnsw_io = HnswIo::new(cache_dir, "index");
    let hnsw = hnsw_io.load_hnsw()?;
    
    // PROBLEM: We can't move hnsw_io into the struct while hnsw borrows it
    Ok(VectorIndexWithIo {
        hnsw: Some(hnsw),      // Borrows from hnsw_io
        hnsw_io: Some(hnsw_io), // Can't move while borrowed
        //                      ^^^^^^^^^^^^
        // Error: cannot move out of `hnsw_io` because it is borrowed
        spans: vec![],
    })
}

// ============================================================================
// ATTEMPT 2: Use generic lifetimes (propagates everywhere)
// ============================================================================

struct VectorIndexGeneric<'a> {
    hnsw: Option<Hnsw<'a, f32, DistCosine>>,
    spans: Vec<Span>,
}

// Now EVERYTHING needs lifetime parameters:
struct Database<'a> {
    vector_index: Option<VectorIndexGeneric<'a>>,
}

impl<'a> Database<'a> {
    fn get_vector_index(&self) -> &VectorIndexGeneric<'a> {
        // Lifetime 'a must outlive the Database instance
        // This means Database can't be stored in Arc, shared across threads, etc.
        self.vector_index.as_ref().unwrap()
    }
}

// PROBLEM: Can't use Arc<Database<'a>> because Arc requires 'static
// PROBLEM: Can't return Database from functions without lifetime parameters
// PROBLEM: Lifetime parameters propagate to ALL callers

// ============================================================================
// ATTEMPT 3: Unsafe code (risky, not recommended)
// ============================================================================

use std::sync::Mutex;

// Global storage to keep HnswIo alive
static HNSW_IO_STORAGE: Mutex<Option<HnswIo>> = Mutex::new(None);

fn load_index_unsafe(cache_dir: &Path) -> Result<VectorIndex> {
    let mut hnsw_io = HnswIo::new(cache_dir, "index");
    let hnsw_loaded = hnsw_io.load_hnsw()?;
    
    // UNSAFE: We're lying to the compiler about the lifetime
    // We promise the data is actually 'static, but it's not
    let hnsw_static: Hnsw<'static, f32, DistCosine> = unsafe {
        std::mem::transmute(hnsw_loaded)
    };
    
    // Store hnsw_io globally to keep it alive
    *HNSW_IO_STORAGE.lock().unwrap() = Some(hnsw_io);
    
    Ok(VectorIndex {
        hnsw: Some(hnsw_static),
        spans: vec![],
    })
    
    // PROBLEMS:
    // 1. If HNSW_IO_STORAGE is ever cleared, hnsw_static becomes invalid
    // 2. If hnsw_rs actually uses references internally, this is undefined behavior
    // 3. Not thread-safe (multiple loads would overwrite each other)
    // 4. Memory leak (HnswIo is never dropped)
    // 5. Breaks if library internals change
}

// ============================================================================
// SOLUTION: What we actually do (rebuild from cached spans)
// ============================================================================

fn load_index_actual(cache_dir: &Path) -> Result<VectorIndex> {
    // Load cached spans (fast, no lifetime issues)
    let spans: Vec<Span> = load_cached_spans(cache_dir)?;
    
    // Rebuild HNSW from spans (slower but necessary)
    // This creates a new HNSW with 'static lifetime
    let hnsw = build_hnsw_from_spans(&spans)?;
    
    Ok(VectorIndex {
        hnsw: Some(hnsw),  // Now it's truly 'static
        spans,
    })
}

// ============================================================================
// IDEAL SOLUTION: What we'd need in hnsw_rs
// ============================================================================

// In hnsw_rs, we'd need a method like this:
/*
impl HnswIo {
    /// Load HNSW with all data owned (no lifetime constraints)
    pub fn load_hnsw_owned<T, D>(&mut self) -> Result<Hnsw<'static, T, D>>
    where
        T: 'static + Serialize + DeserializeOwned + Clone + Sized + Send + Sync,
        D: Distance<T> + Default + Send + Sync,
    {
        // 1. Load with borrowed lifetime
        let hnsw_borrowed = self.load_hnsw()?;
        
        // 2. Extract all internal data (needs to be added to hnsw_rs)
        let internal_data = hnsw_borrowed.extract_owned_data()?;
        
        // 3. Rebuild with owned data
        let mut hnsw_owned = Hnsw::new(
            internal_data.max_nb_connection,
            internal_data.max_elements,
            internal_data.max_layer,
            internal_data.ef_construction,
            D::default(),
        );
        
        // 4. Reinsert all points (now owned)
        for (point, id) in internal_data.points {
            hnsw_owned.insert((point, id));
        }
        
        Ok(hnsw_owned)
    }
}
*/

// Then we could do:
fn load_index_ideal(cache_dir: &Path) -> Result<VectorIndex> {
    let mut hnsw_io = HnswIo::new(cache_dir, "index");
    
    // Now this works! Returns Hnsw<'static, ...>
    let hnsw = hnsw_io.load_hnsw_owned()?;
    
    Ok(VectorIndex {
        hnsw: Some(hnsw),  // 'static lifetime, no issues
        spans: vec![],
    })
}

// ============================================================================
// Helper functions (not implemented, just for illustration)
// ============================================================================

struct Span;

fn load_cached_spans(_cache_dir: &Path) -> Result<Vec<Span>> {
    Ok(vec![])
}

fn build_hnsw_from_spans(_spans: &[Span]) -> Result<Hnsw<'static, f32, DistCosine>> {
    // Implementation would build HNSW from scratch
    Ok(Hnsw::new(32, 1000, 16, 200, DistCosine::default()))
}

