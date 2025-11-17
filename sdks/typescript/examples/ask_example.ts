#!/usr/bin/env ts-node
/**
 * Example: Using AvocadoDB's ask() method with TinyLlama
 *
 * This demonstrates the new v2.0 feature for getting natural language
 * answers from your codebase.
 *
 * Requirements:
 *   npm install avocadodb
 *   pip install avocadodb[llm]  # Python SDK with LLM support
 *
 * Setup:
 *   1. Start AvocadoDB server: ./target/release/avocado-server
 *   2. Ingest documents: ./target/release/avocado ingest test-docs/ --recursive
 *   3. Run: ts-node examples/ask_example.ts
 */

import { AvocadoDB } from '../src/client';

async function main() {
  console.log('🥑 AvocadoDB v2.0 - Ask Questions with TinyLlama\n');
  console.log('='.repeat(60));

  // Connect to AvocadoDB server
  const db = new AvocadoDB('http://localhost:8765');

  try {
    const stats = await db.stats();
    console.log('✅ Connected to AvocadoDB');
    console.log(`   Database: ${stats.artifacts} artifacts, ${stats.spans} spans\n`);
  } catch (error) {
    console.error('❌ Cannot connect to AvocadoDB server:', error);
    console.error('   Start server with: ./target/release/avocado-server');
    return;
  }

  // Example queries
  const queries = [
    'How does the compile function work?',
    'What is the WorkingSet structure?',
    'How does span extraction work?',
  ];

  for (const query of queries) {
    console.log(`Question: ${query}`);
    console.log('-'.repeat(60));

    try {
      // Use ask() method - automatically uses TinyLlama if available
      const answer = await db.ask(query, { llm: 'auto' });
      console.log(`Answer: ${answer}\n`);
    } catch (error) {
      console.error(`❌ Error: ${error}\n`);
      // Fallback to just context
      try {
        const context = await db.compile(query);
        console.log(`Context (fallback): ${context.text.substring(0, 200)}...\n`);
      } catch {
        // Ignore
      }
    }
  }

  console.log('='.repeat(60));
  console.log('\n💡 Tips:');
  console.log("   - Use llm: 'auto' (default) to try LLM, fallback to context");
  console.log("   - Use llm: 'local' to require TinyLlama");
  console.log("   - Use llm: 'none' to just get context (same as compile())");
}

if (require.main === module) {
  main().catch(console.error);
}

