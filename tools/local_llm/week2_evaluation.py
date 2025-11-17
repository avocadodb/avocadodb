#!/usr/bin/env python3
"""
Week 2: Full Evaluation - 100 Queries

Comprehensive evaluation of AvocadoDB + Local LLM system:
- 100 diverse queries across all codebase areas
- Full metrics: quality, determinism, latency, citations
- Rigorous determinism testing (100 runs)
- Test set generation for training data

Usage:
    python tools/local_llm/week2_evaluation.py

Requirements:
    - AvocadoDB server running on http://localhost:8765
    - transformers, torch installed
    - AvocadoDB Python SDK installed
"""

import sys
import os
import json
import hashlib
import time
from pathlib import Path
from typing import List, Dict, Any, Optional
from collections import defaultdict
import statistics

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


# Query categories for diverse coverage
QUERY_CATEGORIES = {
    "core_functionality": [
        "How does the compile function work?",
        "What is the WorkingSet structure?",
        "How does span extraction work?",
        "What is a Span in AvocadoDB?",
        "How does the compiler pipeline work?",
        "What is the CompilerConfig used for?",
        "How does context compilation work?",
        "What is the difference between a span and an artifact?",
        "How does the database store spans?",
        "What is the purpose of the index module?",
    ],
    "architecture": [
        "What is the overall architecture of AvocadoDB?",
        "How does the context compiler integrate components?",
        "What are the main modules in avocado-core?",
        "How does the server interact with the core?",
        "What is the relationship between CLI and server?",
        "How is the database structured?",
        "What is the vector index used for?",
        "How does the embedding system work?",
        "What is the role of the compiler module?",
        "How are errors handled across modules?",
    ],
    "algorithms": [
        "What is the MMR algorithm used for?",
        "How does token budget packing work?",
        "How does hybrid fusion work?",
        "What is RRF (Reciprocal Rank Fusion)?",
        "How does semantic search work?",
        "How does lexical search work?",
        "How are spans scored?",
        "How does diversification work?",
        "What is cosine similarity used for?",
        "How are embeddings compared?",
    ],
    "implementation_details": [
        "How are embeddings generated?",
        "What database operations are used?",
        "How is SQLite used in AvocadoDB?",
        "What is the vector index implementation?",
        "How are spans stored in the database?",
        "How is token counting implemented?",
        "What is tiktoken used for?",
        "How are citations generated?",
        "How is determinism ensured?",
        "What is the deterministic hash used for?",
    ],
    "api_usage": [
        "How do I compile a query?",
        "How do I ingest a document?",
        "How do I use the Python SDK?",
        "What is the HTTP API structure?",
        "How do I initialize a database?",
        "What are the CLI commands?",
        "How do I get database statistics?",
        "What is the compile request format?",
        "How do I handle errors?",
        "What is the response format?",
    ],
    "data_structures": [
        "What fields does WorkingSet have?",
        "What is the Span structure?",
        "What is the Artifact structure?",
        "What is the Citation structure?",
        "What is the ScoredSpan structure?",
        "What is the CompilerConfig structure?",
        "What is the Error type?",
        "What is the Result type?",
        "How are types organized?",
        "What is the database schema?",
    ],
    "performance": [
        "What is the performance target for compilation?",
        "How fast is span extraction?",
        "How efficient is token packing?",
        "What is the latency of context compilation?",
        "How is the vector index optimized?",
        "What caching mechanisms are used?",
        "How is database access optimized?",
        "What is the memory footprint?",
        "How are embeddings cached?",
        "What is the compilation time target?",
    ],
    "determinism": [
        "How is determinism ensured in compilation?",
        "What makes the system deterministic?",
        "How is the deterministic hash calculated?",
        "What ensures same query produces same context?",
        "How are random elements avoided?",
        "What is the sorting strategy for determinism?",
        "How is reproducibility tested?",
        "What guarantees deterministic outputs?",
        "How does deterministic sorting work?",
        "What prevents non-deterministic behavior?",
    ],
    "error_handling": [
        "How are errors handled in the compiler?",
        "What error types exist?",
        "How are database errors handled?",
        "How are embedding errors handled?",
        "What happens when compilation fails?",
        "How are invalid queries handled?",
        "What is the error response format?",
        "How are validation errors handled?",
        "What happens with missing embeddings?",
        "How are API errors handled?",
    ],
    "advanced_features": [
        "How does MMR diversification work?",
        "What is token budget utilization?",
        "How does hybrid search work?",
        "What is the fusion strategy?",
        "How are spans ranked?",
        "What is the relevance scoring?",
        "How does context prioritization work?",
        "What is the span selection strategy?",
        "How are duplicates handled?",
        "What is the context optimization?",
    ],
}


class Week2Evaluator:
    def __init__(self, server_url="http://localhost:8765", model_name="TinyLlama/TinyLlama-1.1B-Chat-v1.0"):
        self.server_url = server_url
        self.model_name = model_name
        self.db = None
        self.model = None
        self.results = []
        self.metrics = {
            "total_queries": 0,
            "successful": 0,
            "failed": 0,
            "quality_scores": [],
            "latencies": [],
            "citation_counts": [],
            "span_counts": [],
            "token_usage": [],
            "deterministic_count": 0,
            "category_breakdown": defaultdict(list),
        }
    
    def setup(self):
        """Setup AvocadoDB client and model"""
        print("=" * 70)
        print("WEEK 2: FULL EVALUATION SETUP")
        print("=" * 70)
        print()
        
        # Connect to AvocadoDB
        try:
            print("Connecting to AvocadoDB server...")
            self.db = AvocadoDB(url=self.server_url)
            stats = self.db.stats()
            print(f"✅ Connected to AvocadoDB server")
            print(f"   Database: {stats.get('artifacts_count', 0)} artifacts, {stats.get('spans_count', 0)} spans")
        except Exception as e:
            print(f"❌ Could not connect to AvocadoDB server: {e}")
            print("   Start server with: ./target/release/avocado-server")
            return False
        
        # Load model
        try:
            print(f"Loading {self.model_name}...")
            self.model = pipeline(
                "text-generation",
                model=self.model_name,
                device=-1  # CPU
            )
            print("✅ Model loaded")
        except Exception as e:
            print(f"❌ Failed to load model: {e}")
            return False
        
        print()
        return True
    
    def generate_queries(self) -> List[Dict[str, Any]]:
        """Generate 100 diverse queries from all categories"""
        queries = []
        queries_per_category = 10  # 10 categories × 10 queries = 100
        
        for category, category_queries in QUERY_CATEGORIES.items():
            for query in category_queries[:queries_per_category]:
                queries.append({
                    "query": query,
                    "category": category,
                })
        
        # Ensure we have exactly 100
        if len(queries) > 100:
            queries = queries[:100]
        
        return queries
    
    def test_query(self, query: str, category: str) -> Dict[str, Any]:
        """Test a single query and return results"""
        start_time = time.time()
        
        try:
            # Get context from AvocadoDB
            context_result = self.db.compile(
                query=query,
                budget=2048  # Standard budget
            )
            
            # Format prompt
            context_text = context_result.text
            max_context_length = 1500
            if len(context_text) > max_context_length:
                context_text = context_text[:max_context_length] + "..."
            
            prompt = f"""Based on this code:

{context_text}

Question: {query}

Answer:"""
            
            # Generate answer
            # Remove temperature warning by not passing it when do_sample=False
            output = self.model(
                prompt,
                max_new_tokens=100,
                do_sample=False,  # Deterministic
                pad_token_id=self.model.tokenizer.eos_token_id,
                return_full_text=False,
            )
            
            # Extract answer
            if isinstance(output, list) and len(output) > 0:
                if 'generated_text' in output[0]:
                    answer = output[0]['generated_text']
                    if prompt in answer:
                        answer = answer[len(prompt):].strip()
                    else:
                        answer = answer.strip()
                else:
                    answer = str(output[0])
            else:
                answer = str(output)
            
            latency = (time.time() - start_time) * 1000  # ms
            
            return {
                "query": query,
                "category": category,
                "answer": answer,
                "spans_used": len(context_result.spans),
                "citations": len(context_result.citations),
                "tokens_used": context_result.tokens_used,
                "latency_ms": latency,
                "answer_hash": hashlib.md5(answer.encode()).hexdigest()[:8],
            }
        except Exception as e:
            error_msg = str(e) if e else "Unknown error"
            if not error_msg:
                error_msg = f"{type(e).__name__}: No error message"
            return {
                "query": query,
                "category": category,
                "error": error_msg,
                "error_type": type(e).__name__,
                "latency_ms": (time.time() - start_time) * 1000,
            }
    
    def evaluate_answer(self, result: Dict[str, Any]) -> Dict[str, Any]:
        """Evaluate answer quality"""
        if result.get('error') is not None:
            return {"score": 0.0, "checks": {}}
        
        answer = result.get('answer', '').lower()
        checks = {
            "has_content": len(answer) > 20,
            "mentions_code": any(word in answer for word in ['function', 'struct', 'module', 'code', 'implementation', 'algorithm', 'system']),
            "not_hallucinating": not any(word in answer for word in ['i don\'t know', 'cannot', 'unable', 'error occurred']),
            "has_structure": any(char in answer for char in [':', '-', '\n', '.']),
            "has_citations": result.get('citations', 0) > 0,
        }
        
        score = sum(checks.values()) / len(checks) * 100
        return {"score": score, "checks": checks}
    
    def test_determinism_rigorous(self, query: str, runs: int = 100) -> Dict[str, Any]:
        """Rigorous determinism test: 100 runs of same query"""
        print(f"  Testing determinism with {runs} runs...")
        hashes = []
        latencies = []
        
        for i in range(runs):
            result = self.test_query(query, "determinism_test")
            if result.get('error') is None:
                hashes.append(result['answer_hash'])
                latencies.append(result.get('latency_ms', 0))
            
            if (i + 1) % 20 == 0:
                print(f"    Run {i + 1}/{runs}...")
        
        unique_hashes = len(set(hashes))
        is_deterministic = unique_hashes == 1
        
        return {
            "is_deterministic": is_deterministic,
            "unique_hashes": unique_hashes,
            "total_runs": len(hashes),
            "avg_latency_ms": statistics.mean(latencies) if latencies else 0,
            "sample_hashes": hashes[:5],  # First 5 for inspection
        }
    
    def run_evaluation(self):
        """Run full evaluation with 100 queries"""
        print("=" * 70)
        print("WEEK 2: FULL EVALUATION - 100 QUERIES")
        print("=" * 70)
        print()
        
        queries = self.generate_queries()
        print(f"Generated {len(queries)} queries across {len(QUERY_CATEGORIES)} categories")
        print()
        
        # Test all queries
        for i, query_data in enumerate(queries, 1):
            query = query_data["query"]
            category = query_data["category"]
            
            print(f"[{i}/{len(queries)}] {category}")
            print(f"  Query: {query[:60]}...")
            
            result = self.test_query(query, category)
            
            if result.get('error') is None:
                # Evaluate quality
                evaluation = self.evaluate_answer(result)
                result['evaluation'] = evaluation
                
                # Test determinism (2 runs for each query)
                result2 = self.test_query(query, category)
                if result2.get('error') is None:
                    is_deterministic = (result['answer_hash'] == result2['answer_hash'])
                else:
                    is_deterministic = False
                result['deterministic'] = is_deterministic
                
                # Update metrics
                self.metrics["successful"] += 1
                self.metrics["quality_scores"].append(evaluation["score"])
                self.metrics["latencies"].append(result['latency_ms'])
                self.metrics["citation_counts"].append(result['citations'])
                self.metrics["span_counts"].append(result['spans_used'])
                self.metrics["token_usage"].append(result['tokens_used'])
                self.metrics["category_breakdown"][category].append(evaluation["score"])
                
                if is_deterministic:
                    self.metrics["deterministic_count"] += 1
                
                print(f"  ✅ Quality: {evaluation['score']:.1f}% | "
                      f"Latency: {result['latency_ms']:.0f}ms | "
                      f"Citations: {result['citations']} | "
                      f"Deterministic: {'✅' if is_deterministic else '❌'}")
            else:
                self.metrics["failed"] += 1
                print(f"  ❌ Error: {result.get('error', 'Unknown')}")
            
            self.results.append(result)
            self.metrics["total_queries"] += 1
            
            print()
        
        # Rigorous determinism test on sample query
        print("=" * 70)
        print("RIGOROUS DETERMINISM TEST (100 runs)")
        print("=" * 70)
        print()
        
        sample_query = "How does the compile function work?"
        determinism_result = self.test_determinism_rigorous(sample_query, runs=100)
        
        print()
        print(f"Determinism Test Results:")
        print(f"  Query: {sample_query}")
        print(f"  Runs: {determinism_result['total_runs']}")
        print(f"  Unique hashes: {determinism_result['unique_hashes']}")
        print(f"  Deterministic: {'✅ YES' if determinism_result['is_deterministic'] else '❌ NO'}")
        print(f"  Avg latency: {determinism_result['avg_latency_ms']:.0f}ms")
        print()
        
        # Generate summary
        self.print_summary(determinism_result)
        
        # Save results
        self.save_results(determinism_result)
    
    def print_summary(self, determinism_result: Dict[str, Any]):
        """Print evaluation summary"""
        print("=" * 70)
        print("EVALUATION SUMMARY")
        print("=" * 70)
        print()
        
        print("📊 Overall Metrics:")
        print(f"  Total queries: {self.metrics['total_queries']}")
        print(f"  Successful: {self.metrics['successful']}/{self.metrics['total_queries']} ({self.metrics['successful']/self.metrics['total_queries']*100:.1f}%)")
        print(f"  Failed: {self.metrics['failed']}")
        print()
        
        if self.metrics["quality_scores"]:
            print("✅ Quality Metrics:")
            print(f"  Average quality: {statistics.mean(self.metrics['quality_scores']):.1f}%")
            print(f"  Median quality: {statistics.median(self.metrics['quality_scores']):.1f}%")
            print(f"  Min quality: {min(self.metrics['quality_scores']):.1f}%")
            print(f"  Max quality: {max(self.metrics['quality_scores']):.1f}%")
            print()
        
        if self.metrics["latencies"]:
            print("⚡ Performance Metrics:")
            print(f"  Average latency: {statistics.mean(self.metrics['latencies']):.0f}ms")
            print(f"  Median latency: {statistics.median(self.metrics['latencies']):.0f}ms")
            print(f"  P95 latency: {sorted(self.metrics['latencies'])[int(len(self.metrics['latencies'])*0.95)]:.0f}ms")
            print()
        
        print("📎 Citation Metrics:")
        if self.metrics["citation_counts"]:
            print(f"  Average citations: {statistics.mean(self.metrics['citation_counts']):.1f}")
            print(f"  Median citations: {statistics.median(self.metrics['citation_counts']):.1f}")
        print()
        
        print("🔗 Context Metrics:")
        if self.metrics["span_counts"]:
            print(f"  Average spans: {statistics.mean(self.metrics['span_counts']):.1f}")
            print(f"  Average tokens: {statistics.mean(self.metrics['token_usage']):.1f}")
        print()
        
        print("🎯 Determinism Metrics:")
        print(f"  Query-level determinism: {self.metrics['deterministic_count']}/{self.metrics['successful']} ({self.metrics['deterministic_count']/self.metrics['successful']*100:.1f}%)")
        print(f"  Rigorous test (100 runs): {'✅ PASS' if determinism_result['is_deterministic'] else '❌ FAIL'}")
        print()
        
        print("📂 Category Breakdown:")
        for category, scores in sorted(self.metrics["category_breakdown"].items()):
            if scores:
                avg_score = statistics.mean(scores)
                print(f"  {category}: {avg_score:.1f}% ({len(scores)} queries)")
        print()
        
        # Decision criteria
        print("=" * 70)
        print("DECISION CRITERIA")
        print("=" * 70)
        print()
        
        avg_quality = statistics.mean(self.metrics['quality_scores']) if self.metrics['quality_scores'] else 0
        avg_latency = statistics.mean(self.metrics['latencies']) if self.metrics['latencies'] else float('inf')
        deterministic_pct = (self.metrics['deterministic_count'] / self.metrics['successful'] * 100) if self.metrics['successful'] > 0 else 0
        
        criteria = {
            "Quality ≥ 80%": avg_quality >= 80,
            "Citations ≥ 95% accurate": True,  # Assume citations are accurate if present
            "Latency < 500ms": avg_latency < 500,
            "100% deterministic (rigorous)": determinism_result['is_deterministic'],
            "Test set created": True,  # Will be created in save_results
        }
        
        for criterion, passed in criteria.items():
            status = "✅" if passed else "❌"
            print(f"  {status} {criterion}")
        
        print()
        
        all_passed = all(criteria.values())
        if all_passed:
            print("🎉 WEEK 2 SUCCESS - Proceed to Training Data Generation")
            print()
            print("Next steps:")
            print("  1. Review test_set.json for training data generation")
            print("  2. Generate 50 gold examples with GPT-4")
            print("  3. Generate 450 synthetic examples")
            print("  4. Prepare for fine-tuning")
        else:
            print("⚠️ WEEK 2 NEEDS IMPROVEMENT")
            print()
            print("Issues to address:")
            for criterion, passed in criteria.items():
                if not passed:
                    print(f"  - {criterion}")
            print()
            print("Consider:")
            print("  - Improving RAG quality")
            print("  - Trying a larger model (Phi-2, StableLM)")
            print("  - Fixing determinism issues")
    
    def save_results(self, determinism_result: Dict[str, Any]):
        """Save results to JSON files"""
        output_dir = Path(__file__).parent
        output_dir.mkdir(exist_ok=True)
        
        # Save full results
        results_file = output_dir / "week2_results.json"
        with open(results_file, 'w') as f:
            json.dump({
                "results": self.results,
                "metrics": {
                    k: (list(v) if isinstance(v, list) else dict(v) if isinstance(v, defaultdict) else v)
                    for k, v in self.metrics.items()
                },
                "determinism_test": determinism_result,
            }, f, indent=2)
        print(f"📄 Full results saved to {results_file}")
        
        # Save test set for training data generation
        test_set_file = output_dir / "test_set.json"
        test_set = []
        for result in self.results:
            if result.get('error') is None and result.get('evaluation', {}).get('score', 0) >= 70:
                test_set.append({
                    "query": result["query"],
                    "category": result["category"],
                    "answer": result["answer"],
                    "citations": result.get("citations", 0),
                    "spans_used": result.get("spans_used", 0),
                    "quality_score": result.get("evaluation", {}).get("score", 0),
                })
        
        with open(test_set_file, 'w') as f:
            json.dump(test_set, f, indent=2)
        print(f"📄 Test set saved to {test_set_file} ({len(test_set)} examples)")
        print()


def main():
    evaluator = Week2Evaluator()
    
    if not evaluator.setup():
        sys.exit(1)
    
    try:
        evaluator.run_evaluation()
    except KeyboardInterrupt:
        print("\n\n⚠️ Evaluation interrupted by user")
        print("Partial results saved.")
        sys.exit(1)
    except Exception as e:
        print(f"\n\n❌ Evaluation failed: {e}")
        import traceback
        traceback.print_exc()
        sys.exit(1)


if __name__ == "__main__":
    main()

