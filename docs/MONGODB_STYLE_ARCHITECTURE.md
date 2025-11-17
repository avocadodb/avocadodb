# MongoDB-Style Daemon Architecture

## Overview

AvocadoDB now uses a **MongoDB-style architecture**: one daemon server managing multiple project indexes, just like MongoDB manages multiple databases.

## The Architecture

```
┌─────────────────────────────────────────┐
│     AvocadoDB Daemon (Port 8765)        │
│                                         │
│  ┌───────────────────────────────────┐ │
│  │  Project Manager (LRU Cache)      │ │
│  │                                   │ │
│  │  project-a/ → Database + HNSW    │ │
│  │  project-b/ → Database + HNSW    │ │
│  │  project-c/ → Database + HNSW    │ │
│  │  ... (up to 10 projects)         │ │
│  └───────────────────────────────────┘ │
└─────────────────────────────────────────┘
         ▲              ▲              ▲
         │              │              │
    project-a/    project-b/    project-c/
```

## How It Works

### 1. One Daemon, Multiple Projects

```bash
# Start ONE daemon (like MongoDB)
avocado-server &

# All projects use the same daemon
cd ~/project-a
avocado compile "query"  # Uses project-a index

cd ~/project-b
avocado compile "query"  # Uses project-b index
```

### 2. Automatic Project Detection

The daemon automatically detects which project to use based on the **current working directory**:

```python
# Python SDK
from avocado import AvocadoDB

# Automatically uses current directory as project
db = AvocadoDB()  # Detects project from os.getcwd()
result = db.compile("query")
```

### 3. In-Memory Indexes

Each project's HNSW index stays in memory:

- **First query**: Builds index (1-2 min for large repos)
- **Subsequent queries**: <100ms (index in memory)
- **Switch projects**: Instant (index already loaded)

### 4. LRU Eviction

The daemon keeps up to 10 projects in memory:

- Most recently used projects stay loaded
- Least recently used projects are evicted
- Evicted projects reload on next access (still fast with cached spans)

## Benefits

### ✅ Solves HNSW Lifetime Issue

- Indexes stay in memory (no serialization needed)
- No lifetime constraints
- Fast queries (<100ms)

### ✅ Multi-Repo Support

- One daemon manages all projects
- No port management
- Automatic project detection

### ✅ Simple User Experience

```bash
# Just works - no server management
cd ~/any-project
avocado ask "how does X work?"  # Fast, automatic
```

### ✅ Efficient Resource Usage

- Shares resources across projects
- LRU eviction prevents memory bloat
- Like MongoDB, Redis, Postgres

## Usage

### Starting the Daemon

```bash
# Manual start
avocado-server &

# Or auto-start (Python SDK)
from avocado import AvocadoDB
db = AvocadoDB()  # Auto-starts daemon if not running
```

### Using Multiple Projects

```bash
# Project A
cd ~/project-a
avocado compile "query 1"  # Fast (<100ms after first query)

# Project B  
cd ~/project-b
avocado compile "query 2"  # Fast (<100ms after first query)

# Back to Project A
cd ~/project-a
avocado compile "query 3"  # Still fast (index in memory)
```

### Python SDK

```python
from avocado import AvocadoDB

# Automatically detects project from current directory
db = AvocadoDB()  # HTTP mode (default)

# Compile query (uses current project)
result = db.compile("how does auth work?")

# Switch projects
import os
os.chdir("~/other-project")
db2 = AvocadoDB()  # Now uses other-project
result2 = db2.compile("what's the API?")
```

## Implementation Details

### Server Side

```rust
// One server, multiple projects
struct AppState {
    projects: Arc<RwLock<HashMap<PathBuf, ProjectIndex>>>,
}

struct ProjectIndex {
    database: Database,
    hnsw_index: Arc<VectorIndex>,  // Stays in memory!
    last_accessed: Instant,
}
```

### Client Side

```python
# Auto-detects project path
self.project_path = str(Path.cwd().resolve())

# Includes in all requests
response = requests.post("/compile", json={
    "query": query,
    "project": self.project_path  # Auto-detected
})
```

## Comparison: Before vs After

| Aspect | Before (CLI Mode) | After (Daemon Mode) |
|--------|------------------|---------------------|
| **First Query** | 1-2 minutes | 1-2 minutes (builds index) |
| **Subsequent Queries** | 1-2 minutes (rebuilds) | <100ms (index in memory) |
| **Multi-Repo** | Manual port management | Automatic (one daemon) |
| **Memory** | Per-process | Shared (LRU eviction) |
| **User Experience** | Manage servers | Just works |

## Configuration

### Environment Variables

```bash
# Server URL (default: http://localhost:8765)
export AVOCADODB_URL="http://localhost:8765"

# Auto-start daemon (default: true)
export AVOCADODB_AUTO_START="true"
```

### Memory Limits

```rust
// Maximum projects in memory (default: 10)
const MAX_PROJECTS_IN_MEMORY: usize = 10;
```

## Future Enhancements

1. **Configurable LRU limit**: Set via environment variable
2. **Memory-based eviction**: Evict based on total memory usage
3. **Persistent connections**: Keep connections alive between requests
4. **Project warmup**: Pre-load frequently used projects

## Conclusion

The MongoDB-style architecture solves both the HNSW lifetime issue and multi-repo support in one elegant solution. It's simple, efficient, and provides the best user experience.

