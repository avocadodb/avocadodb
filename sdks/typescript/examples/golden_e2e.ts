#!/usr/bin/env ts-node
/**
 * Golden E2E (TypeScript) — ingest (optional), compile, session create/compile/history/replay.
 *
 * Prereqs:
 *   - AvocadoDB server running at http://127.0.0.1:8765
 *   - Current working directory is the project root you want to query (project = process.cwd())
 *
 * Run:
 *   cd sdks/typescript
 *   npm install && npm run build
 *   ts-node examples/golden_e2e.ts
 */
import { AvocadoDB, SessionManager } from '../src';

async function main() {
  const baseUrl = 'http://127.0.0.1:8765';
  const project = process.cwd();
  const db = new AvocadoDB(baseUrl);

  // Health check
  const reachable = await db.ping();
  if (!reachable) {
    console.error('Server not reachable at', baseUrl);
    process.exit(1);
  }

  // Stats
  const stats = await db.stats();
  console.log('Stats:', stats);
  if (stats.spans === 0) {
    console.log('No spans found. Ingest your project before running this example.');
    process.exit(0);
  }

  // Compile a query
  const query = 'What is Orion and how does it help APX?';
  const ws = await db.compile(query, { budget: 4000 });
  console.log('\nCompile summary:\n' + ws.summary());

  // Session flow
  const sm = new SessionManager(baseUrl, project);
  const session = await sm.createSession({ userId: 'golden-ts', title: 'Golden E2E (TS)' });
  console.log('\nSession created:', session.toString());

  const turn1 = await session.compile(query, { budget: 4000 });
  console.log('\nSession compile spans:', turn1.workingSet.spans.length);

  await session.addMessage('assistant', 'Context compiled. Proceeding.');
  const history = await session.getHistory(1500);
  console.log('\nHistory (truncated):\n', history.substring(0, 400), '...');

  const replay = await session.replay();
  console.log('\nReplay turns:', replay.turns.length);

  console.log('\nDone.');
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});


