"""Background File Monitoring for AvocadoDB.

Framework-agnostic file watcher that detects changes and triggers re-ingestion.
Useful for keeping the AvocadoDB index up-to-date during development.

Example:
    >>> from avocado import FileMonitor
    >>> monitor = FileMonitor(interval_seconds=30)
    >>> monitor.start_monitoring(["**/*.py", "**/*.md"])
    >>> # Monitor runs in background, auto-re-ingests changed files
"""

import subprocess
import threading
from pathlib import Path
from typing import Callable, Optional


class FileMonitor:
    """Background file watcher for automatic re-ingestion.

    Monitors files matching specified patterns and automatically
    re-ingests them when changes are detected. Runs in a background
    thread with configurable polling interval.

    Example:
        >>> from avocado import FileMonitor
        >>> monitor = FileMonitor(interval_seconds=30)
        >>> monitor.start_monitoring(["docs/**/*.md", "src/**/*.py"])
        >>> # Files are automatically re-ingested when modified
        >>> monitor.stop_monitoring()
    """

    def __init__(self, interval_seconds: int = 30, ingest_binary: Optional[Path] = None):
        """Initialize file monitor.

        Args:
            interval_seconds: How often to check for changes (default: 30)
            ingest_binary: Path to avocado ingest binary (auto-detected if None)
        """
        self.interval_seconds = interval_seconds
        self.ingest_binary = ingest_binary or self._find_ingest_binary()
        self._monitor_thread: Optional[threading.Thread] = None
        self._stop_flag = threading.Event()
        self._last_modified: dict[Path, float] = {}
        self._on_change_callback: Optional[Callable[[list[Path]], None]] = None

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

    def on_change(self, callback: Callable[[list[Path]], None]):
        """Register callback for file change events.

        Args:
            callback: Function called with list of changed files

        Example:
            >>> def handle_changes(files):
            ...     print(f"Changed: {files}")
            >>> monitor.on_change(handle_changes)
        """
        self._on_change_callback = callback

    def _monitor_loop(self, patterns: list[str], cwd: Path):
        """Background monitoring loop.

        Args:
            patterns: Glob patterns to monitor
            cwd: Current working directory
        """
        while not self._stop_flag.is_set():
            try:
                # Wait for interval (allows early exit via stop_flag)
                if self._stop_flag.wait(timeout=self.interval_seconds):
                    break

                if not self.ingest_binary or not self.ingest_binary.exists():
                    continue

                # Find files matching patterns
                paths_to_check = []
                for pattern in patterns:
                    try:
                        matching = list(cwd.glob(pattern))
                        # Exclude common build/cache directories
                        matching = [
                            p for p in matching
                            if not any(part in p.parts for part in [
                                "node_modules", ".git", "venv", ".venv",
                                "__pycache__", "target", "build", "dist",
                                ".next", ".cache", ".tox"
                            ])
                        ]
                        paths_to_check.extend(matching[:100])  # Limit per pattern
                    except:
                        pass

                # Check for modified files
                changed_files = []
                for path in paths_to_check:
                    if not path.is_file():
                        continue

                    try:
                        mtime = path.stat().st_mtime
                        last_mtime = self._last_modified.get(path, 0)

                        # File is new or modified
                        if mtime > last_mtime:
                            # Re-ingest file
                            result = subprocess.run(
                                [str(self.ingest_binary), "ingest", str(path)],
                                capture_output=True,
                                timeout=10,
                            )
                            if result.returncode == 0:
                                self._last_modified[path] = mtime
                                changed_files.append(path)
                    except:
                        pass

                # Notify callback if files changed
                if changed_files:
                    if self._on_change_callback:
                        self._on_change_callback(changed_files)
                    else:
                        print(f"🥑 Background: Re-ingested {len(changed_files)} changed files")

            except Exception:
                pass  # Silently continue on errors

    def start_monitoring(self, patterns: list[str], cwd: Optional[Path] = None):
        """Start background file monitoring.

        Args:
            patterns: Glob patterns to monitor (e.g., ["**/*.py", "docs/**/*.md"])
            cwd: Working directory to monitor (default: current directory)

        Example:
            >>> monitor.start_monitoring([
            ...     "docs/**/*.md",
            ...     "src/**/*.py",
            ...     "README.md"
            ... ])
        """
        if self._monitor_thread and self._monitor_thread.is_alive():
            print("⚠️  Monitor already running")
            return

        cwd = cwd or Path.cwd()
        self._stop_flag.clear()
        self._monitor_thread = threading.Thread(
            target=self._monitor_loop,
            args=(patterns, cwd),
            daemon=True,
            name="AvocadoDB-FileMonitor"
        )
        self._monitor_thread.start()

    def stop_monitoring(self):
        """Stop background file monitoring.

        Example:
            >>> monitor.stop_monitoring()
        """
        if self._monitor_thread:
            self._stop_flag.set()
            self._monitor_thread.join(timeout=2)
            self._monitor_thread = None

    def is_monitoring(self) -> bool:
        """Check if monitoring is active.

        Returns:
            True if monitor thread is running, False otherwise
        """
        return self._monitor_thread is not None and self._monitor_thread.is_alive()


__all__ = ["FileMonitor"]
