#!/usr/bin/env python3
"""
Example: Using AvocadoDB's ask() method with TinyLlama

This demonstrates the new v2.0 feature for getting natural language
answers from your codebase.

Requirements:
    pip install avocadodb[llm]  # Includes TinyLlama support
    # Or: pip install avocadodb transformers torch

Setup:
    1. Start AvocadoDB server: ./target/release/avocado-server
    2. Ingest documents: ./target/release/avocado ingest test-docs/ --recursive
    3. Run: python examples/ask_example.py
"""

import sys
from pathlib import Path

# Add parent directory to path
sys.path.insert(0, str(Path(__file__).parent.parent))

from avocado import AvocadoDB


def main():
    print("🥑 AvocadoDB v2.0 - Ask Questions with TinyLlama\n")
    print("=" * 60)
    
    # Connect to AvocadoDB server
    try:
        db = AvocadoDB(url="http://localhost:8765")
        stats = db.stats()
        print("✅ Connected to AvocadoDB")
        print(f"   Database: {stats.get('artifacts_count', 0)} artifacts, {stats.get('spans_count', 0)} spans\n")
    except Exception as e:
        print(f"❌ Cannot connect to AvocadoDB server: {e}")
        print("   Start server with: ./target/release/avocado-server")
        return
    
    # Example queries
    queries = [
        "How does the compile function work?",
        "What is the WorkingSet structure?",
        "How does span extraction work?",
    ]
    
    for query in queries:
        print(f"Question: {query}")
        print("-" * 60)
        
        try:
            # Use ask() method - automatically uses TinyLlama if available
            answer = db.ask(query, llm="auto")
            print(f"Answer: {answer}\n")
        except Exception as e:
            print(f"❌ Error: {e}\n")
            # Fallback to just context
            try:
                context = db.compile(query)
                print(f"Context (fallback): {context.text[:200]}...\n")
            except:
                pass
    
    print("=" * 60)
    print("\n💡 Tips:")
    print("   - Use llm='auto' (default) to try LLM, fallback to context")
    print("   - Use llm='local' to require TinyLlama")
    print("   - Use llm='none' to just get context (same as compile())")


if __name__ == "__main__":
    main()

