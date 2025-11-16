#!/usr/bin/env ts-node

/**
 * AvocadoDB Example - Demonstrating Deterministic Context Compilation
 *
 * This example shows how AvocadoDB provides deterministic context compilation,
 * unlike traditional RAG systems.
 *
 * Run this after starting the server and ingesting documents:
 *   ./target/release/avocado-server &
 *   ./target/release/avocado ingest test-docs/ --recursive
 *   npm run example
 */

import { AvocadoDB } from './src';

async function main() {
  console.log('🥑 AvocadoDB Determinism Demo\n');
  console.log('='.repeat(60));

  // Initialize client (connects to running server)
  const db = new AvocadoDB('http://localhost:8080');

  // Check if server is reachable
  const isOnline = await db.ping();
  if (!isOnline) {
    console.log('\n⚠️  Server not reachable');
    console.log('\nMake sure AvocadoDB server is running:');
    console.log('  ./target/release/avocado-server');
    return;
  }

  // Check database stats
  try {
    const stats = await db.stats();
    console.log('\nDatabase Stats:');
    console.log(`  Artifacts: ${stats.artifacts}`);
    console.log(`  Spans:     ${stats.spans}`);
    console.log(`  Tokens:    ${stats.tokens}`);

    if (stats.spans === 0) {
      console.log('\n⚠️  No documents in database!');
      console.log('\nIngest some documents first:');
      console.log('  ./target/release/avocado ingest test-docs/ --recursive');
      return;
    }
  } catch (error) {
    console.error('\n⚠️  Failed to get stats:', error);
    return;
  }

  // Run the same query multiple times
  const query = 'How does authentication work?';
  console.log('\n' + '='.repeat(60));
  console.log(`Query: '${query}'`);
  console.log('Running compilation 3 times...\n');

  const results: string[] = [];
  const hashes: string[] = [];

  for (let i = 0; i < 3; i++) {
    console.log(`Run ${i + 1}:`);

    const result = await db.compile(query, { budget: 8000 });

    // Store result and hash
    results.push(result.text);
    const hash = result.deterministicHash();
    hashes.push(hash);

    console.log(`  Spans:  ${result.spans.length}`);
    console.log(`  Tokens: ${result.tokensUsed}`);
    console.log(`  Time:   ${result.compilationTimeMs}ms`);
    console.log(`  Hash:   ${hash.substring(0, 16)}...`);
    console.log();
  }

  // Verify determinism
  console.log('='.repeat(60));
  console.log('\n✨ Determinism Check:\n');

  if (results[0] === results[1] && results[1] === results[2]) {
    console.log('✅ PASS: All results are identical!');
    console.log('   Same text content across all runs');
  } else {
    console.log('❌ FAIL: Results differ!');
  }

  if (hashes[0] === hashes[1] && hashes[1] === hashes[2]) {
    console.log('✅ PASS: All hashes match!');
    console.log(`   Hash: ${hashes[0]}`);
  } else {
    console.log('❌ FAIL: Hashes differ!');
    console.log(`   Hash 1: ${hashes[0]}`);
    console.log(`   Hash 2: ${hashes[1]}`);
    console.log(`   Hash 3: ${hashes[2]}`);
  }

  // Show context preview
  console.log('\n' + '='.repeat(60));
  console.log('\n📄 Context Preview (first 500 chars):\n');
  console.log(results[0].substring(0, 500));
  console.log('...');

  // Get working set for citations
  const finalResult = await db.compile(query, { budget: 8000 });

  // Show citations
  console.log('\n' + '='.repeat(60));
  console.log('\nCitations:');
  finalResult.citations.forEach((citation, i) => {
    console.log(
      `  [${i + 1}] ${citation.artifactPath} ` +
        `(lines ${citation.startLine}-${citation.endLine})`
    );
  });

  console.log('\n' + '='.repeat(60));
  console.log('\n🎉 Demo Complete!');
  console.log('\nKey Takeaway:');
  console.log('  Same query → Same context, every time.');
  console.log('  This is the guarantee AvocadoDB provides.');
  console.log();
}

main().catch((error) => {
  console.error('Error:', error);
  process.exit(1);
});
