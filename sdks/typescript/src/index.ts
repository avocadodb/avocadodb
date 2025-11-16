/**
 * AvocadoDB TypeScript SDK
 *
 * Dead simple HTTP client for AvocadoDB - the deterministic context database.
 *
 * @example
 * ```typescript
 * import { AvocadoDB } from 'avocadodb';
 *
 * const db = new AvocadoDB('http://localhost:8080');
 *
 * // Compile context
 * const result = await db.compile('How does authentication work?', {
 *   budget: 8000
 * });
 *
 * console.log(`Compiled ${result.spans.length} spans`);
 * console.log(`Hash: ${result.deterministicHash()}`);
 * console.log(result.text);
 * ```
 *
 * @packageDocumentation
 */

export {
  AvocadoDB,
  WorkingSet,
  Span,
  Citation,
  CompileOptions,
  Stats,
  IngestResult,
} from './client';
