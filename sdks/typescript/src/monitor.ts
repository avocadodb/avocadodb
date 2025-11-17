/**
 * Background File Monitoring for AvocadoDB
 *
 * Framework-agnostic file watcher that detects changes and triggers re-ingestion.
 * Useful for keeping the AvocadoDB index up-to-date during development.
 *
 * @example
 * ```typescript
 * import { FileMonitor } from 'avocadodb';
 *
 * const monitor = new FileMonitor({ intervalSeconds: 30 });
 * monitor.startMonitoring(['**\/*.ts', '**\/*.md']);
 *
 * // Monitor runs in background, auto-re-ingests changed files
 * // Stop when done: monitor.stopMonitoring();
 * ```
 *
 * @packageDocumentation
 */

import { spawn } from 'child_process';
import { existsSync, statSync } from 'fs';
import { homedir } from 'os';
import { join } from 'path';
import { glob } from 'glob';

/**
 * Callback for file change events
 */
export type OnChangeCallback = (files: string[]) => void;

/**
 * Background file watcher for automatic re-ingestion
 *
 * Monitors files matching specified patterns and automatically
 * re-ingests them when changes are detected. Runs with configurable
 * polling interval.
 *
 * @example
 * ```typescript
 * const monitor = new FileMonitor({ intervalSeconds: 30 });
 * monitor.startMonitoring(['docs/**\/*.md', 'src/**\/*.ts']);
 *
 * // Files are automatically re-ingested when modified
 * monitor.stopMonitoring();
 * ```
 */
export class FileMonitor {
  /** Polling interval in seconds */
  private readonly intervalSeconds: number;

  /** Path to ingest binary */
  private readonly ingestBinary: string | null;

  /** Monitor interval timer */
  private monitorInterval: NodeJS.Timeout | null = null;

  /** Last modified times for tracked files */
  private lastModified: Map<string, number> = new Map();

  /** Change event callback */
  private onChangeCallback: OnChangeCallback | null = null;

  /**
   * Create file monitor
   * @param options - Monitor options
   */
  constructor(options: {
    /** How often to check for changes in seconds (default: 30) */
    intervalSeconds?: number;

    /** Path to avocado ingest binary (auto-detected if not provided) */
    ingestBinary?: string;
  } = {}) {
    this.intervalSeconds = options.intervalSeconds || 30;
    this.ingestBinary = options.ingestBinary || this.findIngestBinary();
  }

  /**
   * Find avocado ingest binary
   * @returns Path to binary if found
   */
  private findIngestBinary(): string | null {
    const cwd = process.cwd();
    const home = homedir();

    const possiblePaths = [
      join(cwd, 'target/release/avocado'),
      join(cwd, '../target/release/avocado'),
      join(cwd, '../../target/release/avocado'),
      join(home, '.avocadodb/repo/target/release/avocado'),
      '/usr/local/bin/avocado',
    ];

    for (const path of possiblePaths) {
      if (existsSync(path)) {
        return path;
      }
    }

    return null;
  }

  /**
   * Register callback for file change events
   * @param callback - Function called with list of changed files
   *
   * @example
   * ```typescript
   * monitor.onChange((files) => {
   *   console.log(`Changed: ${files.join(', ')}`);
   * });
   * ```
   */
  onChange(callback: OnChangeCallback): void {
    this.onChangeCallback = callback;
  }

  /**
   * Monitor loop - checks for file changes
   */
  private async monitorLoop(patterns: string[], cwd: string): Promise<void> {
    if (!this.ingestBinary || !existsSync(this.ingestBinary)) {
      return;
    }

    try {
      // Find files matching patterns
      const pathsToCheck: string[] = [];
      for (const pattern of patterns) {
        try {
          const matching = await glob(pattern, {
            cwd,
            ignore: [
              '**/node_modules/**',
              '**/.git/**',
              '**/venv/**',
              '**/__pycache__/**',
              '**/target/**',
              '**/build/**',
              '**/dist/**',
              '**/.next/**',
              '**/.cache/**',
            ],
          });
          pathsToCheck.push(...matching.slice(0, 100));  // Limit per pattern
        } catch {
          // Ignore pattern errors
        }
      }

      // Check for modified files
      const changedFiles: string[] = [];
      for (const relPath of pathsToCheck) {
        const fullPath = join(cwd, relPath);
        if (!existsSync(fullPath)) continue;

        try {
          const stats = statSync(fullPath);
          if (!stats.isFile()) continue;

          const mtime = stats.mtimeMs;
          const lastMtime = this.lastModified.get(fullPath) || 0;

          // File is new or modified
          if (mtime > lastMtime) {
            // Re-ingest file
            await new Promise<void>((resolve) => {
              const proc = spawn(this.ingestBinary!, ['ingest', fullPath]);
              proc.on('close', (code) => {
                if (code === 0) {
                  this.lastModified.set(fullPath, mtime);
                  changedFiles.push(fullPath);
                }
                resolve();
              });
              proc.on('error', () => resolve());
            });
          }
        } catch {
          // Ignore file errors
        }
      }

      // Notify callback if files changed
      if (changedFiles.length > 0) {
        if (this.onChangeCallback) {
          this.onChangeCallback(changedFiles);
        } else {
          console.log(`🥑 Background: Re-ingested ${changedFiles.length} changed files`);
        }
      }
    } catch {
      // Silently continue on errors
    }
  }

  /**
   * Start background file monitoring
   * @param patterns - Glob patterns to monitor
   * @param cwd - Working directory to monitor (default: current directory)
   *
   * @example
   * ```typescript
   * monitor.startMonitoring([
   *   'docs/**\/*.md',
   *   'src/**\/*.ts',
   *   'README.md'
   * ]);
   * ```
   */
  startMonitoring(patterns: string[], cwd: string = process.cwd()): void {
    if (this.monitorInterval) {
      console.warn('⚠️  Monitor already running');
      return;
    }

    this.monitorInterval = setInterval(
      () => this.monitorLoop(patterns, cwd),
      this.intervalSeconds * 1000
    );
  }

  /**
   * Stop background file monitoring
   *
   * @example
   * ```typescript
   * monitor.stopMonitoring();
   * ```
   */
  stopMonitoring(): void {
    if (this.monitorInterval) {
      clearInterval(this.monitorInterval);
      this.monitorInterval = null;
    }
  }

  /**
   * Check if monitoring is active
   * @returns True if monitor is running
   */
  isMonitoring(): boolean {
    return this.monitorInterval !== null;
  }
}
