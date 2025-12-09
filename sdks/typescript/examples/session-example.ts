/**
 * Session Management Example
 *
 * Demonstrates how to use AvocadoDB sessions in TypeScript:
 * - Creating sessions
 * - Multi-turn conversations
 * - Context compilation
 * - Session replay
 *
 * Run with:
 *   npx ts-node examples/session-example.ts
 */

import { AvocadoDB, SessionManager } from '../src';

async function basicSessionExample() {
  console.log('='.repeat(80));
  console.log('Basic Session Example');
  console.log('='.repeat(80));
  console.log();

  // Initialize client
  const db = new AvocadoDB('http://localhost:8765', '.');

  // Create session manager
  const sessionManager = new SessionManager('http://localhost:8765', '.');

  // Create a new session
  console.log('Creating session...');
  const session = await sessionManager.createSession({
    userId: 'alice',
    title: 'TypeScript Session Demo',
  });

  console.log(`✓ Created session: ${session.id}`);
  console.log(`  User: ${session.userId}`);
  console.log(`  Title: ${session.title}`);
  console.log();

  // First query - compile context
  console.log('User: What is AvocadoDB?');
  const result1 = await session.compile('What is AvocadoDB?', {
    budget: 8000,
  });

  console.log(`✓ Context compiled: ${result1.workingSet.tokensUsed} tokens`);
  console.log();

  // Add assistant response
  console.log('Adding assistant response...');
  await session.addMessage(
    'assistant',
    'AvocadoDB is a deterministic context database...'
  );
  console.log('✓ Response added');
  console.log();

  // Continue conversation
  console.log('User: How does the compiler work?');
  const result2 = await session.compile('How does the compiler work?');
  console.log(`✓ Context compiled: ${result2.workingSet.tokensUsed} tokens`);
  console.log();

  await session.addMessage(
    'assistant',
    'The compiler uses a hybrid search approach...'
  );
  console.log('✓ Response added');
  console.log();

  // Get conversation history
  console.log('Getting conversation history...');
  const history = await session.getHistory();
  console.log('Conversation History:');
  console.log('-'.repeat(80));
  console.log(history);
  console.log('-'.repeat(80));
  console.log();

  return session;
}

async function sessionReplayExample(session: any) {
  console.log('='.repeat(80));
  console.log('Session Replay Example');
  console.log('='.repeat(80));
  console.log();

  // Replay the session
  console.log('Replaying session...');
  const replay = await session.replay();

  console.log(`Session: ${replay.session.id}`);
  console.log(`User: ${replay.session.userId}`);
  console.log(`Turns: ${replay.turns.length}`);
  console.log();

  // Analyze each turn
  replay.turns.forEach((turn, index) => {
    console.log(`Turn ${index + 1}:`);
    console.log(`  User: ${turn.userMessage.content}`);

    if (turn.workingSet) {
      console.log(`  Context: ${turn.workingSet.tokensUsed} tokens`);
      console.log(`  Spans: ${turn.workingSet.spans.length}`);

      // Show top citations
      if (turn.workingSet.spans.length > 0) {
        console.log('  Top citations:');
        turn.workingSet.spans.slice(0, 3).forEach((span, i) => {
          console.log(
            `    ${i + 1}. ${span.artifactPath}:${span.startLine}-${span.endLine} (score: ${span.score.toFixed(3)})`
          );
        });
      }
    }

    if (turn.assistantMessage) {
      const preview = turn.assistantMessage.content.substring(0, 60);
      console.log(`  Assistant: ${preview}...`);
    }

    console.log();
  });
}

async function tokenLimitedHistoryExample() {
  console.log('='.repeat(80));
  console.log('Token-Limited History Example');
  console.log('='.repeat(80));
  console.log();

  const sessionManager = new SessionManager('http://localhost:8765', '.');

  // Create session
  const session = await sessionManager.createSession({
    userId: 'bob',
    title: 'Long Conversation',
  });

  console.log('Creating long conversation...');

  // Add many messages
  for (let i = 0; i < 20; i++) {
    await session.compile(`Question ${i + 1}`);
    await session.addMessage('assistant', `Response ${i + 1}`);
  }

  console.log('✓ Added 20 turns');
  console.log();

  // Get full history
  const fullHistory = await session.getHistory();
  console.log(`Full history: ${fullHistory.length} characters`);
  console.log();

  // Get token-limited history
  const limitedHistory = await session.getHistory(1000);
  console.log(`Limited history (1000 tokens): ${limitedHistory.length} characters`);
  console.log();

  console.log('Limited History Preview:');
  console.log('-'.repeat(80));
  console.log(limitedHistory.substring(0, 500) + '...');
  console.log('-'.repeat(80));
  console.log();

  // Cleanup
  await session.delete();
  console.log('✓ Session deleted');
  console.log();
}

async function listSessionsExample() {
  console.log('='.repeat(80));
  console.log('List Sessions Example');
  console.log('='.repeat(80));
  console.log();

  const sessionManager = new SessionManager('http://localhost:8765', '.');

  // Create multiple sessions
  console.log('Creating test sessions...');
  await sessionManager.createSession({ userId: 'alice', title: 'Session 1' });
  await sessionManager.createSession({ userId: 'alice', title: 'Session 2' });
  await sessionManager.createSession({ userId: 'bob', title: 'Session 3' });
  console.log('✓ Created 3 sessions');
  console.log();

  // List all sessions
  console.log('All sessions:');
  const allSessions = await sessionManager.listSessions();
  allSessions.forEach((s) => {
    console.log(`  - ${s.userId}: ${s.title} (${s.id})`);
  });
  console.log();

  // List sessions for specific user
  console.log("Alice's sessions:");
  const aliceSessions = await sessionManager.listSessions('alice');
  aliceSessions.forEach((s) => {
    console.log(`  - ${s.title} (${s.id})`);
  });
  console.log();
}

async function main() {
  console.log();
  console.log('🥑 AvocadoDB TypeScript Session Examples');
  console.log();

  try {
    // Run basic example
    const session = await basicSessionExample();

    console.log('Press Enter to continue to session replay...');
    await new Promise((resolve) => {
      process.stdin.once('data', resolve);
    });

    // Replay the session
    await sessionReplayExample(session);

    console.log('Press Enter to continue to token-limited history...');
    await new Promise((resolve) => {
      process.stdin.once('data', resolve);
    });

    // Token-limited history
    await tokenLimitedHistoryExample();

    console.log('Press Enter to continue to list sessions...');
    await new Promise((resolve) => {
      process.stdin.once('data', resolve);
    });

    // List sessions
    await listSessionsExample();

    console.log('✓ All examples completed!');
  } catch (error) {
    console.error('❌ Error:', error);
    console.error();
    console.error('Make sure AvocadoDB server is running:');
    console.error('  cargo run --bin avocado-server');
    process.exit(1);
  }
}

// Run if executed directly
if (require.main === module) {
  main();
}
