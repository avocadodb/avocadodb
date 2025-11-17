"""Example: Using AvocadoDB in CLI mode (no server needed).

CLI mode is perfect for multi-repo usage - each directory gets its own database.
No server management needed!
"""

from avocado import AvocadoDB

# CLI mode - automatically uses per-directory database (.avocado/db.sqlite)
# Perfect for multi-repo: each repo gets its own isolated database
db = AvocadoDB(mode="cli")

# Or specify a custom database path
# db = AvocadoDB(mode="cli", db_path=".avocado/my-project.db")

# Ingest files (recursive directory support)
print("📚 Ingesting project files...")
result = db.ingest("./docs", recursive=True)
print(f"✅ Ingested: {result}")

# Compile context (same API as HTTP mode)
print("\n🔍 Compiling context...")
result = db.compile("How does authentication work?", budget=8000)
print(f"✅ Compiled {len(result.spans)} spans, {result.tokens_used} tokens")
print(f"📄 Context preview: {result.text[:200]}...")

# Ask questions (with TinyLlama if available)
print("\n💬 Asking question...")
answer = db.ask("What is this project about?")
print(f"✅ Answer: {answer}")

# Get stats
print("\n📊 Database stats:")
stats = db.stats()
print(f"  Artifacts: {stats['artifacts_count']}")
print(f"  Spans: {stats['spans_count']}")
print(f"  Tokens: {stats['total_tokens']}")

print("\n✨ CLI mode benefits:")
print("  - No server to manage")
print("  - Per-directory databases (perfect for multi-repo)")
print("  - Same API as HTTP mode")
print("  - Automatic binary detection")

