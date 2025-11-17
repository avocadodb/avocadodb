#!/usr/bin/env python3
"""
Week 1 POC: 10-query test for AvocadoDB + TinyLlama

Tests diverse query types to validate the approach works across
different question types and code areas.

Usage:
    python tools/local_llm/poc_10_queries.py

Requirements:
    - AvocadoDB server running on http://localhost:8765
    - transformers, torch installed
    - AvocadoDB Python SDK installed
"""

import sys
import os
import json
import hashlib
from pathlib import Path
from typing import List, Dict, Any

# Add parent directory to path for imports
sys.path.insert(0, str(Path(__file__).parent.parent.parent))

try:
    from avocado import AvocadoDB
except ImportError:
    print("❌ AvocadoDB not found. Install with: pip install -e sdks/python")
    sys.exit(1)

try:
    from transformers import pipeline
    import torch
except ImportError:
    print("❌ transformers not found. Install with: pip install transformers torch")
    sys.exit(1)


# Test queries covering different aspects
TEST_QUERIES = [
    # Core functionality
    "How does the compile function work?",
    "What is the WorkingSet structure?",
    "How does span extraction work?",
    
    # Architecture
    "What is the overall architecture?",
    "How does the context compiler integrate components?",
    
    # Algorithms
    "What is the MMR algorithm used for?",
    "How does token budget packing work?",
    
    # Implementation details
    "How are embeddings generated?",
    "What database operations are used?",
    
    # API/Usage
    "How do I compile a query?"
]


class AvocadoPOCTester:
    def __init__(self, server_url="http://localhost:8765"):
        self.server_url = server_url
        self.db = None
        self.model = None
        self.results = []
    
    def setup(self):
        """Setup AvocadoDB client and model"""
        print("Setting up...")
        
        # Connect to AvocadoDB
        try:
            self.db = AvocadoDB(url=self.server_url)
            print("✅ Connected to AvocadoDB server")
        except Exception as e:
            print(f"❌ Could not connect to AvocadoDB server: {e}")
            print("   Start server with: ./target/release/avocado-server")
            return False
        
        # Load model
        try:
            print("Loading TinyLlama-1.1B model...")
            self.model = pipeline(
                "text-generation",
                model="TinyLlama/TinyLlama-1.1B-Chat-v1.0",
                device=-1  # CPU
            )
            print("✅ Model loaded")
        except Exception as e:
            print(f"❌ Failed to load model: {e}")
            return False
        
        return True
    
    def index_codebase(self):
        """Index key source files"""
        files_to_index = [
            "avocado-core/src/compiler.rs",
            "avocado-core/src/span.rs",
            "avocado-core/src/types.rs",
            "avocado-core/src/embedding.rs",
            "avocado-core/src/index.rs",
            "avocado-core/src/db.rs"
        ]
        
        print("\nIndexing source files...")
        indexed_count = 0
        
        for filepath in files_to_index:
            if not os.path.exists(filepath):
                print(f"  - {filepath}: Not found (skipping)")
                continue
            
            try:
                with open(filepath, 'r') as f:
                    content = f.read()
                
                # Check if already indexed
                stats_before = self.db.stats()
                try:
                    result = self.db.ingest(path=filepath, content=content)
                    span_count = result.get('spans_created', 0)
                    print(f"  ✅ {filepath}: {span_count} spans")
                    indexed_count += 1
                except Exception as ingest_error:
                    # Might already be indexed, check stats
                    stats_after = self.db.stats()
                    if stats_after.get('spans_count', 0) > stats_before.get('spans_count', 0):
                        print(f"  ✅ {filepath}: Already indexed or just indexed")
                        indexed_count += 1
                    else:
                        # Try with unique path
                        import uuid
                        unique_path = f"{filepath}.{uuid.uuid4().hex[:8]}"
                        result = self.db.ingest(path=unique_path, content=content)
                        span_count = result.get('spans_created', 0)
                        print(f"  ✅ {unique_path}: {span_count} spans")
                        indexed_count += 1
            except Exception as e:
                print(f"  ❌ {filepath}: {e}")
        
        print(f"\n✅ Indexed {indexed_count}/{len(files_to_index)} files")
        return indexed_count > 0
    
    def test_query(self, query: str) -> Dict[str, Any]:
        """Test a single query"""
        try:
            # Get context from AvocadoDB
            context_result = self.db.compile(
                query=query,
                budget=2048
            )
            
            # Format prompt
            context_text = context_result.text
            # Truncate for model context limit
            max_context = 1500
            if len(context_text) > max_context:
                context_text = context_text[:max_context] + "..."
            
            prompt = f"""Based on this code:

{context_text}

Question: {query}

Answer:"""
            
            # Generate answer (optimized: shorter for speed)
            output = self.model(
                prompt,
                max_new_tokens=100,  # Reduced from 150 for speed
                do_sample=False,  # Deterministic
                pad_token_id=self.model.tokenizer.eos_token_id,
                return_full_text=False
            )
            
            # Extract answer
            if isinstance(output, list) and len(output) > 0:
                if 'generated_text' in output[0]:
                    answer = output[0]['generated_text']
                    # Remove prompt if included
                    if prompt in answer:
                        answer = answer[len(prompt):].strip()
                    else:
                        answer = answer.strip()
                else:
                    answer = str(output[0])
            else:
                answer = str(output)
            
            return {
                "query": query,
                "answer": answer,
                "spans_used": len(context_result.spans),
                "citations": len(context_result.citations),
                "tokens_used": context_result.tokens_used,
                "answer_hash": hashlib.md5(answer.encode()).hexdigest()[:8]
            }
        except Exception as e:
            return {
                "query": query,
                "error": str(e),
                "error_type": type(e).__name__
            }
    
    def test_determinism(self, query: str, runs: int = 2) -> tuple[bool, List[str]]:
        """Test if same query produces same answer (optimized: 2 runs instead of 3)"""
        hashes = []
        for i in range(runs):
            result = self.test_query(query)
            if 'error' not in result:
                hashes.append(result['answer_hash'])
            # Early exit if we already have different hashes
            if len(set(hashes)) > 1:
                break
        
        is_deterministic = len(set(hashes)) == 1 if hashes else False
        return is_deterministic, hashes
    
    def evaluate_answer(self, result: Dict[str, Any]) -> Dict[str, Any]:
        """Simple evaluation of answer quality"""
        answer = result.get('answer', '').lower()
        
        # Basic quality checks
        checks = {
            "has_content": len(answer) > 20,
            "mentions_code": any(term in answer for term in 
                ['function', 'struct', 'method', 'compile', 'span', 'working', 
                 'algorithm', 'database', 'embedding', 'index']),
            "not_hallucinating": "i don't know" not in answer and "unclear" not in answer,
            "has_structure": '.' in answer or ',' in answer or '\n' in answer
        }
        
        score = sum(checks.values()) / len(checks) * 100
        return {"checks": checks, "score": score}
    
    def run_test_suite(self):
        """Run all tests"""
        print("\n" + "="*60)
        print("RUNNING 10-QUERY TEST SUITE")
        print("="*60)
        
        # Setup
        if not self.setup():
            return False
        
        # Index codebase
        if not self.index_codebase():
            print("⚠️  Warning: No files indexed, but continuing with existing data...")
        
        # Test each query
        print("\n" + "="*60)
        print("TESTING QUERIES")
        print("="*60)
        
        for i, query in enumerate(TEST_QUERIES, 1):
            print(f"\n[{i}/{len(TEST_QUERIES)}] {query}")
            print("-" * 60)
            
            # Test the query
            result = self.test_query(query)
            
            if 'error' in result:
                print(f"  ❌ Error: {result['error']}")
                self.results.append(result)
                continue
            
            # Evaluate answer
            evaluation = self.evaluate_answer(result)
            result['evaluation'] = evaluation
            
            # Test determinism (optimized: only 2 runs, and reuse first result)
            # We already have one result, just need one more for comparison
            result2 = self.test_query(query)
            if 'error' not in result2:
                is_deterministic = (result['answer_hash'] == result2['answer_hash'])
            else:
                is_deterministic = False
            result['deterministic'] = is_deterministic
            
            # Display results
            print(f"  Spans used: {result['spans_used']}")
            print(f"  Citations: {result['citations']}")
            print(f"  Tokens used: {result['tokens_used']}")
            print(f"  Quality score: {evaluation['score']:.0f}%")
            print(f"  Deterministic: {'✅' if is_deterministic else '❌'}")
            print(f"  Answer preview: {result['answer'][:150]}...")
            
            self.results.append(result)
        
        # Summary
        self.print_summary()
        return True
    
    def print_summary(self):
        """Print test summary"""
        print("\n" + "="*60)
        print("TEST SUMMARY")
        print("="*60)
        
        successful = [r for r in self.results if 'error' not in r]
        
        if not successful:
            print("\n❌ All queries failed - check server connection and logs")
            return
        
        # Calculate metrics
        avg_quality = sum(r['evaluation']['score'] for r in successful) / len(successful)
        deterministic_count = sum(1 for r in successful if r.get('deterministic', False))
        avg_spans = sum(r['spans_used'] for r in successful) / len(successful)
        avg_tokens = sum(r['tokens_used'] for r in successful) / len(successful)
        
        print(f"\n📊 Results:")
        print(f"  Total queries: {len(TEST_QUERIES)}")
        print(f"  Successful: {len(successful)}/{len(TEST_QUERIES)}")
        print(f"  Average quality: {avg_quality:.1f}%")
        print(f"  Deterministic: {deterministic_count}/{len(successful)}")
        
        print(f"\n⚡ Performance:")
        print(f"  Avg spans used: {avg_spans:.1f}")
        print(f"  Avg tokens used: {avg_tokens:.0f}")
        
        # Quality breakdown
        print(f"\n✅ Quality Metrics:")
        for check in ['has_content', 'mentions_code', 'not_hallucinating', 'has_structure']:
            passing = sum(1 for r in successful if r['evaluation']['checks'].get(check, False))
            percentage = (passing / len(successful)) * 100
            print(f"  {check}: {passing}/{len(successful)} ({percentage:.0f}%)")
        
        # Decision
        print(f"\n" + "="*60)
        print("DECISION")
        print("="*60)
        
        if avg_quality >= 70 and deterministic_count == len(successful):
            print("\n🎉 WEEK 1 SUCCESS - Proceed to Week 2")
            print("  ✅ Quality adequate (>70%)")
            print("  ✅ Fully deterministic (100%)")
            print("  ✅ Ready to scale to 100 queries")
            print("\nNext steps:")
            print("  1. Review answer quality (human evaluation)")
            print("  2. Proceed to Week 2: Full evaluation with 100 queries")
            print("  3. Consider trying Phi-2 (2.7B) for better quality")
        elif avg_quality >= 60 and deterministic_count == len(successful):
            print("\n⚠️  WEEK 1 ACCEPTABLE - Consider improvements")
            print(f"  ⚠️  Quality: {avg_quality:.1f}% (target: >70%)")
            print("  ✅ Fully deterministic (100%)")
            print("\nOptions:")
            print("  1. Proceed anyway (RAG-only is still useful)")
            print("  2. Try Phi-2 (2.7B) for better quality")
            print("  3. Improve RAG context retrieval")
        elif deterministic_count < len(successful):
            print("\n❌ WEEK 1 FAILED - Fix determinism first")
            print(f"  ❌ Determinism: {deterministic_count}/{len(successful)}")
            print("  ⚠️  This is CRITICAL - must be 100%")
            print("\nFix:")
            print("  - Check model settings (do_sample=False)")
            print("  - Verify temperature=0 or not set")
            print("  - Check for non-deterministic operations")
        else:
            print("\n⚠️  WEEK 1 NEEDS IMPROVEMENT")
            print(f"  ⚠️  Quality: {avg_quality:.1f}% (target: >70%)")
            print("  ✅ Deterministic: OK")
            print("\nOptions:")
            print("  1. Improve RAG context retrieval")
            print("  2. Try larger model (Phi-2, Phi-3-mini)")
            print("  3. Proceed anyway (RAG-only is still useful)")
        
        # Save results
        results_file = "tools/local_llm/poc_results.json"
        os.makedirs(os.path.dirname(results_file), exist_ok=True)
        with open(results_file, "w") as f:
            json.dump(self.results, f, indent=2)
        print(f"\n📄 Detailed results saved to {results_file}")


if __name__ == "__main__":
    print()
    tester = AvocadoPOCTester()
    success = tester.run_test_suite()
    print()
    sys.exit(0 if success else 1)

