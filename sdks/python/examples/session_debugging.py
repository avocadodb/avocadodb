#!/usr/bin/env python3
"""
Advanced Example: Session Debugging and Analysis

This example demonstrates advanced debugging techniques for sessions:

1. Session replay for analyzing agent behavior
2. Context quality analysis
3. Token usage tracking
4. Citation analysis
5. Exporting session data for offline analysis

Use case: Debugging agent issues, analyzing performance, optimizing context.
"""

from avocado import AvocadoDB
import json
import sys
from datetime import datetime
from uuid import uuid4


def create_debug_session():
    """Create a session with sample data for debugging."""

    db = AvocadoDB(mode="http")

    print("Creating debug session with sample data...")

    # Use a non-sensitive, ephemeral user identifier for debugging
    session = db.create_session(user_id=f"debug-{uuid4().hex[:8]}", title="Debugging Session")

    # Simulate a conversation with various queries
    queries = [
        "What is the architecture of AvocadoDB?",
        "How does vector search work?",
        "Explain the compilation algorithm",
        "What are the performance characteristics?",
        "How do I use the Python SDK?"
    ]

    for query in queries:
        session.compile(query, budget=8000)
        session.add_message("assistant", f"Response to: {query}")

    print(f"✓ Created session {session.id} with {len(queries)} turns\n")

    return db, session


def debug_session_replay(session):
    """Analyze session using replay functionality."""

    print("=" * 80)
    print("Session Replay Analysis")
    print("=" * 80 + "\n")

    replay = session.replay()

    # Session metadata
    sess = replay['session']
    print(f"Session ID: {sess['id']}")
    print(f"User: {sess.get('user_id', 'N/A')}")
    print(f"Title: {sess.get('title', 'N/A')}")
    print(f"Created: {sess['created_at']}")
    print(f"Total Turns: {len(replay['turns'])}\n")

    # Analyze each turn
    total_tokens = 0
    total_spans = 0

    for i, turn in enumerate(replay['turns'], 1):
        print(f"Turn {i}")
        print("-" * 80)

        # User message
        user_msg = turn['user_message']
        print(f"User Query: {user_msg['content']}")
        print(f"Timestamp: {user_msg['created_at']}")

        # Working set analysis
        if turn.get('working_set'):
            ws = turn['working_set']
            tokens = ws['tokens_used']
            spans = ws['spans']

            total_tokens += tokens
            total_spans += len(spans)

            print("\nContext Compilation:")
            print(f"  Tokens: {tokens}")
            print(f"  Spans: {len(spans)}")

            # Analyze top citations
            if spans:
                print("  Top Citations:")
                for j, span in enumerate(spans[:3], 1):
                    score = span.get('score', 0)
                    location = span.get('location', 'unknown')
                    print(f"    {j}. {location} (score: {score:.3f})")

        # Assistant response
        if turn.get('assistant_message'):
            asst_msg = turn['assistant_message']
            response_preview = asst_msg['content'][:100]
            print(f"\nAssistant Response: {response_preview}...")

        print("\n")

    # Summary statistics
    print("=" * 80)
    print("Summary Statistics")
    print("=" * 80)
    print(f"Total Tokens Used: {total_tokens}")
    print(f"Total Spans Retrieved: {total_spans}")
    print(f"Avg Tokens/Turn: {total_tokens / len(replay['turns']):.1f}")
    print(f"Avg Spans/Turn: {total_spans / len(replay['turns']):.1f}")
    print()


def analyze_context_quality(session):
    """Analyze the quality of retrieved context."""

    print("=" * 80)
    print("Context Quality Analysis")
    print("=" * 80 + "\n")

    replay = session.replay()

    for i, turn in enumerate(replay['turns'], 1):
        if not turn.get('working_set'):
            continue

        ws = turn['working_set']
        spans = ws['spans']

        print(f"Turn {i}: {turn['user_message']['content'][:60]}...")

        if not spans:
            print("  ⚠️  No spans retrieved!")
            continue

        # Calculate score distribution
        scores = [span.get('score', 0) for span in spans]
        avg_score = sum(scores) / len(scores)
        max_score = max(scores)
        min_score = min(scores)

        print(f"  Spans: {len(spans)}")
        print(f"  Score Range: {min_score:.3f} - {max_score:.3f}")
        print(f"  Avg Score: {avg_score:.3f}")

        # Check for low-quality results
        if avg_score < 0.5:
            print("  ⚠️  Warning: Low average score!")

        # Check for span diversity
        sources = set(span.get('location', 'unknown').split(':')[0] for span in spans)
        print(f"  Unique Sources: {len(sources)}")

        if len(sources) < 3:
            print("  ℹ️  Limited source diversity")

        print()


def analyze_token_usage(session):
    """Track token usage patterns."""

    print("=" * 80)
    print("Token Usage Analysis")
    print("=" * 80 + "\n")

    replay = session.replay()

    token_data = []

    for i, turn in enumerate(replay['turns'], 1):
        if turn.get('working_set'):
            ws = turn['working_set']
            tokens = ws['tokens_used']
            token_data.append({
                'turn': i,
                'query': turn['user_message']['content'][:40],
                'tokens': tokens
            })

    # Print token usage table
    print(f"{'Turn':<6} {'Query':<45} {'Tokens':<10}")
    print("-" * 80)

    for data in token_data:
        print(f"{data['turn']:<6} {data['query']:<45} {data['tokens']:<10}")

    print()

    # Statistics
    if token_data:
        tokens = [d['tokens'] for d in token_data]
        print(f"Total Tokens: {sum(tokens)}")
        print(f"Average: {sum(tokens) / len(tokens):.1f}")
        print(f"Min: {min(tokens)}")
        print(f"Max: {max(tokens)}")
    print()


def export_session_data(session, filename="session_export.json"):
    """Export session data for offline analysis."""

    print("=" * 80)
    print("Export Session Data")
    print("=" * 80 + "\n")

    replay = session.replay()

    # Create exportable data structure
    export_data = {
        'exported_at': datetime.now().isoformat(),
        'session': replay['session'],
        'turns': []
    }

    for turn in replay['turns']:
        turn_data = {
            'user_message': {
                'content': turn['user_message']['content'],
                'timestamp': turn['user_message']['created_at']
            }
        }

        if turn.get('working_set'):
            ws = turn['working_set']
            turn_data['context'] = {
                'tokens_used': ws['tokens_used'],
                'num_spans': len(ws['spans']),
                'spans': [
                    {
                        'location': span.get('location', 'unknown'),
                        'score': span.get('score', 0),
                        'text_preview': span.get('text', '')[:100]
                    }
                    for span in ws['spans'][:10]  # Top 10 spans
                ]
            }

        if turn.get('assistant_message'):
            turn_data['assistant_message'] = {
                'content': turn['assistant_message']['content'],
                'timestamp': turn['assistant_message']['created_at']
            }

        export_data['turns'].append(turn_data)

    # Write to file
    with open(filename, 'w') as f:
        json.dump(export_data, f, indent=2)

    print(f"✓ Exported session data to {filename}")
    print(f"  Session: {export_data['session']['id']}")
    print(f"  Turns: {len(export_data['turns'])}")
    print(f"  File size: {len(json.dumps(export_data))} bytes")
    print()


def compare_token_budgets(db):
    """Compare different token budgets for the same query."""

    print("=" * 80)
    print("Token Budget Comparison")
    print("=" * 80 + "\n")

    budgets = [2000, 4000, 8000, 16000]
    query = "Explain the compilation algorithm in detail"

    results = []

    for budget in budgets:
        # Use a randomized, ephemeral user identifier for comparisons
        session = db.create_session(user_id=f"budget-{uuid4().hex[:8]}", title=f"Budget Test: {budget}")

        result = session.compile(query, budget=budget)
        ws = result['working_set']

        results.append({
            'budget': budget,
            'tokens_used': ws['tokens_used'],
            'num_spans': len(ws['spans'])
        })

        # Clean up
        session.delete()

    # Print comparison table
    print(f"Query: {query}\n")
    print(f"{'Budget':<10} {'Tokens Used':<15} {'Spans':<10} {'Utilization':<15}")
    print("-" * 80)

    for r in results:
        utilization = (r['tokens_used'] / r['budget']) * 100
        print(f"{r['budget']:<10} {r['tokens_used']:<15} {r['num_spans']:<10} {utilization:.1f}%")

    print()


def debug_empty_results(session):
    """Debug queries that return no or poor results."""

    print("=" * 80)
    print("Debug Empty/Poor Results")
    print("=" * 80 + "\n")

    # Test queries
    test_queries = [
        "What is AvocadoDB?",  # Should have results
        "xyzabc123nonsense",   # Likely no results
        "the the the the",     # Common words
    ]

    for query in test_queries:
        print(f"Testing query: '{query}'")

        result = session.compile(query, budget=4000)
        ws = result['working_set']

        print(f"  Tokens: {ws['tokens_used']}")
        print(f"  Spans: {len(ws['spans'])}")

        if ws['spans']:
            avg_score = sum(s.get('score', 0) for s in ws['spans']) / len(ws['spans'])
            print(f"  Avg Score: {avg_score:.3f}")

            if avg_score < 0.3:
                print("  ⚠️  Low relevance scores - consider rephrasing query")
        else:
            print("  ⚠️  No results found!")
            print("  💡 Suggestions:")
            print("     - Check if data is ingested")
            print("     - Try broader search terms")
            print("     - Check for typos")

        print()


def main():
    """Run debugging demos."""

    print("\n🥑 AvocadoDB Session Debugging Examples\n")

    try:
        db, session = create_debug_session()

        demos = [
            ("Session Replay Analysis", lambda: debug_session_replay(session)),
            ("Context Quality Analysis", lambda: analyze_context_quality(session)),
            ("Token Usage Analysis", lambda: analyze_token_usage(session)),
            ("Export Session Data", lambda: export_session_data(session)),
            ("Compare Token Budgets", lambda: compare_token_budgets(db)),
            ("Debug Empty Results", lambda: debug_empty_results(session)),
        ]

        print("Available debugging tools:")
        for i, (name, _) in enumerate(demos, 1):
            print(f"  {i}. {name}")
        print(f"  {len(demos) + 1}. Run all")
        print()

        choice = input(f"Select tool (1-{len(demos) + 1}): ")
        choice = int(choice)

        if choice == len(demos) + 1:
            for name, demo_func in demos:
                print("\n\n")
                demo_func()
        elif 1 <= choice <= len(demos):
            demos[choice - 1][1]()
        else:
            print("Invalid choice")
            sys.exit(1)

    except KeyboardInterrupt:
        print("\n\nInterrupted")
        sys.exit(0)
    except Exception as e:
        print(f"\n❌ Error: {e}")
        print("\nMake sure AvocadoDB server is running:")
        print("  cargo run --bin avocado-server")
        sys.exit(1)


if __name__ == "__main__":
    main()
