"""
AvocadoDB client implementation.

Supports both HTTP server mode and CLI mode (direct binary calls).
CLI mode is recommended for multi-repo usage (no server management needed).
"""

import requests
import subprocess
import json
import os
from pathlib import Path
from typing import Optional, Dict, List, Any, Literal
from dataclasses import dataclass
import hashlib


@dataclass
class Citation:
    """A citation linking to a source location."""
    span_id: str
    artifact_id: str
    artifact_path: str
    start_line: int
    end_line: int
    score: float

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> 'Citation':
        return cls(
            span_id=data['span_id'],
            artifact_id=data['artifact_id'],
            artifact_path=data['artifact_path'],
            start_line=data['start_line'],
            end_line=data['end_line'],
            score=data.get('score', 0.0)
        )


@dataclass
class Span:
    """A text span with metadata."""
    id: str
    artifact_id: str
    start_line: int
    end_line: int
    text: str
    token_count: int
    embedding: Optional[List[float]] = None
    embedding_model: Optional[str] = None
    metadata: Optional[Dict[str, Any]] = None

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> 'Span':
        return cls(
            id=data['id'],
            artifact_id=data['artifact_id'],
            start_line=data['start_line'],
            end_line=data['end_line'],
            text=data['text'],
            token_count=data['token_count'],
            embedding=data.get('embedding'),
            embedding_model=data.get('embedding_model'),
            metadata=data.get('metadata')
        )


class WorkingSet:
    """A compiled context working set with deterministic guarantees."""

    def __init__(self, data: Dict[str, Any]):
        """Initialize from API response data."""
        self.text: str = data['text']
        self.spans: List[Span] = [Span.from_dict(s) for s in data['spans']]
        self.citations: List[Citation] = [Citation.from_dict(c) for c in data['citations']]
        self.tokens_used: int = data['tokens_used']
        self.query: str = data['query']
        self.compilation_time_ms: int = data['compilation_time_ms']

    def deterministic_hash(self) -> str:
        """Calculate deterministic hash of the context text.

        Returns:
            SHA-256 hash of the compiled text (hex string)
        """
        return hashlib.sha256(self.text.encode('utf-8')).hexdigest()

    def __repr__(self) -> str:
        return (
            f"WorkingSet(query='{self.query}', "
            f"spans={len(self.spans)}, "
            f"tokens={self.tokens_used}, "
            f"time={self.compilation_time_ms}ms)"
        )


class AvocadoDB:
    """AvocadoDB client for deterministic context compilation.

    Supports two modes:
    - HTTP mode: Connects to daemon server (MongoDB-style, manages multiple projects)
      * Automatically detects current directory as project path
      * One daemon manages all projects (like MongoDB)
      * Indexes stay in memory for fast queries
    - CLI mode: Direct binary calls (for single-project use)

    Example:
        >>> # HTTP mode (default)
        >>> db = AvocadoDB("http://localhost:8765")
        >>> result = db.compile("How does authentication work?")
        
        >>> # CLI mode (recommended for multi-repo)
        >>> db = AvocadoDB(mode="cli", db_path=".avocado/db.sqlite")
        >>> result = db.compile("How does authentication work?")
    """

    def __init__(
        self,
        url: Optional[str] = None,
        mode: Literal["http", "cli"] = "http",
        db_path: Optional[str] = None,
        cli_binary: Optional[str] = None
    ):
        """Initialize AvocadoDB client.

        Args:
            url: Base URL of AvocadoDB server (HTTP mode only, default: http://localhost:8765)
            mode: "http" for server mode, "cli" for direct CLI calls (default: "http")
            db_path: Database path for CLI mode (default: ".avocado/db.sqlite" in current directory)
            cli_binary: Path to avocado CLI binary (auto-detected if None)
        """
        self.mode = mode
        
        if mode == "cli":
            # CLI mode - find binary and set db path
            self.cli_binary = cli_binary or self._find_cli_binary()
            if not self.cli_binary:
                raise RuntimeError(
                    "AvocadoDB CLI binary not found. "
                    "Install it or set AVOCADODB_CLI_BINARY environment variable. "
                    "Build from source: cargo build --release (binary at target/release/avocado)"
                )
            
            # Default db_path is per-directory (for multi-repo support)
            # Use absolute path from current working directory (not avocado repo)
            if db_path is None:
                db_path = ".avocado/db.sqlite"
            
            # Resolve relative to current working directory (where user is running from)
            if not Path(db_path).is_absolute():
                self.db_path = Path.cwd() / db_path
            else:
                self.db_path = Path(db_path).expanduser()
            
            self.db_path = self.db_path.resolve()
            
            # Ensure parent directory exists
            self.db_path.parent.mkdir(parents=True, exist_ok=True)
            
            # Auto-initialize database if it doesn't exist
            if not self.db_path.exists():
                self._init_database()
            
            self.session = None  # Not used in CLI mode
        else:
            # HTTP mode (MongoDB-style daemon)
            if url is None:
                url = os.environ.get("AVOCADODB_URL", "http://localhost:8765")
            self.url = url.rstrip('/')
            self.session = requests.Session()
            self.cli_binary = None
            self.db_path = None
            # Auto-detect project path (current working directory)
            self.project_path = str(Path.cwd().resolve())
    
    def _find_cli_binary(self) -> Optional[str]:
        """Find AvocadoDB CLI binary in common locations.
        
        Returns:
            Path to binary if found, None otherwise
        """
        # Check environment variable first
        env_binary = os.environ.get("AVOCADODB_CLI_BINARY")
        if env_binary:
            env_path = Path(env_binary).expanduser()
            if env_path.exists() and env_path.is_file():
                return str(env_path.resolve())
            # Also check if it's in PATH
            import shutil
            found = shutil.which(env_binary)
            if found:
                return found
        
        # Search common locations
        import shutil
        
        # First check if 'avocado' is in PATH (most common case)
        found_in_path = shutil.which("avocado")
        if found_in_path:
            return found_in_path
        
        # Then check specific locations
        possible_paths = [
            # In avocado repo (if we're running from there)
            Path(__file__).parent.parent.parent.parent / "target/release/avocado",
            Path(__file__).parent.parent.parent.parent.parent / "target/release/avocado",
            # In current directory
            Path.cwd() / "target/release/avocado",
            # In parent directories (up to 3 levels)
            Path.cwd().parent / "target/release/avocado",
            Path.cwd().parent.parent / "target/release/avocado",
            Path.cwd().parent.parent.parent / "target/release/avocado",
            # In home directory
            Path.home() / ".avocadodb/avocado",
            # System-wide
            Path("/usr/local/bin/avocado"),
        ]
        
        for path in possible_paths:
            if path.exists() and path.is_file():
                return str(path.resolve())
        
        return None
    
    def _init_database(self):
        """Initialize the database if it doesn't exist."""
        try:
            cmd = [
                self.cli_binary,
                "init",
                "--path", str(self.db_path),
            ]
            result = subprocess.run(
                cmd,
                capture_output=True,
                text=True,
                check=True,
                cwd=Path.cwd()  # Run from current working directory
            )
        except subprocess.CalledProcessError as e:
            # If init fails, database might be created on first use anyway
            # (Database::new() creates it automatically)
            pass

    def compile(
        self,
        query: str,
        budget: int = 8000,
        semantic_weight: float = 0.7,
        lexical_weight: float = 0.3,
        mmr_lambda: float = 0.5,
        enable_mmr: bool = True
    ) -> WorkingSet:
        """Compile a deterministic context for a query.

        Args:
            query: Search query
            budget: Token budget for compiled context (default: 8000)
            semantic_weight: Weight for semantic search (default: 0.7)
            lexical_weight: Weight for lexical search (default: 0.3)
            mmr_lambda: MMR diversity parameter, 0.0-1.0 (default: 0.5)
                - Higher (0.7-1.0) = more relevant but potentially redundant
                - Lower (0.0-0.3) = more diverse but potentially less relevant
            enable_mmr: Enable MMR diversification (default: True)

        Returns:
            WorkingSet with compiled context and citations

        Raises:
            requests.HTTPError: If API request fails (HTTP mode)
            subprocess.CalledProcessError: If CLI call fails (CLI mode)

        Example:
            >>> result = db.compile("authentication", budget=8000)
            >>> print(f"Compiled {len(result.spans)} spans")
            >>> print(f"Hash: {result.deterministic_hash()}")
        """
        if self.mode == "cli":
            return self._compile_cli(
                query=query,
                budget=budget,
                semantic_weight=semantic_weight,
                lexical_weight=lexical_weight,
                mmr_lambda=mmr_lambda,
                enable_mmr=enable_mmr
            )
        else:
            return self._compile_http(
                query=query,
                budget=budget,
                semantic_weight=semantic_weight,
                lexical_weight=lexical_weight,
                mmr_lambda=mmr_lambda,
                enable_mmr=enable_mmr
            )
    
    def _compile_cli(
        self,
        query: str,
        budget: int,
        semantic_weight: float,
        lexical_weight: float,
        mmr_lambda: float,
        enable_mmr: bool
    ) -> WorkingSet:
        """Compile using CLI mode (direct binary call)."""
        # Build CLI command
        cmd = [
            self.cli_binary,
            "compile",
            query,
            "--budget", str(budget),
            "--json",
            "--db-path", str(self.db_path),
        ]
        
        # Add optional parameters if CLI supports them
        # Note: CLI might not support all parameters yet, so we'll add them as they become available
        
        # Execute CLI from current working directory (not avocado repo)
        result = subprocess.run(
            cmd,
            capture_output=True,
            text=True,
            check=True,
            cwd=Path.cwd()  # Ensure we run from user's project directory
        )
        
        # Parse JSON output from CLI
        import json
        try:
            data = json.loads(result.stdout)
            return WorkingSet(data)
        except json.JSONDecodeError as e:
            # Fallback: if JSON parsing fails, raise with helpful error
            raise ValueError(f"Failed to parse CLI JSON output: {e}\nOutput: {result.stdout[:500]}")
    
    def _compile_http(
        self,
        query: str,
        budget: int,
        semantic_weight: float,
        lexical_weight: float,
        mmr_lambda: float,
        enable_mmr: bool
    ) -> WorkingSet:
        """Compile using HTTP mode (MongoDB-style daemon, auto-detects project)."""
        response = self.session.post(
            f"{self.url}/compile",
            json={
                "query": query,
                "token_budget": budget,
                "semantic_weight": semantic_weight,
                "lexical_weight": lexical_weight,
                "mmr_lambda": mmr_lambda,
                "enable_mmr": enable_mmr,
                "project": self.project_path  # Auto-detect project from current directory
            }
        )
        response.raise_for_status()
        data = response.json()
        # Server returns {"working_set": {...}}, extract it
        if "working_set" in data:
            return WorkingSet(data["working_set"])
        # Fallback: assume direct working_set structure
        return WorkingSet(data)

    def ingest(self, path: str, content: Optional[str] = None, recursive: bool = False) -> Dict[str, Any]:
        """Ingest a document into the database.

        Args:
            path: Document path (used as artifact identifier) or directory path
            content: Document content (if None, reads from file)
            recursive: If True and path is directory, ingest recursively (CLI mode only)

        Returns:
            Dict with artifact_id and span count

        Raises:
            requests.HTTPError: If API request fails (HTTP mode)
            subprocess.CalledProcessError: If CLI call fails (CLI mode)
            FileNotFoundError: If content is None and path doesn't exist

        Example:
            >>> result = db.ingest("docs/auth.md")
            >>> print(f"Created {result['spans']} spans")
        """
        if self.mode == "cli":
            return self._ingest_cli(path, recursive)
        else:
            return self._ingest_http(path, content)
    
    def _ingest_cli(self, path: str, recursive: bool) -> Dict[str, Any]:
        """Ingest using CLI mode."""
        # Resolve path relative to current working directory
        if not Path(path).is_absolute():
            path_obj = Path.cwd() / path
        else:
            path_obj = Path(path).expanduser()
        
        path_obj = path_obj.resolve()
        
        if not path_obj.exists():
            raise FileNotFoundError(f"Path not found: {path}")
        
        cmd = [
            self.cli_binary,
            "ingest",
            str(path_obj),
            "--db-path", str(self.db_path),
        ]
        
        if recursive:
            cmd.append("--recursive")
        
        result = subprocess.run(
            cmd,
            capture_output=True,
            text=True,
            check=True,
            cwd=Path.cwd()  # Run from current working directory
        )
        
        # Parse output to extract stats
        # CLI outputs: "✓ Indexed X files → Y spans (Z successful, W failed/skipped)"
        output = result.stdout
        spans_created = 0
        files_indexed = 0
        
        # Try to extract numbers from output
        import re
        # Match pattern like "Indexed 42 files → 387 spans"
        match = re.search(r'Indexed\s+(\d+)\s+files\s+→\s+(\d+)\s+spans', output)
        if match:
            files_indexed = int(match.group(1))
            spans_created = int(match.group(2))
        
        return {
            "success": True,
            "path": str(path_obj),
            "spans_created": spans_created,
            "files_indexed": files_indexed,
            "output": output
        }
    
    def _ingest_http(self, path: str, content: Optional[str]) -> Dict[str, Any]:
        """Ingest using HTTP mode."""
        if content is None:
            with open(path, 'r', encoding='utf-8') as f:
                content = f.read()

        response = self.session.post(
            f"{self.url}/ingest",
            json={
                "path": path,
                "content": content,
                "project": self.project_path  # Auto-detect project from current directory
            }
        )
        response.raise_for_status()
        return response.json()

    def stats(self) -> Dict[str, int]:
        """Get database statistics.

        Returns:
            Dict with artifacts, spans, and tokens counts

        Example:
            >>> stats = db.stats()
            >>> print(f"Database: {stats['spans']} spans, {stats['tokens']} tokens")
        """
        if self.mode == "cli":
            return self._stats_cli()
        else:
            return self._stats_http()
    
    def _stats_cli(self) -> Dict[str, int]:
        """Get stats using CLI mode."""
        cmd = [
            self.cli_binary,
            "stats",
            "--db-path", str(self.db_path),
        ]
        
        result = subprocess.run(
            cmd,
            capture_output=True,
            text=True,
            check=True,
            cwd=Path.cwd()  # Run from current working directory
        )
        
        # Parse text output (CLI doesn't support --json yet)
        # Format: "Artifacts: X", "Spans: Y", "Tokens: Z"
        artifacts = 0
        spans = 0
        tokens = 0
        
        for line in result.stdout.split('\n'):
            if 'Artifacts:' in line:
                try:
                    artifacts = int(line.split('Artifacts:')[1].strip().split()[0])
                except (ValueError, IndexError):
                    pass
            elif 'Spans:' in line:
                try:
                    spans = int(line.split('Spans:')[1].strip().split()[0])
                except (ValueError, IndexError):
                    pass
            elif 'Tokens:' in line:
                try:
                    tokens = int(line.split('Tokens:')[1].strip().split()[0])
                except (ValueError, IndexError):
                    pass
        
        return {
            "artifacts_count": artifacts,
            "spans_count": spans,
            "total_tokens": tokens
        }
    
    def _stats_http(self) -> Dict[str, int]:
        """Get stats using HTTP mode (MongoDB-style daemon, auto-detects project)."""
        response = self.session.get(
            f"{self.url}/stats",
            params={"project": self.project_path}  # Include project path
        )
        response.raise_for_status()
        return response.json()

    def ask(
        self,
        query: str,
        llm: str = "none",  # Default: skip LLM for speed. Use "auto" or "local" to enable.
        budget: int = 8000,
        max_new_tokens: int = 150,
        deterministic: bool = True,
        **compile_kwargs
    ) -> str:
        """Ask a question and get context or a natural language answer.
        
        By default, returns raw context (fast, ~300-500ms). Set llm="auto" or "local"
        to enable TinyLlama summarization (slower, ~5-8s on CPU, but provides natural language answers).
        
        Falls back to returning context text if LLM is not available or llm="none".
        
        Args:
            query: The question to ask
            llm: LLM mode - "auto" (try local, fallback to context), 
                 "local" (require TinyLlama), "none" (just context)
            budget: Token budget for context compilation (default: 8000)
            max_new_tokens: Maximum tokens for answer generation (default: 150)
            deterministic: Use deterministic generation (default: True)
            **compile_kwargs: Additional arguments passed to compile()
            
        Returns:
            Natural language answer as string, or context text if LLM unavailable
            
        Example:
            >>> db = AvocadoDB()
            >>> answer = db.ask("How does authentication work?")
            >>> print(answer)
        """
        # Get context
        context = self.compile(query, budget=budget, **compile_kwargs)
        
        # Handle LLM modes
        if llm == "none":
            return context.text
        
        if llm == "local":
            # Require LLM
            try:
                from .llm import generate_answer
                return generate_answer(
                    query=query,
                    context_text=context.text,
                    max_new_tokens=max_new_tokens,
                    deterministic=deterministic,
                )
            except ImportError:
                raise ImportError(
                    "TinyLlama not available. Install with: pip install avocadodb[llm]"
                )
        
        # llm == "auto" - try LLM, fallback to context
        try:
            from .llm import generate_answer
            return generate_answer(
                query=query,
                context_text=context.text,
                max_new_tokens=max_new_tokens,
                deterministic=deterministic,
            )
        except ImportError:
            # No LLM installed, return context
            return context.text
        except Exception as e:
            # LLM failed, fallback to context
            import warnings
            warnings.warn(f"LLM generation failed: {e}. Returning context text.")
            return context.text

    def close(self):
        """Close the HTTP session (no-op in CLI mode)."""
        if self.session:
            self.session.close()

    def __enter__(self):
        """Context manager entry."""
        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        """Context manager exit."""
        self.close()
