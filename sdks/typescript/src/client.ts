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

  /**
   * Ask a question and get a natural language answer (v2.0)
   * Uses TinyLlama to generate answers from AvocadoDB context.
   * Falls back to returning context text if LLM is not available.
   *
   * @param query - The question to ask
   * @param options - Options for asking
   * @returns Natural language answer as string, or context text if LLM unavailable
   */
  async ask(
    query: string,
    options: {
      /** LLM mode: "auto" (try local, fallback), "local" (require), "none" (just context) */
      llm?: 'auto' | 'local' | 'none';
      /** Token budget for context compilation (default: 8000) */
      budget?: number;
      /** Maximum tokens for answer generation (default: 150) */
      maxTokens?: number;
      /** Use deterministic generation (default: true) */
      deterministic?: boolean;
    } = {}
  ): Promise<string> {
    const {
      llm = 'auto',
      budget = 8000,
      maxTokens = 150,
      deterministic = true,
    } = options;

    // Get context first
    const context = await this.compile(query, { budget });

    // If llm is "none", just return context
    if (llm === 'none') {
      return context.text;
    }

    // Try to use Python SDK's ask() method
    try {
      return await this._callPythonAsk(query, {
        llm,
        budget,
        maxTokens,
        deterministic,
      });
    } catch (error) {
      // If Python SDK not available or failed, fallback to context
      if (llm === 'local') {
        throw new Error(
          `TinyLlama not available: ${error}. Install Python SDK with: pip install avocadodb[llm]`
        );
      }
      // For "auto" mode, just return context
      return context.text;
    }
  }

  /**
   * Internal method to call Python SDK's ask() method
   * @private
   */
  private async _callPythonAsk(
    query: string,
    options: {
      llm: string;
      budget: number;
      maxTokens: number;
      deterministic: boolean;
    }
  ): Promise<string> {
    const { spawn } = await import('child_process');
    const pathModule = await import('path');
    const fs = await import('fs/promises');

    // Find the ask.py script or create a temporary Python script
    const projectRoot = this._findProjectRoot();
    const askScript = pathModule.join(projectRoot, 'avocado-cli', 'scripts', 'ask.py');

    // If ask.py exists, use it; otherwise create inline Python code
    let pythonArgs: string[];
    let pythonCode: string;

    try {
      await fs.access(askScript);
      // Use the ask.py script
      pythonArgs = [
        askScript,
        query,
        '--url', this.baseUrl,
        '--budget', options.budget.toString(),
        '--llm', options.llm,
        '--max-tokens', options.maxTokens.toString(),
      ];
      pythonCode = '';
    } catch {
      // Fallback: inline Python code
      const escapedQuery = query.replace(/'/g, "\\'").replace(/\n/g, '\\n');
      pythonCode = `
import sys
import os
# Try to find SDK in common locations
sdk_paths = [
    os.path.join(os.getcwd(), 'sdks', 'python'),
    os.path.join(os.path.dirname(__file__), '..', 'sdks', 'python'),
    os.path.expanduser('~/.local/lib/python3.*/site-packages'),
]
for sdk_path in sdk_paths:
    if os.path.exists(sdk_path):
        sys.path.insert(0, sdk_path)
        break

from avocado import AvocadoDB

db = AvocadoDB(url='${this.baseUrl}')
answer = db.ask(
    query='${escapedQuery}',
    llm='${options.llm}',
    budget=${options.budget},
    max_new_tokens=${options.maxTokens},
    deterministic=${options.deterministic}
)
print(answer)
`;
      pythonArgs = ['-c', pythonCode];
    }

    return new Promise((resolve, reject) => {
      const python = spawn('python3', pythonArgs);

      let stdout = '';
      let stderr = '';

      python.stdout.on('data', (data) => {
        stdout += data.toString();
      });

      python.stderr.on('data', (data) => {
        stderr += data.toString();
      });

      python.on('close', (code) => {
        if (code === 0) {
          resolve(stdout.trim());
        } else {
          reject(new Error(`Python SDK call failed: ${stderr || 'Unknown error'}`));
        }
      });

      python.on('error', (error) => {
        reject(new Error(`Failed to spawn Python process: ${error.message}`));
      });
    });
  }

  /**
   * Find the project root directory
   * @private
   */
  private _findProjectRoot(): string {
    const pathModule = require('path');
    const fs = require('fs');
    let currentDir = process.cwd();
    
    // Look for common project root indicators
    const indicators = ['avocado-cli', 'sdks', 'Cargo.toml', 'package.json'];
    
    for (let i = 0; i < 10; i++) {
      for (const indicator of indicators) {
        const checkPath = pathModule.join(currentDir, indicator);
        try {
          if (fs.existsSync(checkPath)) {
            return currentDir;
          }
        } catch {
          // Continue searching
        }
      }
      
      const parent = pathModule.dirname(currentDir);
      if (parent === currentDir) {
        break; // Reached filesystem root
      }
      currentDir = parent;
    }
    
    return process.cwd(); // Fallback to current directory
  }
}
