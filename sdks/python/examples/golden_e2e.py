from pathlib import Path
from avocado import AvocadoDB

def main() -> None:
    # Use daemon (HTTP mode). Ensure server is running separately.
    db = AvocadoDB()  # defaults to http://localhost:8765 and project=PWD

    # 1) Ingest a small document (idempotent; duplicates return 0 spans)
    sample = Path("golden.md")
    sample.write_text("# AvocadoDB\nDeterministic RAG with citations.\nMMR and hybrid search.\n")
    ingest = db.ingest(str(sample), sample.read_text())
    print("Ingest:", ingest)

    # 2) Ask (LLM disabled by default; returns deterministic context text)
    context = db.ask("What is AvocadoDB?", llm="none", budget=4000)
    print("Context preview:", context.splitlines()[:6])

    # 3) Sessions: create → compile → add message → history → replay
    s = db.create_session(user_id="golden", title="Golden E2E")
    result = s.compile("Explain determinism", budget=4000)
    print("Tokens used:", result["working_set"]["tokens_used"])
    s.add_message("assistant", "Determinism ensures same query → same context.")
    print("History:", s.get_history())
    replay = s.replay()
    print("Replay turns:", len(replay["turns"]))

if __name__ == "__main__":
    main()

