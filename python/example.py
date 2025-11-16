"""
AvocadoDB Example - Demonstrating Determinism

This example shows how AvocadoDB provides deterministic context compilation,
unlike traditional RAG systems.
"""

import hashlib
from avocado import AvocadoDB


def test_determinism():
    """Test that AvocadoDB produces deterministic results."""

    # Initialize client
    db = AvocadoDB()

    # Clear any existing data
    db.clear()

    # Ingest some sample documents
    docs = [
        {
            "path": "auth.md",
            "content": """# Authentication

            Our system uses JWT tokens for authentication.
            Users must provide valid credentials to receive a token.

            ## Login Endpoint

            POST /api/login
            - username: string
            - password: string

            Returns a JWT token valid for 24 hours.
            """,
        },
        {
            "path": "api.md",
            "content": """# API Reference

            All API endpoints require authentication via Bearer token.

            ## Endpoints

            - GET /api/users - List all users
            - POST /api/users - Create new user
            - PUT /api/users/:id - Update user
            """,
        },
        {
            "path": "security.md",
            "content": """# Security Best Practices

            1. Always use HTTPS in production
            2. Rotate JWT secrets regularly
            3. Implement rate limiting
            4. Use strong password policies
            """,
        },
    ]

    print("Ingesting documents...")
    for doc in docs:
        result = db.ingest(doc["path"], doc["content"])
        print(f"  ✓ {doc['path']}: {result['spans_created']} spans")

    # The critical test: Run the same query multiple times
    query = "How does authentication work?"

    print(f"\nCompiling context for: '{query}'")
    print("Running query 3 times to test determinism...\n")

    results = []
    hashes = []

    for i in range(3):
        result = db.compile(query, budget=8000)
        text_hash = hashlib.sha256(result.text.encode()).hexdigest()[:16]

        results.append(result)
        hashes.append(text_hash)

        print(f"Run {i + 1}:")
        print(f"  Hash: {text_hash}")
        print(f"  Tokens: {result.tokens_used}")
        print(f"  Spans: {len(result.spans)}")
        print(f"  Time: {result.compilation_time_ms}ms")
        print()

    # Verify determinism
    if len(set(hashes)) == 1:
        print("✅ SUCCESS: All runs produced identical results!")
        print("   This is the key differentiator vs traditional RAG.")
    else:
        print("❌ FAILURE: Results differ across runs")
        print("   Hashes:", hashes)
        return

    # Show the compiled context
    print("\n" + "=" * 60)
    print("COMPILED CONTEXT")
    print("=" * 60)
    print(results[0].text)
    print("\n" + "=" * 60)

    # Show citations
    print("\nCitations:")
    for i, citation in enumerate(results[0].citations, 1):
        print(f"  [{i}] {citation['artifact_path']} "
              f"(lines {citation['start_line']}-{citation['end_line']})")

    # Show stats
    stats = db.stats()
    print(f"\nDatabase Stats:")
    print(f"  Artifacts: {stats['artifacts_count']}")
    print(f"  Spans: {stats['spans_count']}")
    print(f"  Total tokens: {stats['total_tokens']}")


if __name__ == "__main__":
    test_determinism()
