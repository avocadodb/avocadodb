/**
 * Smart Auto-Ingestion for AvocadoDB
 *
 * Framework-agnostic intelligent ingestion with:
 * - Project type detection (Python, Node, Rust, Go, etc.)
 * - Language-specific file patterns
 * - Binary file filtering
 * - Recursive directory traversal
 *
 * @example
 * ```typescript
 * import { AutoIngest } from 'avocadodb';
 *
 * const ingester = new AutoIngest();
 * const result = await ingester.ingestProject('.');  // Auto-detects project type
 * console.log(`Ingested ${result.ingested} files`);
 * ```
 *
 * @packageDocumentation
 */

import { spawn } from 'child_process';
import { existsSync, readFileSync } from 'fs';
import { homedir } from 'os';
import { join } from 'path';
import { glob } from 'glob';

/**
 * Detected project types
 */
export enum ProjectType {
  PYTHON = 'python',
  JAVASCRIPT = 'javascript',
  TYPESCRIPT = 'typescript',
  RUST = 'rust',
  GO = 'go',
  JAVA = 'java',
  CPP = 'cpp',
  RUBY = 'ruby',
  PHP = 'php',
  UNKNOWN = 'unknown',
}

/**
 * Result of ingesting a project
 */
export interface IngestProjectResult {
  /** Number of files ingested */
  ingested: number;

  /** Number of files skipped */
  skipped: number;

  /** Detected project type */
  projectType: ProjectType;

  /** Patterns used */
  patterns: string[];
}

/**
 * Smart auto-ingestion with project type detection
 *
 * Automatically detects project type and ingests relevant files:
 * - Documentation (README, docs/)
 * - Source code (language-specific patterns)
 * - Excludes build artifacts and dependencies
 *
 * @example
 * ```typescript
 * const ingester = new AutoIngest();
 * const result = await ingester.ingestProject('.', { maxFiles: 100 });
 * console.log(`Ingested ${result.ingested} files`);
 * ```
 */
export class AutoIngest {
  /** Path to ingest binary */
  private readonly ingestBinary: string | null;

  /**
   * Create auto-ingester
   * @param options - Ingester options
   */
  constructor(options: {
    /** Path to avocado ingest binary (auto-detected if not provided) */
    ingestBinary?: string;
  } = {}) {
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
   * Detect project type from marker files
   * @param path - Project directory to analyze
   * @returns Detected project type
   *
   * @example
   * ```typescript
   * const projectType = ingester.detectProjectType('.');
   * console.log(`Detected: ${projectType}`);
   * ```
   */
  detectProjectType(path: string): ProjectType {
    // Python
    if (
      existsSync(join(path, 'pyproject.toml')) ||
      existsSync(join(path, 'setup.py')) ||
      existsSync(join(path, 'requirements.txt'))
    ) {
      return ProjectType.PYTHON;
    }

    // JavaScript/TypeScript (Node.js)
    if (existsSync(join(path, 'package.json'))) {
      try {
        const packageJson = JSON.parse(
          readFileSync(join(path, 'package.json'), 'utf-8')
        );
        const deps = {
          ...packageJson.dependencies,
          ...packageJson.devDependencies,
        };
        if (deps.typescript || deps['@types/node']) {
          return ProjectType.TYPESCRIPT;
        }
      } catch {
        // Ignore parse errors
      }
      return ProjectType.JAVASCRIPT;
    }

    // Rust
    if (existsSync(join(path, 'Cargo.toml'))) {
      return ProjectType.RUST;
    }

    // Go
    if (existsSync(join(path, 'go.mod'))) {
      return ProjectType.GO;
    }

    // Java/Kotlin
    if (
      existsSync(join(path, 'pom.xml')) ||
      existsSync(join(path, 'build.gradle'))
    ) {
      return ProjectType.JAVA;
    }

    // C/C++
    if (
      existsSync(join(path, 'Makefile')) ||
      existsSync(join(path, 'CMakeLists.txt'))
    ) {
      return ProjectType.CPP;
    }

    // Ruby
    if (existsSync(join(path, 'Gemfile'))) {
      return ProjectType.RUBY;
    }

    // PHP
    if (existsSync(join(path, 'composer.json'))) {
      return ProjectType.PHP;
    }

    return ProjectType.UNKNOWN;
  }

  /**
   * Get file patterns for a project type
   * @param projectType - Project type
   * @returns List of glob patterns for source files
   *
   * @example
   * ```typescript
   * const patterns = ingester.getPatternsForProject(ProjectType.TYPESCRIPT);
   * // Returns: ['**\/*.ts', '**\/*.tsx', '**\/*.d.ts']
   * ```
   */
  getPatternsForProject(projectType: ProjectType): string[] {
    const patternsMap: Record<ProjectType, string[]> = {
      [ProjectType.PYTHON]: ['**/*.py', '**/*.pyi'],
      [ProjectType.JAVASCRIPT]: ['**/*.js', '**/*.jsx', '**/*.mjs'],
      [ProjectType.TYPESCRIPT]: ['**/*.ts', '**/*.tsx', '**/*.d.ts'],
      [ProjectType.RUST]: ['**/*.rs'],
      [ProjectType.GO]: ['**/*.go'],
      [ProjectType.JAVA]: ['**/*.java', '**/*.kt'],
      [ProjectType.CPP]: ['**/*.c', '**/*.cpp', '**/*.h', '**/*.hpp'],
      [ProjectType.RUBY]: ['**/*.rb'],
      [ProjectType.PHP]: ['**/*.php'],
      [ProjectType.UNKNOWN]: [
        '**/*.py',
        '**/*.js',
        '**/*.ts',
        '**/*.java',
        '**/*.go',
        '**/*.rs',
      ],
    };

    return patternsMap[projectType] || [];
  }

  /**
   * Get common documentation file patterns
   * @returns List of glob patterns for documentation files
   */
  getDocumentationPatterns(): string[] {
    return [
      'README.md',
      'QUICKSTART.md',
      'CONTRIBUTING.md',
      'CHANGELOG.md',
      'docs/**/*.md',
      '*.md',
    ];
  }

  /**
   * Intelligently ingest a project directory
   *
   * Auto-detects project type and ingests relevant files:
   * - Documentation (always included if includeDocs=true)
   * - Source code (language-specific, if includeSource=true)
   * - Excludes: node_modules, .git, venv, build artifacts
   *
   * @param path - Project directory to ingest (default: current directory)
   * @param options - Ingestion options
   * @returns Ingestion result
   *
   * @example
   * ```typescript
   * const result = await ingester.ingestProject('.', { maxFiles: 100 });
   * console.log(`Ingested ${result.ingested} ${result.projectType} files`);
   * ```
   */
  async ingestProject(
    path: string = '.',
    options: {
      /** Maximum number of files to ingest (default: 100) */
      maxFiles?: number;

      /** Include source code files (default: true) */
      includeSource?: boolean;

      /** Include documentation files (default: true) */
      includeDocs?: boolean;
    } = {}
  ): Promise<IngestProjectResult> {
    const maxFiles = options.maxFiles || 100;
    const includeSource = options.includeSource ?? true;
    const includeDocs = options.includeDocs ?? true;

    if (!this.ingestBinary || !existsSync(this.ingestBinary)) {
      console.warn('⚠️  Avocado ingest binary not found');
      return {
        ingested: 0,
        skipped: 0,
        projectType: ProjectType.UNKNOWN,
        patterns: [],
      };
    }

    if (!existsSync(path)) {
      console.warn(`⚠️  Path not found: ${path}`);
      return {
        ingested: 0,
        skipped: 0,
        projectType: ProjectType.UNKNOWN,
        patterns: [],
      };
    }

    // Detect project type
    const projectType = this.detectProjectType(path);

    // Build patterns
    const patterns: string[] = [];
    if (includeDocs) {
      patterns.push(...this.getDocumentationPatterns());
    }
    if (includeSource) {
      patterns.push(...this.getPatternsForProject(projectType));
    }

    // Find matching files
    let pathsToIngest: string[] = [];
    for (const pattern of patterns) {
      try {
        const matching = await glob(pattern, {
          cwd: path,
          ignore: [
            '**/node_modules/**',
            '**/.git/**',
            '**/venv/**',
            '**/.venv/**',
            '**/__pycache__/**',
            '**/target/**',
            '**/build/**',
            '**/dist/**',
            '**/.next/**',
            '**/.cache/**',
            '**/.tox/**',
            '**/vendor/**',
          ],
        });
        pathsToIngest.push(...matching.slice(0, 50));  // Limit per pattern
      } catch {
        // Ignore pattern errors
      }
    }

    // Remove duplicates and limit total
    pathsToIngest = [...new Set(pathsToIngest)].slice(0, maxFiles);

    if (pathsToIngest.length === 0) {
      console.log('   No files found to ingest');
      return {
        ingested: 0,
        skipped: 0,
        projectType,
        patterns,
      };
    }

    // Ingest each file
    let ingested = 0;
    let skipped = 0;

    for (const relPath of pathsToIngest) {
      const fullPath = join(path, relPath);
      try {
        const success = await new Promise<boolean>((resolve) => {
          const proc = spawn(this.ingestBinary!, ['ingest', fullPath]);
          proc.on('close', (code) => resolve(code === 0));
          proc.on('error', () => resolve(false));
        });

        if (success) {
          ingested++;
        } else {
          skipped++;
        }
      } catch {
        skipped++;
      }
    }

    console.log(`✅ Auto-ingested ${ingested} files (${projectType} project)`);

    return {
      ingested,
      skipped,
      projectType,
      patterns,
    };
  }

  /**
   * Ingest a single file
   * @param path - File to ingest
   * @returns True if ingestion succeeded
   *
   * @example
   * ```typescript
   * const success = await ingester.ingestFile('README.md');
   * ```
   */
  async ingestFile(path: string): Promise<boolean> {
    if (!this.ingestBinary || !existsSync(this.ingestBinary)) {
      return false;
    }

    try {
      return await new Promise<boolean>((resolve) => {
        const proc = spawn(this.ingestBinary!, ['ingest', path]);
        proc.on('close', (code) => resolve(code === 0));
        proc.on('error', () => resolve(false));
      });
    } catch {
      return false;
    }
  }
}
