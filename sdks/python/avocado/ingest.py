"""Smart Auto-Ingestion for AvocadoDB.

Framework-agnostic intelligent ingestion with:
- Project type detection (Python, Node, Rust, Go, etc.)
- Language-specific file patterns
- Binary file filtering
- Recursive directory traversal

Example:
    >>> from avocado import AutoIngest
    >>> ingester = AutoIngest()
    >>> ingester.ingest_project(".")  # Auto-detects project type
"""

import subprocess
from pathlib import Path
from typing import Optional


class ProjectType:
    """Detected project types with their file patterns."""

    PYTHON = "python"
    JAVASCRIPT = "javascript"
    TYPESCRIPT = "typescript"
    RUST = "rust"
    GO = "go"
    JAVA = "java"
    CPP = "cpp"
    RUBY = "ruby"
    PHP = "php"
    UNKNOWN = "unknown"


class AutoIngest:
    """Smart auto-ingestion with project type detection.

    Automatically detects project type and ingests relevant files:
    - Documentation (README, docs/)
    - Source code (language-specific patterns)
    - Excludes build artifacts and dependencies

    Example:
        >>> from avocado import AutoIngest
        >>> ingester = AutoIngest()
        >>> result = ingester.ingest_project(".", max_files=100)
        >>> print(f"Ingested {result['ingested']} files")
    """

    def __init__(self, ingest_binary: Optional[Path] = None):
        """Initialize auto-ingester.

        Args:
            ingest_binary: Path to avocado ingest binary (auto-detected if None)
        """
        self.ingest_binary = ingest_binary or self._find_ingest_binary()

    def _find_ingest_binary(self) -> Optional[Path]:
        """Find avocado ingest binary.

        Returns:
            Path to ingest binary if found, None otherwise
        """
        possible_paths = [
            Path.cwd() / "target/release/avocado",
            Path.cwd().parent / "target/release/avocado",
            Path.cwd().parent.parent / "target/release/avocado",
            Path.home() / ".avocadodb/repo/target/release/avocado",
            Path("/usr/local/bin/avocado"),
        ]

        for path in possible_paths:
            if path.exists() and path.is_file():
                return path

        return None

    def detect_project_type(self, path: Path) -> str:
        """Detect project type from marker files.

        Args:
            path: Project directory to analyze

        Returns:
            Project type constant (e.g., ProjectType.PYTHON)

        Example:
            >>> project_type = ingester.detect_project_type(Path("."))
            >>> print(f"Detected: {project_type}")
        """
        # Python
        if (path / "pyproject.toml").exists() or (path / "setup.py").exists() or (path / "requirements.txt").exists():
            return ProjectType.PYTHON

        # JavaScript/TypeScript (Node.js)
        if (path / "package.json").exists():
            package_json = path / "package.json"
            try:
                import json
                with open(package_json) as f:
                    data = json.load(f)
                    # Check if TypeScript is in dependencies
                    deps = {**data.get("dependencies", {}), **data.get("devDependencies", {})}
                    if "typescript" in deps or "@types/node" in deps:
                        return ProjectType.TYPESCRIPT
            except:
                pass
            return ProjectType.JAVASCRIPT

        # Rust
        if (path / "Cargo.toml").exists():
            return ProjectType.RUST

        # Go
        if (path / "go.mod").exists():
            return ProjectType.GO

        # Java/Kotlin
        if (path / "pom.xml").exists() or (path / "build.gradle").exists():
            return ProjectType.JAVA

        # C/C++
        if (path / "Makefile").exists() or (path / "CMakeLists.txt").exists():
            return ProjectType.CPP

        # Ruby
        if (path / "Gemfile").exists():
            return ProjectType.RUBY

        # PHP
        if (path / "composer.json").exists():
            return ProjectType.PHP

        return ProjectType.UNKNOWN

    def get_patterns_for_project(self, project_type: str) -> list[str]:
        """Get file patterns for a project type.

        Args:
            project_type: Project type constant

        Returns:
            List of glob patterns for source files

        Example:
            >>> patterns = ingester.get_patterns_for_project(ProjectType.PYTHON)
            >>> # Returns: ["**/*.py", "**/*.pyi"]
        """
        patterns_map = {
            ProjectType.PYTHON: ["**/*.py", "**/*.pyi"],
            ProjectType.JAVASCRIPT: ["**/*.js", "**/*.jsx", "**/*.mjs"],
            ProjectType.TYPESCRIPT: ["**/*.ts", "**/*.tsx", "**/*.d.ts"],
            ProjectType.RUST: ["**/*.rs"],
            ProjectType.GO: ["**/*.go"],
            ProjectType.JAVA: ["**/*.java", "**/*.kt"],
            ProjectType.CPP: ["**/*.c", "**/*.cpp", "**/*.h", "**/*.hpp"],
            ProjectType.RUBY: ["**/*.rb"],
            ProjectType.PHP: ["**/*.php"],
            ProjectType.UNKNOWN: ["**/*.py", "**/*.js", "**/*.ts", "**/*.java", "**/*.go", "**/*.rs"],
        }

        return patterns_map.get(project_type, [])

    def get_documentation_patterns(self) -> list[str]:
        """Get common documentation file patterns.

        Returns:
            List of glob patterns for documentation files
        """
        return [
            "README.md",
            "QUICKSTART.md",
            "CONTRIBUTING.md",
            "CHANGELOG.md",
            "docs/**/*.md",
            "*.md",
        ]

    def ingest_project(
        self,
        path: str | Path = ".",
        max_files: int = 1000,
        include_source: bool = True,
        include_docs: bool = True,
    ) -> dict:
        """Intelligently ingest a project directory.

        Auto-detects project type and ingests relevant files:
        - Documentation (always included if include_docs=True)
        - Source code (language-specific, if include_source=True)
        - Excludes: node_modules, .git, venv, build artifacts

        Args:
            path: Project directory to ingest (default: current directory)
            max_files: Maximum number of files to ingest (default: 1000)
            include_source: Include source code files (default: True)
            include_docs: Include documentation files (default: True)

        Returns:
            Dict with:
            - project_type: Detected project type
            - ingested: Number of files ingested
            - skipped: Number of files skipped
            - patterns: Patterns used

        Example:
            >>> result = ingester.ingest_project(".", max_files=100)
            >>> print(f"Ingested {result['ingested']} {result['project_type']} files")
        """
        if not self.ingest_binary or not self.ingest_binary.exists():
            print("⚠️  Avocado ingest binary not found")
            return {"ingested": 0, "skipped": 0, "project_type": "unknown", "patterns": []}

        path = Path(path)
        if not path.exists():
            print(f"⚠️  Path not found: {path}")
            return {"ingested": 0, "skipped": 0, "project_type": "unknown", "patterns": []}

        # Detect project type
        project_type = self.detect_project_type(path)

        # Build patterns
        patterns = []
        if include_docs:
            patterns.extend(self.get_documentation_patterns())
        if include_source:
            patterns.extend(self.get_patterns_for_project(project_type))

        # Find matching files
        paths_to_ingest = []
        for pattern in patterns:
            try:
                matching = list(path.glob(pattern))
                # Exclude common directories
                matching = [
                    p for p in matching
                    if p.is_file() and not any(part in p.parts for part in [
                        "node_modules", ".git", "venv", ".venv",
                        "__pycache__", "target", "build", "dist",
                        ".next", ".cache", ".tox", "vendor"
                    ])
                ]
                paths_to_ingest.extend(matching[:500])  # Increased limit per pattern
            except:
                pass

        # Remove duplicates and limit total
        paths_to_ingest = list(set(paths_to_ingest))[:max_files]

        if not paths_to_ingest:
            print("   No files found to ingest")
            return {"ingested": 0, "skipped": 0, "project_type": project_type, "patterns": patterns}

        # Ingest each file
        ingested = 0
        skipped = 0
        for file_path in paths_to_ingest:
            try:
                result = subprocess.run(
                    [str(self.ingest_binary), "ingest", str(file_path)],
                    capture_output=True,
                    timeout=30,
                )
                if result.returncode == 0:
                    ingested += 1
                else:
                    skipped += 1
            except:
                skipped += 1

        print(f"✅ Auto-ingested {ingested} files ({project_type} project)")

        return {
            "ingested": ingested,
            "skipped": skipped,
            "project_type": project_type,
            "patterns": patterns,
        }

    def ingest_file(self, path: str | Path) -> bool:
        """Ingest a single file.

        Args:
            path: File to ingest

        Returns:
            True if ingestion succeeded, False otherwise

        Example:
            >>> success = ingester.ingest_file("README.md")
        """
        if not self.ingest_binary or not self.ingest_binary.exists():
            return False

        try:
            result = subprocess.run(
                [str(self.ingest_binary), "ingest", str(path)],
                capture_output=True,
                timeout=30,
            )
            return result.returncode == 0
        except:
            return False


__all__ = ["AutoIngest", "ProjectType"]
