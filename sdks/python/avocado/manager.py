"""AvocadoDB Server Lifecycle Management.

Framework-agnostic server management for AvocadoDB.
Handles auto-start, health checks, and daemon mode persistence.

Example:
    >>> from avocado import AvocadoDBManager
    >>> manager = AvocadoDBManager(auto_start=True)
    >>> manager.ensure_running()
    >>> stats = manager.get_stats()
"""

import os
import subprocess
import time
import urllib.parse
from pathlib import Path
from typing import Optional

import requests


class AvocadoDBManager:
    """Manages AvocadoDB server lifecycle (framework-agnostic).

    Features:
    - Auto-detection of binary location
    - Auto-installation from source
    - Daemon mode (server persists after CLI exit)
    - Health checks and stats
    - Environment variable configuration

    Example:
        >>> manager = AvocadoDBManager(auto_start=True)
        >>> if manager.ensure_running():
        ...     print("Server ready!")
        >>> stats = manager.get_stats()
        >>> print(f"Indexed: {stats['artifacts_count']} docs")
    """

    def __init__(self, auto_start: bool = True, port: int = 8765):
        """Initialize AvocadoDB manager.

        Args:
            auto_start: Automatically start server if not running
            port: Server port (default: 8765, or from AVOCADODB_URL)
        """
        # Allow override via environment variable
        env_url = os.environ.get("AVOCADODB_URL")
        if env_url:
            parsed = urllib.parse.urlparse(env_url)
            self.server_url = env_url
            self.port = parsed.port or port
        else:
            self.port = port
            self.server_url = f"http://localhost:{self.port}"

        self.auto_start = auto_start
        self.server_process: Optional[subprocess.Popen] = None
        self.binary_path: Optional[Path] = None

        # Find or install AvocadoDB
        if self.auto_start:
            self._ensure_available()

    def _find_binary(self) -> Optional[Path]:
        """Find AvocadoDB binary in common locations.

        Searches:
        - Current directory (./target/release/avocado-server)
        - Parent directories (up to 3 levels)
        - Home directory (~/.avocadodb/avocado-server)
        - System-wide (/usr/local/bin/avocado-server)

        Returns:
            Path to binary if found, None otherwise
        """
        possible_paths = [
            # In current directory
            Path.cwd() / "target/release/avocado-server",
            # In parent directories (up to 3 levels)
            Path.cwd().parent / "target/release/avocado-server",
            Path.cwd().parent.parent / "target/release/avocado-server",
            Path.cwd().parent.parent.parent / "target/release/avocado-server",
            # In home directory
            Path.home() / ".avocadodb/avocado-server",
            # System-wide
            Path("/usr/local/bin/avocado-server"),
        ]

        for path in possible_paths:
            if path.exists() and path.is_file():
                return path

        return None

    def _install_binary(self) -> Optional[Path]:
        """Install AvocadoDB binary automatically.

        Downloads pre-built binary or builds from source.
        Only happens once, then cached in ~/.avocadodb/

        Returns:
            Path to installed binary if successful, None otherwise
        """
        install_dir = Path.home() / ".avocadodb"
        install_dir.mkdir(exist_ok=True)

        print("🥑 Installing AvocadoDB...")
        print("   This only happens once, please wait...")

        try:
            # Clone and build from source
            repo_dir = install_dir / "repo"

            if not repo_dir.exists():
                print("   Cloning repository...")
                subprocess.run(
                    [
                        "git",
                        "clone",
                        "https://github.com/servesys-labs/avacadodb.git",
                        str(repo_dir),
                    ],
                    check=True,
                    capture_output=True,
                )

            print("   Building (this may take 2-3 minutes)...")
            subprocess.run(
                ["cargo", "build", "--release"],
                cwd=repo_dir,
                check=True,
                capture_output=True,
            )

            # Copy binary to install location
            binary_src = repo_dir / "target/release/avocado-server"
            binary_dst = install_dir / "avocado-server"

            if binary_src.exists():
                import shutil

                shutil.copy2(binary_src, binary_dst)
                binary_dst.chmod(0o755)
                print(f"✅ Installed to {binary_dst}")
                return binary_dst

        except Exception as e:
            print(f"⚠️  Auto-install failed: {e}")
            print("   Please install manually:")
            print("   git clone https://github.com/servesys-labs/avacadodb")
            print("   cd avacadodb && cargo build --release")

        return None

    def _ensure_available(self):
        """Ensure AvocadoDB binary is available (find or install)."""
        self.binary_path = self._find_binary()

        if not self.binary_path and self.auto_start:
            # Try to install automatically
            self.binary_path = self._install_binary()

    def is_running(self) -> bool:
        """Check if AvocadoDB server is running.

        Returns:
            True if server is reachable, False otherwise
        """
        try:
            response = requests.get(f"{self.server_url}/stats", timeout=1)
            return response.status_code == 200
        except:
            return False

    def start_server(self) -> bool:
        """Start AvocadoDB server as background daemon.

        Features:
        - Detached process (survives CLI exit)
        - Port configuration via environment variable
        - Auto-retry with health checks

        Returns:
            True if server started successfully, False otherwise
        """
        if self.is_running():
            # Server already running - no need to start
            return True

        if not self.binary_path:
            print("⚠️  AvocadoDB binary not found")
            return False

        print(f"🥑 Starting AvocadoDB server on port {self.port}...")

        try:
            # Start server in background with PORT env var
            env = os.environ.copy()
            env["PORT"] = str(self.port)

            self.server_process = subprocess.Popen(
                [str(self.binary_path)],
                env=env,
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
                start_new_session=True,  # Detach from parent (daemon mode)
            )

            # Wait for server to be ready (max 5 seconds)
            for _ in range(10):
                time.sleep(0.5)
                if self.is_running():
                    print("✅ Server started (daemon mode - stays running)")
                    return True

            print("⚠️  Server failed to start")
            return False

        except Exception as e:
            print(f"⚠️  Failed to start server: {e}")
            return False

    def stop_server(self):
        """Stop AvocadoDB server subprocess.

        Note: Only stops the server if it was started by this manager.
        Servers started externally will continue running.
        """
        if self.server_process:
            try:
                self.server_process.terminate()
                self.server_process.wait(timeout=5)
                print("🥑 AvocadoDB server stopped")
            except:
                self.server_process.kill()

    def ensure_running(self) -> bool:
        """Ensure AvocadoDB server is running (start if needed).

        Returns:
            True if server is available, False otherwise

        Example:
            >>> manager = AvocadoDBManager()
            >>> if manager.ensure_running():
            ...     # Server is ready, proceed with queries
            ...     pass
        """
        if self.is_running():
            return True

        if self.auto_start:
            return self.start_server()

        return False

    def get_stats(self) -> dict:
        """Get database statistics from server.

        Returns:
            Dict with stats like:
            - artifacts_count: Number of documents indexed
            - spans_count: Total number of spans
            - total_tokens: Total tokens indexed
            Returns empty dict if server is not available.

        Example:
            >>> stats = manager.get_stats()
            >>> print(f"Indexed: {stats['artifacts_count']} docs")
        """
        if not self.is_running():
            return {}

        try:
            response = requests.get(f"{self.server_url}/stats", timeout=2)
            if response.status_code == 200:
                return response.json()
        except:
            pass

        return {}

    def health_check(self) -> dict:
        """Comprehensive health check.

        Returns:
            Dict with:
            - running: Server status
            - binary_found: Binary availability
            - stats: Server statistics (if running)

        Example:
            >>> health = manager.health_check()
            >>> if health['running']:
            ...     print(f"Server healthy: {health['stats']}")
        """
        return {
            "running": self.is_running(),
            "binary_found": self.binary_path is not None,
            "server_url": self.server_url,
            "stats": self.get_stats() if self.is_running() else {},
        }


# Global singleton instance
_manager: Optional[AvocadoDBManager] = None


def get_manager(auto_start: bool = True, port: int = 8765) -> AvocadoDBManager:
    """Get or create global AvocadoDB manager instance.

    Args:
        auto_start: Auto-start server if not running (default: True)
        port: Server port (default: 8765)

    Returns:
        Global manager instance

    Example:
        >>> from avocado import get_manager
        >>> manager = get_manager()
        >>> manager.ensure_running()
    """
    global _manager
    if _manager is None:
        # Check environment variable for auto-start override
        auto_start_env = os.environ.get("AVOCADODB_AUTO_START", "true").lower() == "true"
        _manager = AvocadoDBManager(auto_start=auto_start and auto_start_env, port=port)

    return _manager


__all__ = ["AvocadoDBManager", "get_manager"]
