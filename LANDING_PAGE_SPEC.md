# AvocadoDB Landing Page Specification

**Domain**: avocadodb.ai
**Purpose**: Convert visitors to GitHub stars and users
**Target**: AI engineers, developers building RAG systems

---

## WIREFRAME STRUCTURE

```
┌─────────────────────────────────────────────────────────────────┐
│                         NAVBAR                                   │
│  [Logo] AvocadoDB      Docs  GitHub  crates.io    [Get Started] │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│                         HERO SECTION                             │
│                                                                  │
│     The Deterministic Context Database                           │
│           for AI Agents                                          │
│                                                                  │
│     Fix your RAG in 5 minutes.                                   │
│     Same query → same context, every time.                       │
│                                                                  │
│     ┌─────────┐  ┌─────────┐  ┌─────────┐                       │
│     │6x Faster│  │  100%   │  │   $0    │                       │
│     │ 40-60ms │  │Determin.│  │  Cost   │                       │
│     └─────────┘  └─────────┘  └─────────┘                       │
│                                                                  │
│     [Get Started]          [View on GitHub ⭐]                   │
│                                                                  │
│     ┌──────────────────────────────────────┐                    │
│     │ $ cargo install avocado-cli          │                    │
│     └──────────────────────────────────────┘                    │
│                                                                  │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│                    THE PROBLEM SECTION                           │
│                                                                  │
│     "Why Your RAG System is Broken"                              │
│                                                                  │
│     ┌─────────────────┐  ┌─────────────────┐                    │
│     │  BEFORE         │  │  AFTER          │                    │
│     │  ❌ Random       │  │  ✅ Deterministic│                   │
│     │  ❌ 60% tokens   │  │  ✅ 95% tokens   │                   │
│     │  ❌ No citations │  │  ✅ Line-level   │                   │
│     │  ❌ 300ms        │  │  ✅ 50ms         │                   │
│     │  ❌ API costs    │  │  ✅ Free         │                   │
│     └─────────────────┘  └─────────────────┘                    │
│                                                                  │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│                    DEMO / ANIMATION                              │
│                                                                  │
│     [Terminal animation showing]:                                │
│     $ avocado compile "How does auth work?"                      │
│     → Shows deterministic output with hash                       │
│     → Run again, same hash appears                               │
│                                                                  │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│                    KEY FEATURES (6 cards)                        │
│                                                                  │
│     ┌───────────┐ ┌───────────┐ ┌───────────┐                   │
│     │Determin-  │ │ Hybrid    │ │ Session   │                   │
│     │istic      │ │ Search    │ │ Memory    │                   │
│     │100% same  │ │Semantic + │ │Multi-turn │                   │
│     │results    │ │Lexical    │ │tracking   │                   │
│     └───────────┘ └───────────┘ └───────────┘                   │
│     ┌───────────┐ ┌───────────┐ ┌───────────┐                   │
│     │ Token     │ │ Line-level│ │ Pure Rust │                   │
│     │ Efficient │ │ Citations │ │ Embeddings│                   │
│     │95%+ util  │ │Verifiable │ │6x faster  │                   │
│     └───────────┘ └───────────┘ └───────────┘                   │
│                                                                  │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│                    PERFORMANCE METRICS                           │
│                                                                  │
│     ┌──────────────────────────────────────────────────┐        │
│     │              vs OpenAI Embeddings                 │        │
│     │  ┌────────────────────────────────────────────┐  │        │
│     │  │ Query Embed │ ████ 3ms  vs ████████ 250ms  │  │        │
│     │  │ Compilation │ ████ 50ms vs ████████ 300ms  │  │        │
│     │  │ Cost/month  │ $0        vs ~$100           │  │        │
│     │  └────────────────────────────────────────────┘  │        │
│     └──────────────────────────────────────────────────┘        │
│                                                                  │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│                    HOW IT WORKS                                  │
│                                                                  │
│     1. INGEST        2. INDEX         3. COMPILE                │
│     ┌────────┐      ┌────────┐       ┌────────┐                 │
│     │ 📄→📄→📄│  →   │ 🔍 HNSW│   →   │ ✨ LLM │                 │
│     │Documents│      │ Vector │       │ Context│                 │
│     │→ Spans  │      │ Index  │       │ Window │                 │
│     └────────┘      └────────┘       └────────┘                 │
│                                                                  │
│     Query → Embed → Semantic + Lexical → Hybrid Fusion          │
│     → MMR Diversity → Token Pack → Deterministic Sort            │
│                                                                  │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│                    CODE EXAMPLES (tabs)                          │
│                                                                  │
│     [CLI] [Python] [TypeScript] [Rust] [HTTP API]               │
│                                                                  │
│     ┌──────────────────────────────────────────────┐            │
│     │ # CLI                                         │            │
│     │ cargo install avocado-cli                     │            │
│     │ avocado init                                  │            │
│     │ avocado ingest ./docs --recursive             │            │
│     │ avocado compile "How does auth work?"         │            │
│     └──────────────────────────────────────────────┘            │
│                                                                  │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│                    INTEGRATIONS                                  │
│                                                                  │
│     [LangChain] [LlamaIndex] [Docker] [K8s]                     │
│     [Python]    [TypeScript] [Rust]   [REST]                    │
│                                                                  │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│                    USE CASES (carousel)                          │
│                                                                  │
│     "Code Documentation" → "Support Q&A" → "Agent Memory"       │
│                                                                  │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│                    QUICK START CTA                               │
│                                                                  │
│     Ready to fix your RAG?                                       │
│                                                                  │
│     cargo install avocado-cli                                    │
│                                                                  │
│     [Get Started]  [Read Docs]  [GitHub ⭐]                      │
│                                                                  │
├─────────────────────────────────────────────────────────────────┤
│                    FOOTER                                        │
│  MIT License | GitHub | Docs | crates.io | Discord               │
└─────────────────────────────────────────────────────────────────┘
```

---

## COPY / CONTENT

### Hero Section

**Headline:**
```
The Deterministic Context Database for AI Agents
```

**Subheadline:**
```
Fix your RAG in 5 minutes. Same query → same context, every time.
```

**Value Pills:**
| Metric | Value | Description |
|--------|-------|-------------|
| Speed | 6x Faster | 40-60ms vs 300ms |
| Determinism | 100% | Same hash every time |
| Cost | $0 | Pure Rust, no API |

**CTA Buttons:**
- Primary: "Get Started" → #quick-start
- Secondary: "View on GitHub ⭐" → github.com/avocadodb/avocadodb

**Install Command (copyable):**
```bash
cargo install avocado-cli
```

---

### Problem Section

**Headline:**
```
Why Your RAG System is Broken
```

**Before/After Comparison:**

| Problem (Before) | Solution (After) |
|-----------------|------------------|
| ❌ Same query, different results | ✅ 100% deterministic |
| ❌ 60-70% token utilization | ✅ 95%+ token utilization |
| ❌ No citations or sources | ✅ Line-level citations |
| ❌ 200-300ms embedding latency | ✅ 1-5ms local embeddings |
| ❌ API costs add up | ✅ Free, works offline |
| ❌ Debugging impossible | ✅ SHA-256 hash verification |

**Supporting Text:**
```
Traditional vector databases use approximate nearest neighbor search
with non-deterministic tie-breaking. Same query returns different
context each time. AvocadoDB fixes this with deterministic ordering
and hybrid retrieval.
```

---

### Demo Section

**Terminal Animation Script:**
```bash
# Frame 1: Install
$ cargo install avocado-cli
✓ Installed avocado v2.0.0

# Frame 2: Initialize
$ avocado init
✓ Created database at .avocado/db.sqlite

# Frame 3: Ingest
$ avocado ingest ./docs --recursive
Ingesting 42 files...
✓ Extracted 387 spans
✓ Generated embeddings (local)
✓ Indexed in 2.3s

# Frame 4: Compile (first time)
$ avocado compile "How does authentication work?"
[1] docs/auth.md:1-23
# Authentication System
Our system uses JWT tokens...

Compiled 12 spans | 7,891 tokens | 98.6% utilization
Hash: e3b0c44298fc1c14... ✓

# Frame 5: Compile (second time - SAME HASH)
$ avocado compile "How does authentication work?"
[1] docs/auth.md:1-23
# Authentication System
Our system uses JWT tokens...

Compiled 12 spans | 7,891 tokens | 98.6% utilization
Hash: e3b0c44298fc1c14... ✓  ← SAME!
```

**Caption:**
```
Same query. Same context. Same hash. Every single time.
```

---

### Features Section (6 Cards)

**Card 1: 100% Deterministic**
```
Icon: 🎯
Title: 100% Deterministic
Description: Same query returns identical context every time.
Verified across 100+ runs with SHA-256 hash matching.
```

**Card 2: Hybrid Search**
```
Icon: 🔍
Title: Hybrid Retrieval
Description: Combines semantic (vector) and lexical (keyword) search.
Best of both worlds with configurable weights.
```

**Card 3: Session Memory**
```
Icon: 💬
Title: Session Management
Description: Multi-turn conversation tracking for AI agents.
Track queries, responses, and compiled context.
```

**Card 4: Token Efficient**
```
Icon: 📊
Title: 95%+ Token Utilization
Description: Greedy packing maximizes your context window.
40% better than traditional RAG (60-70%).
```

**Card 5: Line-Level Citations**
```
Icon: 📍
Title: Verifiable Citations
Description: Every span includes exact file path and line numbers.
Trust your AI's sources.
```

**Card 6: Pure Rust Embeddings**
```
Icon: ⚡
Title: 6x Faster Embeddings
Description: 1-5ms local embeddings vs 200-300ms API calls.
No API key required. Works offline.
```

---

### Performance Section

**Headline:**
```
Built for Speed. Designed for Trust.
```

**Metrics Bar Chart Data:**

| Metric | AvocadoDB | OpenAI/Traditional |
|--------|-----------|-------------------|
| Query Embedding | 3ms | 250ms |
| Total Compilation | 50ms | 300ms |
| Token Utilization | 95% | 65% |
| Monthly Cost | $0 | ~$100 |

**Callout Stats:**
- **6x** faster compilation
- **100%** deterministic
- **95%+** token utilization
- **0** API costs

---

### How It Works Section

**Pipeline Diagram:**

```
┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│   INGEST    │ → │    INDEX    │ → │   COMPILE   │
│             │    │             │    │             │
│ Documents   │    │ HNSW Vector │    │ LLM-Ready   │
│ → Spans     │    │ + SQLite    │    │ Context     │
│ (20-50 lines)│   │             │    │             │
└─────────────┘    └─────────────┘    └─────────────┘
```

**Pipeline Steps:**
1. **Query** → Embed with local model (1-5ms)
2. **Search** → Semantic + Lexical hybrid
3. **Fuse** → Reciprocal Rank Fusion scoring
4. **Diversify** → MMR removes redundancy
5. **Pack** → Greedy token budget filling
6. **Sort** → Deterministic by (artifact_id, line)
7. **Output** → Working set with citations

---

### Code Examples Section (Tabbed)

**Tab 1: CLI**
```bash
# Install
cargo install avocado-cli

# Initialize
avocado init

# Ingest your docs
avocado ingest ./docs --recursive

# Compile context
avocado compile "How does authentication work?" --budget 8000
```

**Tab 2: Python**
```python
from avocado import AvocadoDB

db = AvocadoDB()
db.ingest("./docs", recursive=True)

result = db.compile("How does auth work?", budget=8000)
print(result.text)       # Deterministic context
print(result.citations)  # Line-level sources
```

**Tab 3: TypeScript**
```typescript
import { AvocadoDB } from 'avocadodb';

const db = new AvocadoDB();
await db.ingest('./docs', { recursive: true });

const result = await db.compile('How does auth work?', { budget: 8000 });
console.log(result.text);      // Deterministic context
console.log(result.citations); // Line-level sources
```

**Tab 4: Rust**
```rust
use avocado_core::{Database, compiler, CompilerConfig};

let db = Database::new(".avocado/db.sqlite")?;
let config = CompilerConfig { token_budget: 8000, ..Default::default() };

let result = compiler::compile("How does auth work?", config, &db).await?;
println!("{}", result.text);
println!("Hash: {}", result.deterministic_hash());
```

**Tab 5: HTTP API**
```bash
# Start server
avocado-server

# Compile via API
curl -X POST http://localhost:8765/compile \
  -H "Content-Type: application/json" \
  -d '{"query": "authentication", "token_budget": 8000}'
```

---

### Integrations Section

**Framework Logos (with links):**
- LangChain → `pip install langchain-avocadodb`
- LlamaIndex → `pip install llama-index-avocadodb`
- Docker → `docker pull avocadodb/avocadodb`
- Kubernetes → `kubectl apply -k k8s/`

**SDK Badges:**
- Python SDK
- TypeScript SDK
- Rust Crate
- REST API

---

### Use Cases Section (Carousel)

**Use Case 1: Code Documentation Assistant**
```
Help developers understand large codebases with deterministic answers.
Perfect for onboarding and code exploration.
```

**Use Case 2: Technical Support Q&A**
```
Build support systems with verifiable, citation-backed answers.
Reduce ticket resolution time with trustworthy context.
```

**Use Case 3: AI Agent Memory**
```
Session management for multi-turn conversations.
Track context, replay sessions, debug agent behavior.
```

**Use Case 4: Research & Knowledge Base**
```
Search technical documentation and research papers.
Line-level citations for academic rigor.
```

---

### Final CTA Section

**Headline:**
```
Ready to Fix Your RAG?
```

**Subheadline:**
```
Get deterministic context in under 5 minutes.
```

**Install Command:**
```bash
cargo install avocado-cli
```

**Buttons:**
- [Get Started] → Quick Start docs
- [Read Documentation] → docs.rs/avocado-core
- [Star on GitHub ⭐] → github.com/avocadodb/avocadodb

---

### Footer

**Links:**
- Documentation
- GitHub
- crates.io
- Discord (if applicable)

**Legal:**
- MIT License
- Privacy Policy
- Terms of Service

**Copyright:**
```
© 2025 AvocadoDB. Making retrieval deterministic, one context at a time.
```

---

## DESIGN RECOMMENDATIONS

### Color Palette
- **Primary**: #4CAF50 (Avocado green)
- **Secondary**: #2E7D32 (Dark green)
- **Accent**: #8BC34A (Light green)
- **Background**: #FAFAFA (Light) / #1A1A2E (Dark mode)
- **Text**: #212121 (Dark) / #FFFFFF (Light on dark)

### Typography
- **Headlines**: Inter Bold or similar sans-serif
- **Body**: Inter Regular
- **Code**: JetBrains Mono or Fira Code

### Visual Elements
- Terminal animations for demo
- Bar charts for performance comparison
- Pipeline diagram for "How it Works"
- Code syntax highlighting
- Copy buttons on code blocks

### Mobile Responsive
- Stack feature cards vertically
- Collapse code tabs to accordion
- Sticky CTA button at bottom

### Animations
- Fade-in on scroll for sections
- Terminal typing animation for demo
- Counter animation for metrics
- Smooth tab transitions

---

## SEO METADATA

**Title:**
```
AvocadoDB - Deterministic Context Database for AI Agents
```

**Description:**
```
Fix your RAG in 5 minutes. AvocadoDB provides 100% deterministic context
retrieval for AI agents. 6x faster than OpenAI, works offline, costs $0.
Same query → same context, every time.
```

**Keywords:**
```
RAG, vector database, AI agents, deterministic, embeddings, LLM,
context retrieval, semantic search, Rust, LangChain, LlamaIndex
```

**Open Graph:**
```
og:title: AvocadoDB - Deterministic RAG for AI Agents
og:description: Same query → same context, every time. 6x faster, $0 cost.
og:image: [avocado logo/hero image]
og:url: https://avocadodb.ai
```

---

## TRACKING & ANALYTICS

**Key Events to Track:**
- Install command copied
- GitHub link clicked
- Docs link clicked
- Code tab switched
- Scroll depth
- Time on page

**Conversion Goals:**
- GitHub star (primary)
- `cargo install` copy (primary)
- Documentation visit
- crates.io visit

---

This spec is ready for your Manus agent to build the landing page!
