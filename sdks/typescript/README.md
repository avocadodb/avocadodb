# AvocadoDB TypeScript SDK

**Simple HTTP client for AvocadoDB - the deterministic context database.**

## Installation

```bash
npm install avocadodb
```

Or with Yarn:

```bash
yarn add avocadodb
```

Or from source:

```bash
cd typescript
npm install
npm run build
```

## Quick Start

### 1. Start Server & Ingest Data

```bash
# Start the server
./target/release/avocado-server &

# Ingest documents
./target/release/avocado ingest test-docs/ --recursive
```

### 2. Use TypeScript SDK

```typescript
import { AvocadoDB } from 'avocadodb';

// Connect to server
const db = new AvocadoDB('http://localhost:8080');

// Compile context
const result = await db.compile('How does authentication work?', {
  budget: 8000
});

console.log(`Compiled ${result.spans.length} spans`);
console.log(`Used ${result.tokensUsed} tokens`);
console.log(`Hash: ${result.deterministicHash()}`);
console.log(result.text);
```

## Features

- ✅ **Deterministic**: Same query → same context, every time
- ✅ **Citation-backed**: Every span has exact line numbers
- ✅ **Token efficient**: 90-95% budget utilization
- ✅ **Fast**: < 500ms for 8K token context
- ✅ **Simple**: Just HTTP requests, no complex dependencies
- ✅ **TypeScript**: Full type definitions included

## API Reference

### `new AvocadoDB(url?)`

Create client connection.

**Parameters:**
- `url` (string, optional): Server URL (default: `http://localhost:8080`)

**Example:**
```typescript
const db = new AvocadoDB('http://localhost:8080');
```

### `db.compile(query, options?)`

Compile deterministic context.

**Parameters:**
- `query` (string): Search query
- `options` (CompileOptions, optional):
  - `budget` (number): Token budget (default: 8000)
  - `semanticWeight` (number): Semantic weight (default: 0.7)
  - `lexicalWeight` (number): Lexical weight (default: 0.3)
  - `mmrLambda` (number): Diversity 0.0-1.0 (default: 0.5)
  - `enableMmr` (boolean): Enable MMR (default: true)

**Returns:** `Promise<WorkingSet>`

**Example:**
```typescript
const result = await db.compile('authentication', {
  budget: 8000,
  mmrLambda: 0.7
});
```

### `db.ingest(path, content?)`

Ingest document.

**Parameters:**
- `path` (string): Document path
- `content` (string, optional): Content (reads from file if not provided)

**Returns:** `Promise<IngestResult>`

**Example:**
```typescript
const result = await db.ingest('docs/guide.md');
console.log(`Created ${result.spanCount} spans`);
```

### `db.stats()`

Get database statistics.

**Returns:** `Promise<Stats>`

**Example:**
```typescript
const stats = await db.stats();
console.log(`Artifacts: ${stats.artifacts}`);
console.log(`Spans: ${stats.spans}`);
console.log(`Tokens: ${stats.tokens}`);
```

### `db.ping()`

Check if server is reachable.

**Returns:** `Promise<boolean>`

**Example:**
```typescript
if (await db.ping()) {
  console.log('Server is online');
}
```

## WorkingSet Class

**Properties:**
- `text` (string): Compiled context
- `spans` (Span[]): Included spans
- `citations` (Citation[]): Citations
- `tokensUsed` (number): Tokens used
- `query` (string): Original query
- `compilationTimeMs` (number): Compilation time

**Methods:**

#### `deterministicHash(): string`

Calculate SHA-256 hash of context.

**Example:**
```typescript
const hash = result.deterministicHash();
console.log(`Hash: ${hash}`);
```

#### `summary(): string`

Get human-readable summary.

**Example:**
```typescript
console.log(result.summary());
// Output:
// Query: authentication
// Spans: 12
// Tokens: 7843
// Time: 287ms
// Hash: b08193f7acf79cf...
```

## Type Definitions

### Span

```typescript
interface Span {
  artifactId: string;
  artifactPath: string;
  startLine: number;
  endLine: number;
  text: string;
  tokenCount: number;
  score: number;
}
```

### Citation

```typescript
interface Citation {
  artifactPath: string;
  startLine: number;
  endLine: number;
}
```

### Stats

```typescript
interface Stats {
  artifacts: number;
  spans: number;
  tokens: number;
}
```

## Examples

### Verify Determinism

```typescript
import { AvocadoDB } from 'avocadodb';

const db = new AvocadoDB();

// Run same query 3 times
const hashes: string[] = [];
for (let i = 0; i < 3; i++) {
  const result = await db.compile('authentication');
  hashes.push(result.deterministicHash());
}

// All hashes identical
if (hashes[0] === hashes[1] && hashes[1] === hashes[2]) {
  console.log('✅ Deterministic!');
}
```

### Tune Parameters

```typescript
// More diverse results
const result1 = await db.compile('query', { mmrLambda: 0.3 });

// More keyword matching
const result2 = await db.compile('query', { lexicalWeight: 0.5 });

// Large context
const result3 = await db.compile('query', { budget: 16000 });
```

### Use with OpenAI

```typescript
import { AvocadoDB } from 'avocadodb';
import OpenAI from 'openai';

const db = new AvocadoDB();
const openai = new OpenAI();

// Compile context
const context = await db.compile('How does authentication work?', {
  budget: 8000
});

// Use with GPT-4
const completion = await openai.chat.completions.create({
  model: 'gpt-4',
  messages: [
    {
      role: 'system',
      content: 'Answer questions using only the provided context.'
    },
    {
      role: 'user',
      content: `Context:\n${context.text}\n\nQuestion: How does authentication work?`
    }
  ]
});

console.log(completion.choices[0].message.content);

// Show citations
context.citations.forEach((citation, i) => {
  console.log(
    `[${i + 1}] ${citation.artifactPath}:${citation.startLine}-${citation.endLine}`
  );
});
```

### Error Handling

```typescript
import { AvocadoDB } from 'avocadodb';

const db = new AvocadoDB();

try {
  // Check server health
  if (!(await db.ping())) {
    console.error('Server is not reachable');
    process.exit(1);
  }

  // Compile context
  const result = await db.compile('query', { budget: 8000 });
  console.log(result.text);
} catch (error) {
  console.error('Error:', error);
}
```

### Ingest from Memory

```typescript
const content = `
# My Document

This is some content I want to ingest.
`;

const result = await db.ingest('my-doc.md', content);
console.log(`Created artifact: ${result.artifactId}`);
```

See `example.ts` for full demonstration.

## Requirements

- Node.js 14+
- Running AvocadoDB server

## Development

```bash
# Install dependencies
npm install

# Build TypeScript
npm run build

# Run example
npm run example
```

## License

MIT License
