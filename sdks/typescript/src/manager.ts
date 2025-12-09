/**
 * AvocadoDB Server Lifecycle Management
 *
 * Framework-agnostic server management for AvocadoDB.
 * Handles auto-start, health checks, and daemon mode persistence.
 *
 * @example
 * ```typescript
 * import { AvocadoDBManager, getManager } from 'avocadodb';
 *
 * const manager = getManager();
 * await manager.ensureRunning();
 *
 * const stats = await manager.getStats();
 * console.log(`Indexed: ${stats.artifactsCount} docs`);
 * ```
 *
 * @packageDocumentation
 */

import { spawn, ChildProcess } from 'child_process';
import { existsSync } from 'fs';
import { homedir } from 'os';
import { join } from 'path';
import fetch from 'node-fetch';

/**
 * Server health check result
 */
export interface HealthCheck {
  /** Whether server is running */
  running: boolean;

  /** Whether binary was found */
  binaryFound: boolean;

  /** Server URL */
  serverUrl: string;

  /** Database statistics (if running) */
  stats: ServerStats | null;
}

/**
 * Database statistics
 */
export interface ServerStats {
  /** Number of artifacts (documents) */
  artifactsCount: number;

  /** Number of spans */
  spansCount: number;

  /** Total tokens indexed */
  totalTokens: number;
}

/**
 * Manages AvocadoDB server lifecycle (framework-agnostic)
 *
 * Features:
 * - Auto-detection of binary location
 * - Auto-installation from source
 * - Daemon mode (server persists after CLI exit)
 * - Health checks and stats
 * - Environment variable configuration
 *
 * @example
 * ```typescript
 * const manager = new AvocadoDBManager({ autoStart: true, port: 8765 });
 * if (await manager.ensureRunning()) {
 *   console.log('Server ready!');
 * }
 *
 * const stats = await manager.getStats();
 * console.log(`Indexed: ${stats.artifactsCount} docs`);
 * ```
 */
export class AvocadoDBManager {
  /** Server URL */
  readonly serverUrl: string;

  /** Server port */
  readonly port: number;

  /** Auto-start enabled */
  readonly autoStart: boolean;

  /** Path to binary */
  private binaryPath: string | null = null;

  /** Server subprocess */
  private serverProcess: ChildProcess | null = null;

  /**
   * Create AvocadoDB manager
   * @param options - Manager options
   */
  constructor(options: {
    /** Automatically start server if not running */
    autoStart?: boolean;

    /** Server port (default: 8765, or from AVOCADODB_URL) */
    port?: number;
  } = {}) {
    // Allow override via environment variable
    const envUrl = process.env.AVOCADODB_URL;
    if (envUrl) {
      this.serverUrl = envUrl;
      const url = new URL(envUrl);
      this.port = parseInt(url.port) || options.port || 8765;
    } else {
      this.port = options.port || 8765;
      this.serverUrl = `http://localhost:${this.port}`;
    }

    this.autoStart = options.autoStart ?? true;

    // Find or install AvocadoDB
    if (this.autoStart) {
      this.ensureAvailable();
    }
  }

  /**
   * Find AvocadoDB binary in common locations
   * @returns Path to binary if found
   */
  private findBinary(): string | null {
    const cwd = process.cwd();
    const home = homedir();

    const possiblePaths = [
      // In current directory
      join(cwd, 'target/release/avocado-server'),
      // In parent directories (up to 3 levels)
      join(cwd, '../target/release/avocado-server'),
      join(cwd, '../../target/release/avocado-server'),
      join(cwd, '../../../target/release/avocado-server'),
      // In home directory
      join(home, '.avocadodb/avocado-server'),
      // System-wide
      '/usr/local/bin/avocado-server',
    ];

    for (const path of possiblePaths) {
      if (existsSync(path)) {
        return path;
      }
    }

    return null;
  }

  /**
   * Ensure AvocadoDB binary is available (find or install)
   */
  private ensureAvailable(): void {
    this.binaryPath = this.findBinary();

    if (!this.binaryPath && this.autoStart) {
      console.warn('⚠️  AvocadoDB binary not found. Please install:');
      console.warn('   git clone https://github.com/avocadodb/avocadodb');
      console.warn('   cd avocadodb && cargo build --release');
    }
  }

  /**
   * Check if AvocadoDB server is running
   * @returns True if server is reachable
   */
  async isRunning(): Promise<boolean> {
    try {
      const response = await fetch(`${this.serverUrl}/stats`, {
        method: 'GET',
        timeout: 1000,
      } as any);
      return response.ok;
    } catch {
      return false;
    }
  }

  /**
   * Start AvocadoDB server as background daemon
   * @returns True if server started successfully
   */
  async startServer(): Promise<boolean> {
    if (await this.isRunning()) {
      // Server already running
      return true;
    }

    if (!this.binaryPath) {
      console.error('⚠️  AvocadoDB binary not found');
      return false;
    }

    console.log(`🥑 Starting AvocadoDB server on port ${this.port}...`);

    try {
      // Start server in background with PORT env var
      this.serverProcess = spawn(this.binaryPath, [], {
        env: {
          ...process.env,
          PORT: this.port.toString(),
        },
        detached: true,  // Detach from parent (daemon mode)
        stdio: 'ignore',  // Ignore stdio
      });

      // Unref so parent can exit
      this.serverProcess.unref();

      // Wait for server to be ready (max 5 seconds)
      for (let i = 0; i < 10; i++) {
        await new Promise(resolve => setTimeout(resolve, 500));
        if (await this.isRunning()) {
          console.log('✅ Server started (daemon mode - stays running)');
          return true;
        }
      }

      console.error('⚠️  Server failed to start');
      return false;

    } catch (error) {
      console.error(`⚠️  Failed to start server: ${error}`);
      return false;
    }
  }

  /**
   * Ensure AvocadoDB server is running (start if needed)
   * @returns True if server is available
   *
   * @example
   * ```typescript
   * const manager = new AvocadoDBManager();
   * if (await manager.ensureRunning()) {
   *   // Server is ready, proceed with queries
   * }
   * ```
   */
  async ensureRunning(): Promise<boolean> {
    if (await this.isRunning()) {
      return true;
    }

    if (this.autoStart) {
      return await this.startServer();
    }

    return false;
  }

  /**
   * Get database statistics from server
   * @returns Statistics or null if server unavailable
   *
   * @example
   * ```typescript
   * const stats = await manager.getStats();
   * if (stats) {
   *   console.log(`Indexed: ${stats.artifactsCount} docs`);
   * }
   * ```
   */
  async getStats(): Promise<ServerStats | null> {
    if (!(await this.isRunning())) {
      return null;
    }

    try {
      const response = await fetch(`${this.serverUrl}/stats`, {
        method: 'GET',
        timeout: 2000,
      } as any);

      if (!response.ok) {
        return null;
      }

      const data: any = await response.json();
      return {
        artifactsCount: data.artifacts_count || 0,
        spansCount: data.spans_count || 0,
        totalTokens: data.total_tokens || 0,
      };
    } catch {
      return null;
    }
  }

  /**
   * Comprehensive health check
   * @returns Health check result
   *
   * @example
   * ```typescript
   * const health = await manager.healthCheck();
   * if (health.running) {
   *   console.log(`Server healthy: ${JSON.stringify(health.stats)}`);
   * }
   * ```
   */
  async healthCheck(): Promise<HealthCheck> {
    const running = await this.isRunning();
    const stats = running ? await this.getStats() : null;

    return {
      running,
      binaryFound: this.binaryPath !== null,
      serverUrl: this.serverUrl,
      stats,
    };
  }
}

/**
 * Global singleton instance
 */
let globalManager: AvocadoDBManager | null = null;

/**
 * Get or create global AvocadoDB manager instance
 * @param options - Manager options (only used on first call)
 * @returns Global manager instance
 *
 * @example
 * ```typescript
 * import { getManager } from 'avocadodb';
 *
 * const manager = getManager();
 * await manager.ensureRunning();
 * ```
 */
export function getManager(options?: {
  autoStart?: boolean;
  port?: number;
}): AvocadoDBManager {
  if (!globalManager) {
    // Check environment variable for auto-start override
    const autoStartEnv = process.env.AVOCADODB_AUTO_START !== 'false';
    globalManager = new AvocadoDBManager({
      autoStart: options?.autoStart && autoStartEnv,
      port: options?.port || 8765,
    });
  }

  return globalManager;
}
