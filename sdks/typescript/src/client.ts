import fetch, { Response } from 'node-fetch';
import { createHash } from 'crypto';

/**
 * A single span of text from a document
 */
export interface Span {
  artifactId: string;
  artifactPath: string;
  startLine: number;
  endLine: number;
  text: string;
  tokenCount: number;
  score: number;
}

/**
 * A citation referencing a span in a document
 */
export interface Citation {
  artifactPath: string;
  startLine: number;
  endLine: number;
}

/**
 * A compiled context working set with deterministic guarantees
 */
export class WorkingSet {
  /** The compiled context text */
  readonly text: string;

  /** The spans included in this working set */
  readonly spans: Span[];

  /** Citations for the included spans */
  readonly citations: Citation[];

  /** Number of tokens used */
  readonly tokensUsed: number;

  /** The original query */
  readonly query: string;

  /** Compilation time in milliseconds */
  readonly compilationTimeMs: number;

  constructor(data: any) {
    this.text = data.text;
    this.spans = data.spans.map((s: any) => ({
      artifactId: s.artifact_id,
      artifactPath: s.artifact_path,
      startLine: s.start_line,
      endLine: s.end_line,
      text: s.text,
      tokenCount: s.token_count,
      score: s.score,
    }));
    this.citations = data.citations.map((c: any) => ({
      artifactPath: c.artifact_path,
      startLine: c.start_line,
      endLine: c.end_line,
    }));
    this.tokensUsed = data.tokens_used;
    this.query = data.query;
    this.compilationTimeMs = data.compilation_time_ms;
  }

  /**
   * Calculate deterministic SHA-256 hash of the context text
   */
  deterministicHash(): string {
    return createHash('sha256').update(this.text).digest('hex');
  }

  /**
   * Get a human-readable summary of the working set
   */
  summary(): string {
    return [
      `Query: ${this.query}`,
      `Spans: ${this.spans.length}`,
      `Tokens: ${this.tokensUsed}`,
      `Time: ${this.compilationTimeMs}ms`,
      `Hash: ${this.deterministicHash().substring(0, 16)}...`,
    ].join('\n');
  }
}

/**
 * Options for compiling context
 */
export interface CompileOptions {
  /** Token budget for the compiled context (default: 8000) */
  budget?: number;

  /** Weight for semantic (vector) search 0.0-1.0 (default: 0.7) */
  semanticWeight?: number;

  /** Weight for lexical (keyword) search 0.0-1.0 (default: 0.3) */
  lexicalWeight?: number;

  /** MMR diversity parameter 0.0-1.0 (default: 0.5) */
  mmrLambda?: number;

  /** Enable Maximal Marginal Relevance diversification (default: true) */
  enableMmr?: boolean;
}

/**
 * Statistics about the database
 */
export interface Stats {
  /** Number of artifacts (documents) */
  artifacts: number;

  /** Number of spans */
  spans: number;

  /** Total tokens across all spans */
  tokens: number;
}

/**
 * Result of ingesting a document
 */
export interface IngestResult {
  /** The artifact ID */
  artifactId: string;

  /** Number of spans created */
  spanCount: number;
}

/**
 * AvocadoDB client for deterministic context compilation
 */
export class AvocadoDB {
  private readonly baseUrl: string;

  /**
   * Create a new AvocadoDB client
   * @param url - Base URL of the AvocadoDB server (default: http://localhost:8080)
   */
  constructor(url: string = 'http://localhost:8080') {
    this.baseUrl = url.replace(/\/$/, ''); // Remove trailing slash
  }

  /**
   * Compile deterministic context for a query
   * @param query - The search query
   * @param options - Compilation options
   * @returns A WorkingSet with compiled context
   */
  async compile(query: string, options: CompileOptions = {}): Promise<WorkingSet> {
    const response = await fetch(`${this.baseUrl}/compile`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({
        query,
        token_budget: options.budget ?? 8000,
        semantic_weight: options.semanticWeight ?? 0.7,
        lexical_weight: options.lexicalWeight ?? 0.3,
        mmr_lambda: options.mmrLambda ?? 0.5,
        enable_mmr: options.enableMmr ?? true,
      }),
    });

    if (!response.ok) {
      throw new Error(`Compile failed: ${response.status} ${response.statusText}`);
    }

    const data = await response.json();
    return new WorkingSet(data);
  }

  /**
   * Ingest a document into the database
   * @param path - Document path
   * @param content - Document content (will read from file if not provided)
   * @returns Ingest result with artifact ID and span count
   */
  async ingest(path: string, content?: string): Promise<IngestResult> {
    // If content not provided, read from file
    let documentContent = content;
    if (!documentContent) {
      const fs = await import('fs/promises');
      documentContent = await fs.readFile(path, 'utf-8');
    }

    const response = await fetch(`${this.baseUrl}/ingest`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({
        path,
        content: documentContent,
      }),
    });

    if (!response.ok) {
      throw new Error(`Ingest failed: ${response.status} ${response.statusText}`);
    }

    const data = await response.json();
    return {
      artifactId: data.artifact_id,
      spanCount: data.spans_created,
    };
  }

  /**
   * Get database statistics
   * @returns Statistics about artifacts, spans, and tokens
   */
  async stats(): Promise<Stats> {
    const response = await fetch(`${this.baseUrl}/stats`);

    if (!response.ok) {
      throw new Error(`Stats failed: ${response.status} ${response.statusText}`);
    }

    const data = await response.json();
    return {
      artifacts: data.artifacts,
      spans: data.spans,
      tokens: data.tokens,
    };
  }

  /**
   * Check if the server is reachable
   * @returns true if server is responding
   */
  async ping(): Promise<boolean> {
    try {
      const response = await fetch(`${this.baseUrl}/stats`);
      return response.ok;
    } catch {
      return false;
    }
  }
}
