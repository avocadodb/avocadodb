#!/usr/bin/env python3
"""
Advanced Example: Batch Session Processing

This example demonstrates batch processing techniques for sessions:

1. Processing multiple sessions in parallel
2. Batch analysis of historical sessions
3. Session cleanup and archiving
4. Generating reports across multiple sessions
5. Performance optimization for bulk operations

Use case: Analytics, batch reporting, session maintenance, data migration.
"""

from avocado import AvocadoDB
import json
import sys
from datetime import datetime
from concurrent.futures import ThreadPoolExecutor, as_completed
import time


def create_sample_sessions(db, count=10):
    """Create multiple sample sessions for batch processing."""

    print(f"Creating {count} sample sessions...")

    sessions = []
    users = ["alice", "bob", "charlie", "dave", "eve"]

    for i in range(count):
        user = users[i % len(users)]
        session = db.create_session(
            user_id=user,
            title=f"Session {i+1}: {user}'s conversation"
        )

        # Add some messages
        for j in range(3):
            session.compile(f"Question {j+1} from {user}", budget=4000)
            session.add_message("assistant", f"Response {j+1}")

        sessions.append(session)

    print(f"✓ Created {count} sessions\n")
    return sessions


def batch_analyze_sessions(db, user_id=None):
    """Analyze multiple sessions in batch."""

    print("=" * 80)
    print("Batch Session Analysis")
    print("=" * 80 + "\n")

    # List sessions
    sessions = db.list_sessions(user_id=user_id, limit=100)
    print(f"Found {len(sessions)} sessions")

    if user_id:
        print(f"Filtered by user: {user_id}")

    print()

    # Analyze each session
    analysis = []

    for sess_info in sessions:
        # Get full session data
        session = db.get_session(sess_info.id)

        # Get replay data
        replay = session.replay()

        # Calculate statistics
        total_turns = len(replay['turns'])
        total_tokens = sum(
            turn['working_set']['tokens_used']
            for turn in replay['turns']
            if turn.get('working_set')
        )

        avg_tokens = total_tokens / total_turns if total_turns > 0 else 0

        analysis.append({
            'session_id': sess_info.id,
            'user_id': sess_info.user_id,
            'title': sess_info.title,
            'created_at': sess_info.created_at,
            'turns': total_turns,
            'total_tokens': total_tokens,
            'avg_tokens': avg_tokens
        })

    # Print analysis table
    print(f"{'User':<15} {'Turns':<8} {'Total Tokens':<15} {'Avg Tokens':<12}")
    print("-" * 80)

    for a in analysis:
        user = a['user_id'] or 'N/A'
        print(f"{user:<15} {a['turns']:<8} {a['total_tokens']:<15} {a['avg_tokens']:<12.1f}")

    print()

    # Summary statistics
    total_sessions = len(analysis)
    total_all_tokens = sum(a['total_tokens'] for a in analysis)
    total_all_turns = sum(a['turns'] for a in analysis)

    print("Summary:")
    print(f"  Total Sessions: {total_sessions}")
    print(f"  Total Turns: {total_all_turns}")
    print(f"  Total Tokens: {total_all_tokens}")
    if total_all_turns > 0:
        print(f"  Avg Tokens/Turn: {total_all_tokens / total_all_turns:.1f}")
    print()

    return analysis


def parallel_session_processing(db, session_ids, process_func):
    """Process multiple sessions in parallel."""

    print("=" * 80)
    print("Parallel Session Processing")
    print("=" * 80 + "\n")

    print(f"Processing {len(session_ids)} sessions in parallel...")

    results = []
    start_time = time.time()

    # Use ThreadPoolExecutor for parallel processing
    with ThreadPoolExecutor(max_workers=5) as executor:
        # Submit all tasks
        future_to_session = {
            executor.submit(process_func, db, session_id): session_id
            for session_id in session_ids
        }

        # Collect results as they complete
        for future in as_completed(future_to_session):
            session_id = future_to_session[future]
            try:
                result = future.result()
                results.append(result)
                print(f"✓ Processed {session_id[:8]}...")
            except Exception as e:
                print(f"✗ Error processing {session_id[:8]}...: {e}")

    elapsed = time.time() - start_time
    print(f"\n✓ Processed {len(results)} sessions in {elapsed:.2f}s")
    print(f"  Avg time per session: {elapsed / len(results):.3f}s\n")

    return results


def extract_session_summary(db, session_id):
    """Extract summary information from a session (for parallel processing)."""

    session = db.get_session(session_id)
    replay = session.replay()

    return {
        'session_id': session_id,
        'user_id': replay['session'].get('user_id'),
        'turns': len(replay['turns']),
        'first_query': replay['turns'][0]['user_message']['content'] if replay['turns'] else None
    }


def batch_export_sessions(db, output_dir="session_exports"):
    """Export multiple sessions to JSON files."""

    print("=" * 80)
    print("Batch Session Export")
    print("=" * 80 + "\n")

    import os

    # Create output directory
    os.makedirs(output_dir, exist_ok=True)

    # List all sessions
    sessions = db.list_sessions(limit=100)
    print(f"Exporting {len(sessions)} sessions to {output_dir}/")

    exported = 0

    for sess_info in sessions:
        try:
            session = db.get_session(sess_info.id)
            replay = session.replay()

            # Create export data
            export_data = {
                'exported_at': datetime.now().isoformat(),
                'session': replay['session'],
                'turns': [
                    {
                        'user_query': turn['user_message']['content'],
                        'tokens_used': turn['working_set']['tokens_used'] if turn.get('working_set') else 0,
                        'assistant_response': turn['assistant_message']['content'] if turn.get('assistant_message') else None
                    }
                    for turn in replay['turns']
                ]
            }

            # Write to file
            filename = f"{output_dir}/session_{sess_info.id}.json"
            with open(filename, 'w') as f:
                json.dump(export_data, f, indent=2)

            exported += 1
            print(f"✓ Exported {sess_info.id}")

        except Exception as e:
            print(f"✗ Error exporting {sess_info.id}: {e}")

    print(f"\n✓ Exported {exported}/{len(sessions)} sessions\n")


def batch_cleanup_old_sessions(db, dry_run=True):
    """Clean up old or inactive sessions."""

    print("=" * 80)
    print("Batch Session Cleanup")
    print("=" * 80 + "\n")

    if dry_run:
        print("DRY RUN MODE - No sessions will be deleted\n")

    # List all sessions
    sessions = db.list_sessions(limit=1000)
    print(f"Total sessions: {len(sessions)}\n")

    # Identify sessions to clean up
    # Example criteria: sessions with no messages, or very old sessions
    to_delete = []

    for sess_info in sessions:
        session = db.get_session(sess_info.id)
        replay = session.replay()

        # Criteria: sessions with no turns
        if len(replay['turns']) == 0:
            to_delete.append({
                'session_id': sess_info.id,
                'reason': 'No turns'
            })

    print(f"Sessions to delete: {len(to_delete)}")

    if to_delete:
        print("\nSessions marked for deletion:")
        for item in to_delete[:10]:  # Show first 10
            print(f"  - {item['session_id']}: {item['reason']}")

        if len(to_delete) > 10:
            print(f"  ... and {len(to_delete) - 10} more")

    if not dry_run and to_delete:
        print("\nDeleting sessions...")
        for item in to_delete:
            try:
                session = db.get_session(item['session_id'])
                session.delete()
                print(f"✓ Deleted {item['session_id']}")
            except Exception as e:
                print(f"✗ Error deleting {item['session_id']}: {e}")

    print()


def generate_user_report(db, user_id):
    """Generate a comprehensive report for a specific user."""

    print("=" * 80)
    print(f"User Report: {user_id}")
    print("=" * 80 + "\n")

    # Get user's sessions
    sessions = db.list_sessions(user_id=user_id)

    if not sessions:
        print(f"No sessions found for user: {user_id}\n")
        return

    print(f"Total Sessions: {len(sessions)}\n")

    # Analyze each session
    total_turns = 0
    total_tokens = 0
    all_queries = []

    for sess_info in sessions:
        session = db.get_session(sess_info.id)
        replay = session.replay()

        session_turns = len(replay['turns'])
        total_turns += session_turns

        for turn in replay['turns']:
            if turn.get('working_set'):
                total_tokens += turn['working_set']['tokens_used']
            all_queries.append(turn['user_message']['content'])

    # Generate report
    print("Activity Summary:")
    print(f"  Sessions: {len(sessions)}")
    print(f"  Total Turns: {total_turns}")
    print(f"  Total Tokens: {total_tokens}")
    print(f"  Avg Turns/Session: {total_turns / len(sessions):.1f}")
    print(f"  Avg Tokens/Turn: {total_tokens / total_turns:.1f}" if total_turns > 0 else "  Avg Tokens/Turn: 0")
    print()

    print("Recent Queries:")
    for i, query in enumerate(all_queries[-5:], 1):
        print(f"  {i}. {query[:60]}...")
    print()


def demo_batch_operations():
    """Demonstrate all batch operations."""

    db = AvocadoDB(mode="http")

    # Create sample data
    sessions = create_sample_sessions(db, count=15)
    session_ids = [s.id for s in sessions]

    # Run batch analyses
    batch_analyze_sessions(db)

    input("\nPress Enter to continue to parallel processing...")

    # Parallel processing
    parallel_session_processing(
        db,
        session_ids[:10],
        extract_session_summary
    )

    input("\nPress Enter to continue to batch export...")

    # Batch export
    batch_export_sessions(db)

    input("\nPress Enter to continue to user report...")

    # User report
    generate_user_report(db, "alice")

    input("\nPress Enter to continue to cleanup (dry run)...")

    # Cleanup (dry run)
    batch_cleanup_old_sessions(db, dry_run=True)

    print("\n✓ All batch operations completed!")


def main():
    """Run batch processing demos."""

    print("\n🥑 AvocadoDB Batch Session Processing Examples\n")

    try:
        db = AvocadoDB(mode="http")

        demos = [
            ("Create Sample Sessions", lambda: create_sample_sessions(db, 10)),
            ("Batch Analysis", lambda: batch_analyze_sessions(db)),
            ("Batch Export", lambda: batch_export_sessions(db)),
            ("User Report", lambda: generate_user_report(db, "alice")),
            ("Cleanup (Dry Run)", lambda: batch_cleanup_old_sessions(db, dry_run=True)),
            ("Full Demo", demo_batch_operations),
        ]

        print("Available batch operations:")
        for i, (name, _) in enumerate(demos, 1):
            print(f"  {i}. {name}")
        print()

        choice = input(f"Select operation (1-{len(demos)}): ")
        choice = int(choice)

        if 1 <= choice <= len(demos):
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
