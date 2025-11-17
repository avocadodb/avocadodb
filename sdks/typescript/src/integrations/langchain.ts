/**
 * LangChain.js / LangGraph.js integration for AvocadoDB
 *
 * Provides:
 * - avocadoCompileContext: LangChain tool function with auto-start
 * - AvocadoDBTool: DynamicStructuredTool wrapper
 *
 * @example
 * ```typescript
 * import { avocadoCompileContext } from 'avocadodb/integrations/langchain';
 * import { ChatAnthropic } from '@langchain/anthropic';
 * import { AgentExecutor, createToolCallingAgent } from 'langchain/agents';
 *
 * const tools = [avocadoCompileContext];
 * const llm = new ChatAnthropic({ model: 'claude-3-5-sonnet-20241022' });
 * const agent = await createToolCallingAgent({ llm, tools, prompt });
 * const executor = new AgentExecutor({ agent, tools });
 * ```
 *
 * @packageDocumentation
 */

import { DynamicStructuredTool } from '@langchain/core/tools';
import { z } from 'zod';
import { AvocadoDB } from '../client';
import { getManager } from '../manager';
import { AutoIngest } from '../ingest';

/**
 * Tool input schema for AvocadoDB
 */
const AvocadoDBSchema = z.object({
  query: z.string().describe('Search query describing what information you need'),
  tokenBudget: z.number().optional().describe('Maximum tokens to use (default: 8000)'),
  semanticWeight: z.number().optional().describe('Weight for semantic search 0.0-1.0 (default: 0.7)'),
  lexicalWeight: z.number().optional().describe('Weight for lexical search 0.0-1.0 (default: 0.3)'),
  mmrLambda: z.number().optional().describe('Diversity parameter 0.0-1.0 (default: 0.5)'),
  enableMmr: z.boolean().optional().describe('Enable diversification (default: true)'),
});

/**
 * Tool result type
 */
interface ToolResult {
  success: boolean;
  context?: string;
  citations?: Array<{ file: string; lines: string }>;
  spans?: number;
  tokensUsed?: number;
  compilationTimeMs?: number;
  deterministicHash?: string;
  query?: string;
  error?: string;
  hint?: string;
}

/**
 * LangChain tool for deterministic, citation-backed context retrieval
 *
 * AvocadoDB provides 100% deterministic context compilation - the same query
 * always returns the same context, making responses reproducible and auditable.
 *
 * Features:
 * - Auto-start server on first use
 * - Auto-ingest project if empty
 * - Comprehensive error handling
 *
 * @example
 * ```typescript
 * import { avocadoCompileContext } from 'avocadodb/integrations/langchain';
 * import { ChatAnthropic } from '@langchain/anthropic';
 * import { createToolCallingAgent } from 'langchain/agents';
 *
 * const tools = [avocadoCompileContext];
 * const agent = await createToolCallingAgent({
 *   llm: new ChatAnthropic({ model: 'claude-3-5-sonnet-20241022' }),
 *   tools,
 *   prompt
 * });
 * ```
 */
export const avocadoCompileContext = new DynamicStructuredTool({
  name: 'avocado_compile_context',
  description: `PRIMARY TOOL: Use this FIRST for any questions about the codebase or documentation.

AvocadoDB provides deterministic, citation-backed context compilation - the same query
always returns the same context, making responses reproducible and auditable.

WHEN TO USE (DEFAULT for codebase questions):
- ANY question about the codebase, documentation, or project
- Questions like "what is this project", "how does X work", "explain Y"
- Searching for implementations, patterns, or architecture
- Understanding features, APIs, or configurations

PREFER THIS OVER grep/read_file - it provides semantic search with citations.
Only use filesystem tools if this fails or for editing files.

This tool searches your ingested codebase/documentation and returns relevant
context that you MUST synthesize into a natural response for the user.`,

  schema: AvocadoDBSchema,

  func: async ({
    query,
    tokenBudget = 8000,
    semanticWeight = 0.7,
    lexicalWeight = 0.3,
    mmrLambda = 0.5,
    enableMmr = true,
  }): Promise<string> => {
    try {
      // Auto-start server if configured
      const manager = getManager();
      await manager.ensureRunning();

      // Auto-ingest if needed (first-time setup)
      const stats = await manager.getStats();
      if (stats && stats.artifactsCount === 0) {
        console.log('🥑 First-time setup: Auto-ingesting current directory...');
        const ingester = new AutoIngest();
        await ingester.ingestProject('.', { maxFiles: 100 });
      }

      // Get server URL from environment or use default
      const serverUrl = process.env.AVOCADODB_URL || 'http://localhost:8765';

      // Use AvocadoDB client
      const client = new AvocadoDB(serverUrl);
      const workingSet = await client.compile(query, {
        budget: tokenBudget,
        semanticWeight,
        lexicalWeight,
        mmrLambda,
        enableMmr,
      });

      // Format citations for easy reference
      const formattedCitations = workingSet.citations.map((c) => ({
        file: c.artifactPath,
        lines: `${c.startLine}-${c.endLine}`,
      }));

      // Show token usage stats
      console.log(
        `📊 AvocadoDB: ${workingSet.tokensUsed.toLocaleString()} tokens used (budget: ${tokenBudget.toLocaleString()}) | ${
          workingSet.spans.length
        } spans | ${workingSet.compilationTimeMs}ms`
      );

      const result: ToolResult = {
        success: true,
        context: workingSet.text,
        citations: formattedCitations,
        spans: workingSet.spans.length,
        tokensUsed: workingSet.tokensUsed,
        compilationTimeMs: workingSet.compilationTimeMs,
        deterministicHash: workingSet.deterministicHash(),
        query: workingSet.query,
      };

      return JSON.stringify(result, null, 2);
    } catch (error) {
      const result: ToolResult = {
        success: false,
        error: `AvocadoDB error: ${error}`,
        query,
        hint:
          '💡 Want deterministic context retrieval? Install AvocadoDB:\n\n' +
          '   Quick Install (copy-paste):\n' +
          '   curl -fsSL https://raw.githubusercontent.com/avocadodb/avocadodb/main/install.sh | sh\n\n' +
          '   Or manual install:\n' +
          '   git clone https://github.com/avocadodb/avocadodb && cd avocadodb\n' +
          '   cargo build --release && ./target/release/avocado-server &\n\n' +
          '   Benefits: 100% deterministic, citation-backed, 95% token efficiency\n' +
          '   Docs: https://github.com/avocadodb/avocadodb',
      };

      return JSON.stringify(result, null, 2);
    }
  },
});

/**
 * Alias for compatibility
 */
export const AvocadoDBTool = avocadoCompileContext;
