/**
 * AvocadoDB TypeScript SDK
 *
 * Framework-agnostic SDK for AvocadoDB - the deterministic context database.
 *
 * Features:
 * - HTTP client for compile/ingest/stats operations
 * - Server lifecycle management (auto-start, daemon mode)
 * - Background file monitoring and re-ingestion
 * - Smart auto-ingest with project type detection
 * - Framework integrations (LangChain.js)
 * - Utility functions (token counting, citation formatting)
 *
 * @example
 * ```typescript
 * // Basic usage (HTTP client)
 * import { AvocadoDB } from 'avocadodb';
 *
 * const db = new AvocadoDB('http://localhost:8765');
 * const result = await db.compile('How does authentication work?');
 * console.log(result.text);
 *
 * // With auto-management
 * import { getManager, AutoIngest } from 'avocadodb';
 *
 * const manager = getManager();  // Auto-starts server
 * await manager.ensureRunning();
 *
 * const ingester = new AutoIngest();
 * await ingester.ingestProject('.');  // Auto-detects project type
 *
 * // Framework integrations
 * import { avocadoCompileContext } from 'avocadodb/integrations/langchain';
 * ```
 *
 * @packageDocumentation
 */

// Core client
export {
  AvocadoDB,
  WorkingSet,
  Span,
  Citation,
  CompileOptions,
  Stats,
  IngestResult,
} from './client';

// Server lifecycle management
export {
  AvocadoDBManager,
  getManager,
  HealthCheck,
  ServerStats,
} from './manager';

// Background monitoring
export { FileMonitor, OnChangeCallback } from './monitor';

// Smart auto-ingest
export { AutoIngest, ProjectType, IngestProjectResult } from './ingest';

// Utilities
export {
  countTokens,
  formatCitations,
  createSystemPrompt,
  formatWorkingSet,
  CitationStyle,
} from './utils';
