#!/usr/bin/env python3
"""
AvocadoDB Example - Demonstrating Deterministic Context Compilation

This example shows how AvocadoDB provides deterministic context compilation,
unlike traditional RAG systems.

Run this after starting the server and ingesting documents:
  ./target/release/avocado-server &
  ./target/release/avocado ingest test-docs/ --recursive
  python3 python/example.py
"""

from avocado import AvocadoDB


def main():
    """Demonstrate AvocadoDB determinism."""
    print("🥑 AvocadoDB Determinism Demo\n")
    print("=" * 60)

    # Initialize client (connects to running server)
    db = AvocadoDB("http://localhost:8765")

    # Check database stats
    try:
        stats = db.stats()
        print("\nDatabase Stats:")
        print(f"  Artifacts: {stats.get('artifacts', 0)}")
        print(f"  Spans:     {stats.get('spans', 0)}")
        print(f"  Tokens:    {stats.get('tokens', 0)}")
    except Exception as e:
        print(f"\n⚠️  Server not reachable: {e}")
        print("\nMake sure AvocadoDB server is running:")
        print("  ./target/release/avocado-server")
        return

    if stats.get('spans', 0) == 0:
        print("\n⚠️  No documents in database!")
        print("\nIngest some documents first:")
        print("  ./target/release/avocado ingest test-docs/ --recursive")
        return

    # Run the same query multiple times
    query = "How does authentication work?"
    print(f"\n{'=' * 60}")
    print(f"Query: '{query}'")
    print("Running compilation 3 times...\n")

    results = []
    hashes = []

    for i in range(3):
        print(f"Run {i + 1}:")
        result = db.compile(query, budget=8000)

        # Store result and hash
        results.append(result.text)
        hash_value = result.deterministic_hash()
        hashes.append(hash_value)

        print(f"  Spans:  {len(result.spans)}")
        print(f"  Tokens: {result.tokens_used}")
        print(f"  Time:   {result.compilation_time_ms}ms")
        print(f"  Hash:   {hash_value[:16]}...")
        print()

    # Verify determinism
    print(f"{'=' * 60}")
    print("\n✨ Determinism Check:")
    print()

    if results[0] == results[1] == results[2]:
        print("✅ PASS: All results are identical!")
        print("   Same text content across all runs")
    else:
        print("❌ FAIL: Results differ!")

    if hashes[0] == hashes[1] == hashes[2]:
        print("✅ PASS: All hashes match!")
        print(f"   Hash: {hashes[0]}")
    else:
        print("❌ FAIL: Hashes differ!")
        print(f"   Hash 1: {hashes[0]}")
        print(f"   Hash 2: {hashes[1]}")
        print(f"   Hash 3: {hashes[2]}")

    # Show context preview
    print(f"\n{'=' * 60}")
    print("\n📄 Context Preview (first 500 chars):\n")
    print(results[0][:500])
    print("...")

    # Show citations
    print(f"\n{'=' * 60}")
    print("\nCitations:")
    for i, citation in enumerate(results[0].citations, 1):
        print(f"  [{i}] {citation.artifact_path} "
              f"(lines {citation.start_line}-{citation.end_line})")

    print(f"\n{'=' * 60}")
    print("\n🎉 Demo Complete!")
    print("\nKey Takeaway:")
    print("  Same query → Same context, every time.")
    print("  This is the guarantee AvocadoDB provides.")
    print()


if __name__ == "__main__":
    main()
