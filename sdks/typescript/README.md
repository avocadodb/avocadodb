# AvocadoDB TypeScript SDK

**Framework-agnostic SDK for AvocadoDB** - the deterministic context database for AI agents.

## 🚀 What's New in v2.0

The SDK has been completely refactored to be **framework-agnostic**, moving all intelligence from framework-specific code into reusable SDK primitives.

### New Features

- ✅ **Local LLM Support**: Optional TinyLlama integration for natural language answers
- ✅ **Server Lifecycle Management**: Auto-start, health checks, daemon mode
- ✅ **Background File Monitoring**: Auto-detect and re-ingest changed files
- ✅ **Smart Auto-Ingest**: Project type detection (Python/Node/Rust/Go/etc.)
- ✅ **Framework Integrations**: LangChain.js support
- ✅ **Utilities**: Token counting, citation formatting, prompt generation
- ✅ **100% Backward Compatible**: All v1.0 APIs still work

## 📦 Installation

```bash
npm install avocadodb
```

For LangChain integration:
```bash
npm install avocadodb @langchain/core zod
```

## 🎯 Quick Start

### Basic Usage (HTTP Client)

```typescript
import { AvocadoDB } from 'avocadodb';

// Connect to server
const db = new AvocadoDB('http://localhost:8765');

// Compile context
const result = await db.compile('How does authentication work?');

// Use the context
console.log(`Context (${result.tokensUsed} tokens):`);
console.log(result.text);

// Show citations
for (const citation of result.citations) {
  console.log(`  - ${citation.artifactPath}:${citation.startLine}-${citation.endLine}`);
}
```

### Ask Questions (v2.0 - New!)

Get natural language answers using TinyLlama:

```typescript
import { AvocadoDB } from 'avocadodb';

const db = new AvocadoDB('http://localhost:8765');

// Ask a question - uses TinyLlama if available
const answer = await db.ask('How does authentication work?');
console.log(answer);

// Options:
// - llm: 'auto' (default) - Try TinyLlama, fallback to context
// - llm: 'local' - Require TinyLlama (throws error if not available)
// - llm: 'none' - Just return context (same as compile())
```

**Note**: Requires Python SDK with LLM support installed: `pip install avocadodb[llm]`

### With Auto-Management

```typescript
import { getManager, AutoIngest } from 'avocadodb';

// Auto-start server (daemon mode)
const manager = getManager();
await manager.ensureRunning();

// Auto-ingest project (detects Python/Node/Rust/etc.)
const ingester = new AutoIngest();
const result = await ingester.ingestProject('.', { maxFiles: 100 });
console.log(`Ingested ${result.ingested} files (${result.projectType} project)`);

// Compile context
import { AvocadoDB } from 'avocadodb';
const db = new AvocadoDB();
const context = await db.compile('How does the authentication module work?');
```

### Background File Monitoring

```typescript
import { FileMonitor } from 'avocadodb';

// Monitor files and auto-re-ingest on changes
const monitor = new FileMonitor({ intervalSeconds: 30 });
monitor.startMonitoring([
  'docs/**/*.md',
  'src/**/*.ts',
  'README.md'
]);

// Optional: callback for change events
monitor.onChange((files) => {
  console.log(`Re-ingested ${files.length} changed files`);
});

// Monitor runs in background
// Stop when done: monitor.stopMonitoring();
```

## 🔧 Framework Integrations

### LangChain.js / LangGraph.js

```typescript
import { avocadoCompileContext } from 'avocadodb/integrations/langchain';
import { ChatAnthropic } from '@langchain/anthropic';
import { AgentExecutor, createToolCallingAgent } from 'langchain/agents';

const tools = [avocadoCompileContext];  // Auto-start, auto-ingest built-in!
const llm = new ChatAnthropic({ model: 'claude-3-5-sonnet-20241022' });
const agent = await createToolCallingAgent({ llm, tools, prompt });
const executor = new AgentExecutor({ agent, tools });

const result = await executor.invoke({
  input: 'How does authentication work in this codebase?'
});

// The agent will automatically:
// 1. Start AvocadoDB server (daemon mode on port 8765)
// 2. Auto-ingest current directory on first query
// 3. Use AvocadoDB exclusively for codebase questions
```

## 📚 API Reference

### Core Client

#### `AvocadoDB(url?: string)`

HTTP client for AvocadoDB server.

**Methods:**
- `compile(query, options?)` → `Promise<WorkingSet>`
- `ingest(path, content?)` → `Promise<IngestResult>`
- `stats()` → `Promise<Stats>`
- `ping()` → `Promise<boolean>`

**Example:**
```typescript
const db = new AvocadoDB('http://localhost:8765');
const result = await db.compile('authentication', { budget: 8000 });
console.log(`Compiled ${result.spans.length} spans`);
```

### Server Management

#### `AvocadoDBManager({ autoStart?, port? })`

Manages server lifecycle (auto-start, health checks, daemon mode).

**Methods:**
- `ensureRunning()` → `Promise<boolean>` - Start server if not running
- `isRunning()` → `Promise<boolean>` - Check if server is reachable
- `startServer()` → `Promise<boolean>` - Start server as daemon
- `getStats()` → `Promise<ServerStats | null>` - Get database statistics
- `healthCheck()` → `Promise<HealthCheck>` - Comprehensive health check

**Example:**
```typescript
const manager = new AvocadoDBManager({ autoStart: true, port: 8765 });
if (await manager.ensureRunning()) {
  const stats = await manager.getStats();
  console.log(`Indexed: ${stats.artifactsCount} docs`);
}
```

#### `getManager()` → `AvocadoDBManager`

Get global manager instance (singleton pattern).

### Auto-Ingest

#### `AutoIngest({ ingestBinary? })`

Smart auto-ingestion with project type detection.

**Methods:**
- `detectProjectType(path)` → `ProjectType` - Detect Python/Node/Rust/etc.
- `getPatternsForProject(projectType)` → `string[]` - Get file patterns
- `ingestProject(path, options?)` → `Promise<IngestProjectResult>` - Ingest entire project
- `ingestFile(path)` → `Promise<boolean>` - Ingest single file

**Example:**
```typescript
const ingester = new AutoIngest();

// Auto-detect and ingest
const result = await ingester.ingestProject('.', { maxFiles: 100 });
console.log(`Ingested ${result.ingested} files`);
console.log(`Project type: ${result.projectType}`);
```

### File Monitoring

#### `FileMonitor({ intervalSeconds?, ingestBinary? })`

Background file watcher for automatic re-ingestion.

**Methods:**
- `startMonitoring(patterns, cwd?)` - Start background monitoring
- `stopMonitoring()` - Stop monitoring
- `onChange(callback)` - Register change event handler
- `isMonitoring()` → `boolean` - Check if monitoring is active

**Example:**
```typescript
const monitor = new FileMonitor({ intervalSeconds: 30 });
monitor.startMonitoring(['docs/**/*.md', 'src/**/*.ts']);

monitor.onChange((files) => {
  console.log(`Changed: ${files.map(f => path.basename(f)).join(', ')}`);
});
```

### Utilities

#### `countTokens(text)` → `number`

Approximate token count (~4 characters per token).

#### `formatCitations(citations, style?)` → `string`

Format citations for display.

**Styles:** `'compact'`, `'verbose'`, `'markdown'`

#### `createSystemPrompt(options?)` → `string`

Generate AvocadoDB-first system prompt.

#### `formatWorkingSet(workingSet, includeContext?)` → `string`

Format WorkingSet for human-readable display.

## 🔄 Migration from v1.0

### What Changed

**v1.0** (Thin HTTP wrapper):
```typescript
import { AvocadoDB } from 'avocadodb';
const db = new AvocadoDB();
// Manual server management required
```

**v2.0** (Full-featured SDK):
```typescript
import { getManager, AvocadoDB } from 'avocadodb';

// Auto-start server
const manager = getManager();
await manager.ensureRunning();

// Use client
const db = new AvocadoDB();
```

### Backward Compatibility

**All v1.0 APIs still work!** The new features are additive:

```typescript
// v1.0 code (still works in v2.0)
import { AvocadoDB } from 'avocadodb';
const db = new AvocadoDB();
const result = await db.compile('query');

// v2.0 code (new features)
import { getManager, AutoIngest, FileMonitor } from 'avocadodb';
const manager = getManager();  // Auto-start
const ingester = new AutoIngest();  // Auto-ingest
const monitor = new FileMonitor();  // Auto-monitor
```

## 🌐 Environment Variables

- `AVOCADODB_URL` - Server URL (default: `http://localhost:8765`)
- `AVOCADODB_AUTO_START` - Enable auto-start (default: `true`)

## 📖 Examples

See `examples/` directory for:
- Basic HTTP client usage
- Auto-management with lifecycle
- Background monitoring
- LangChain.js integration

## 🤝 Contributing

Contributions welcome! See [CONTRIBUTING.md](../../CONTRIBUTING.md)

## 📄 License

MIT License - see [LICENSE](../../LICENSE)

## 🔗 Links

- [GitHub](https://github.com/servesys-labs/avacadodb)
- [Documentation](https://github.com/servesys-labs/avacadodb/tree/main/docs)
- [Python SDK](../python)
