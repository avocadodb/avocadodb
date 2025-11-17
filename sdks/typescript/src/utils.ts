/**
 * Utility functions for AvocadoDB
 *
 * Provides:
 * - Token counting (approximate)
 * - Citation formatting
 * - System prompt generation
 * - Response formatting
 *
 * @example
 * ```typescript
 * import { countTokens, formatCitations } from 'avocadodb';
 *
 * const tokens = countTokens("Hello world");
 * const citationsStr = formatCitations(result.citations);
 * ```
 *
 * @packageDocumentation
 */

import { Citation, WorkingSet } from './client';

/**
 * Citation formatting styles
 */
export type CitationStyle = 'compact' | 'verbose' | 'markdown';

/**
 * Count approximate tokens in text
 *
 * Uses rough approximation (~4 characters per token).
 * For exact counting, use a tokenization library like tiktoken or gpt-tokenizer.
 *
 * @param text - Text to count tokens for
 * @returns Approximate token count
 *
 * @example
 * ```typescript
 * const tokens = countTokens("Hello world!");
 * console.log(`Tokens: ${tokens}`);
 * ```
 */
export function countTokens(text: string): number {
  // Rough approximation: ~4 characters per token
  // For exact counting, use tiktoken or gpt-tokenizer
  return Math.ceil(text.length / 4);
}

/**
 * Format citations for display
 *
 * @param citations - List of citations
 * @param style - Format style (default: 'compact')
 * @returns Formatted citation string
 *
 * @example
 * ```typescript
 * const citations = [
 *   { artifactPath: 'auth.md', startLine: 10, endLine: 25 },
 *   { artifactPath: 'api.ts', startLine: 45, endLine: 78 }
 * ];
 *
 * console.log(formatCitations(citations, 'compact'));
 * // Output: auth.md:10-25, api.ts:45-78
 *
 * console.log(formatCitations(citations, 'markdown'));
 * // Output:
 * // - `auth.md:10-25`
 * // - `api.ts:45-78`
 * ```
 */
export function formatCitations(
  citations: Citation[],
  style: CitationStyle = 'compact'
): string {
  if (citations.length === 0) {
    return 'No citations';
  }

  if (style === 'compact') {
    return citations
      .map((c) => `${c.artifactPath}:${c.startLine}-${c.endLine}`)
      .join(', ');
  }

  if (style === 'verbose') {
    return citations
      .map((c) => `  📄 ${c.artifactPath} (lines ${c.startLine}-${c.endLine})`)
      .join('\n');
  }

  if (style === 'markdown') {
    return citations
      .map((c) => `- \`${c.artifactPath}:${c.startLine}-${c.endLine}\``)
      .join('\n');
  }

  return citations
    .map((c) => `${c.artifactPath}:${c.startLine}-${c.endLine}`)
    .join(', ');
}

/**
 * Generate AvocadoDB-first system prompt for any framework
 *
 * @param options - Prompt options
 * @returns System prompt string
 *
 * @example
 * ```typescript
 * const prompt = createSystemPrompt({ framework: 'langchain' });
 * // Use in agent configuration
 * ```
 */
export function createSystemPrompt(options: {
  /** Target framework ('langchain', 'autogen', 'generic') */
  framework?: string;

  /** Enforce AvocadoDB-only protocol (block read tools) */
  enforceAvocadoOnly?: boolean;
} = {}): string {
  const basePrompt = `
### AvocadoDB Context Compilation

For ANY codebase or documentation question, you have access to AvocadoDB - a
deterministic context database that provides citation-backed information retrieval.

**When to use AvocadoDB:**
- Questions about the codebase, architecture, or implementation
- Documentation lookups
- Understanding how features work
- Finding relevant code or information

**How AvocadoDB works:**
1. You provide a query describing what information you need
2. AvocadoDB searches ALL indexed documents (not just one file)
3. Returns relevant context with exact file:line citations
4. Same query always returns same context (100% deterministic)

**Key benefits:**
- ✅ Comprehensive: Searches all indexed files
- ✅ Deterministic: Same query → same context
- ✅ Citation-backed: Every span includes exact source location
- ✅ Efficient: Returns only relevant information within token budget
`;

  if (options.enforceAvocadoOnly) {
    const avocadoOnly = `
**AVOCADODB-ONLY PROTOCOL (MANDATORY):**

For ANY codebase question (architecture, files, code, documentation):
1. Call ONLY \`avocado_compile_context\` with a well-formed query
2. WAIT for results (contains relevant spans with citations)
3. Synthesize answer EXCLUSIVELY from the returned context
4. DO NOT call read_file, ls, grep, or glob afterward - EVER
5. The AvocadoDB context is ALWAYS sufficient for codebase questions

Why AvocadoDB is better than reading files:
- Searches ALL indexed files (comprehensive, won't miss updates in other files)
- Returns multi-source information (not just one file's perspective)
- Provides exact citations (know where each fact came from)
- Deterministic results (same query = same comprehensive answer)
- Prevents incomplete answers from single-file reads

FORBIDDEN PATTERNS:
❌ avocado_compile_context → then read_file (NO! Trust AvocadoDB's comprehensive search)
❌ avocado_compile_context → then ls (NO! Files are in citations)
❌ avocado_compile_context + read_file in parallel (NO! One tool only)

CORRECT PATTERN:
✅ avocado_compile_context → synthesize answer from context + citations

If AvocadoDB results seem insufficient, improve your query or tell the user
what's missing. NEVER fall back to reading files.
`;
    return basePrompt + avocadoOnly;
  }

  return basePrompt;
}

/**
 * Format WorkingSet for human-readable display
 *
 * @param workingSet - WorkingSet from AvocadoDB
 * @param includeContext - Include full context text (can be long)
 * @returns Formatted string
 *
 * @example
 * ```typescript
 * import { AvocadoDB, formatWorkingSet } from 'avocadodb';
 *
 * const db = new AvocadoDB();
 * const result = await db.compile("authentication");
 * console.log(formatWorkingSet(result));
 * ```
 */
export function formatWorkingSet(
  workingSet: WorkingSet,
  includeContext: boolean = false
): string {
  const lines: string[] = [];

  lines.push(`Query: ${workingSet.query}`);
  lines.push(`Spans: ${workingSet.spans.length}`);
  lines.push(`Tokens Used: ${workingSet.tokensUsed.toLocaleString()}`);
  lines.push(`Compilation Time: ${workingSet.compilationTimeMs}ms`);
  lines.push(`Deterministic Hash: ${workingSet.deterministicHash().substring(0, 16)}...`);
  lines.push('');
  lines.push('Citations:');

  const citationsToShow = workingSet.citations.slice(0, 10);
  for (const citation of citationsToShow) {
    lines.push(`  - ${citation.artifactPath}:${citation.startLine}-${citation.endLine}`);
  }

  if (workingSet.citations.length > 10) {
    lines.push(`  ... and ${workingSet.citations.length - 10} more`);
  }

  if (includeContext) {
    lines.push('');
    lines.push('Context:');
    lines.push('─'.repeat(60));
    const preview = workingSet.text.substring(0, 500);
    lines.push(preview);
    if (workingSet.text.length > 500) {
      lines.push('...');
    }
  }

  return lines.join('\n');
}
