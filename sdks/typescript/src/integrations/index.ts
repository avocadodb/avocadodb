/**
 * Framework integrations for AvocadoDB
 *
 * Provides ready-to-use integrations for popular agent frameworks:
 * - LangChain.js / LangGraph.js
 *
 * Each integration provides:
 * - Tool wrapper (avocado_compile_context)
 * - Framework-specific configuration
 *
 * @example
 * ```typescript
 * // LangChain
 * import { avocadoCompileContext } from 'avocadodb/integrations/langchain';
 * ```
 *
 * @packageDocumentation
 */

export * from './langchain';
