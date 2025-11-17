#!/usr/bin/env python3
"""
AvocadoDB CLI - Ask command

This script is called by the Rust CLI to use the Python SDK's ask() method.
"""

import sys
import os
import json
from pathlib import Path
from urllib.parse import urlparse

# Add SDK to path
sdk_path = Path(__file__).parent.parent.parent / "sdks" / "python"
sys.path.insert(0, str(sdk_path))

try:
    from avocado import AvocadoDB, get_manager
except ImportError:
    print("❌ AvocadoDB Python SDK not found. Install with: pip install -e sdks/python", file=sys.stderr)
    sys.exit(1)


def main():
    import argparse
    
    parser = argparse.ArgumentParser(description="Ask a question using AvocadoDB + TinyLlama")
    parser.add_argument("query", help="The question to ask")
    parser.add_argument("--url", default="http://localhost:8765", help="AvocadoDB server URL")
    parser.add_argument("--budget", type=int, default=8000, help="Token budget for context")
    parser.add_argument("--llm", choices=["auto", "local", "none"], default="auto",
                       help="LLM mode: auto (try local, fallback), local (require), none (just context)")
    parser.add_argument("--max-tokens", type=int, default=150, help="Max tokens for answer generation")
    parser.add_argument("--json", action="store_true", help="Output as JSON")
    
    args = parser.parse_args()
    
    try:
        # Auto-start local server as a background daemon when using a localhost URL.
        # This matches the \"invisible server\" UX: the user just runs `avocado ask ...`
        # and we ensure the HTTP server (with in-memory HNSW index) is running.
        parsed = urlparse(args.url)
        if parsed.hostname in ("localhost", "127.0.0.1"):
            # Propagate URL so the Python manager uses the same host/port
            os.environ.setdefault("AVOCADODB_URL", args.url)
            port = parsed.port or 8765
            manager = get_manager(auto_start=True, port=port)
            manager.ensure_running()

        # Connect to AvocadoDB
        db = AvocadoDB(url=args.url)
        
        # Ask question
        answer = db.ask(
            query=args.query,
            llm=args.llm,
            budget=args.budget,
            max_new_tokens=args.max_tokens,
            deterministic=True,
        )
        
        if args.json:
            # Output as JSON
            result = {
                "query": args.query,
                "answer": answer,
                "llm_mode": args.llm,
            }
            print(json.dumps(result, indent=2))
        else:
            # Output answer
            print(answer)
            
    except Exception as e:
        print(f"❌ Error: {e}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
