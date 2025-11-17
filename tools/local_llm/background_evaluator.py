#!/usr/bin/env python3
"""
Background Evaluation Service for AvocadoDB

Runs evaluation in the background while you work on your codebase.
Integrates with Avocado CLI to automatically re-evaluate when codebase changes.

Usage:
    # Start background evaluator
    avocado eval start --watch
    
    # Check status
    avocado eval status
    
    # Stop
    avocado eval stop

Features:
    - Watches codebase for changes
    - Re-runs evaluation automatically
    - Uses OpenAI API (cheap, fast, good quality)
    - Background process, doesn't block CLI
    - Results saved to .avocado/eval_results.json
"""

import os
import sys
import json
import time
import signal
import subprocess
from pathlib import Path
from typing import Dict, Any, Optional, List
from datetime import datetime
import hashlib
import requests

# Add parent directory to path
sys.path.insert(0, str(Path(__file__).parent.parent.parent))

try:
    from avocado import AvocadoDB
except ImportError:
    print("❌ AvocadoDB SDK not found. Install with: pip install -e sdks/python")
    sys.exit(1)


class BackgroundEvaluator:
    """Background evaluation service"""
    
    def __init__(
        self,
        avocadodb_url: str = "http://localhost:8765",
        openai_api_key: Optional[str] = None,
        watch_paths: Optional[List[str]] = None,
        eval_interval: int = 300,  # 5 minutes
    ):
        self.avocadodb_url = avocadodb_url
        self.openai_api_key = openai_api_key or os.environ.get("OPENAI_API_KEY")
        self.watch_paths = watch_paths or ["avocado-core/src", "avocado-server/src", "avocado-cli/src"]
        self.eval_interval = eval_interval
        self.running = False
        self.pid_file = Path(".avocado/eval_service.pid")
        self.results_file = Path(".avocado/eval_results.json")
        self.db = None
        
    def setup(self):
        """Setup evaluator"""
        # Create .avocado directory
        Path(".avocado").mkdir(exist_ok=True)
        
        # Connect to AvocadoDB
        try:
            self.db = AvocadoDB(url=self.avocadodb_url)
            stats = self.db.stats()
            print(f"✅ Connected to AvocadoDB: {stats.get('spans_count', 0)} spans")
        except Exception as e:
            print(f"❌ Cannot connect to AvocadoDB: {e}")
            return False
        
        # Check OpenAI API key
        if not self.openai_api_key:
            print("❌ OPENAI_API_KEY not set")
            return False
        
        return True
    
    def evaluate_with_api(self, queries: List[str]) -> Dict[str, Any]:
        """Evaluate queries using OpenAI API (cheap, fast)"""
        results = []
        
        for i, query in enumerate(queries, 1):
            print(f"[{i}/{len(queries)}] {query[:60]}...")
            
            try:
                # Get context from AvocadoDB
                context_result = self.db.compile(query=query, budget=2048)
                
                # Call OpenAI API
                response = requests.post(
                    "https://api.openai.com/v1/chat/completions",
                    headers={
                        "Authorization": f"Bearer {self.openai_api_key}",
                        "Content-Type": "application/json",
                    },
                    json={
                        "model": "gpt-3.5-turbo",  # Cheap, fast, good quality
                        "messages": [
                            {
                                "role": "system",
                                "content": "You are a codebase Q&A assistant. Answer questions based on the provided code context. Be concise and accurate."
                            },
                            {
                                "role": "user",
                                "content": f"Based on this code:\n\n{context_result.text[:2000]}\n\nQuestion: {query}\n\nAnswer:"
                            }
                        ],
                        "temperature": 0,  # Deterministic
                        "max_tokens": 200,
                    },
                    timeout=30
                )
                
                if response.status_code == 200:
                    answer = response.json()["choices"][0]["message"]["content"]
                    
                    # Evaluate quality
                    quality_score = self.evaluate_answer_quality(query, answer, context_result)
                    
                    results.append({
                        "query": query,
                        "answer": answer,
                        "spans_used": len(context_result.spans),
                        "citations": len(context_result.citations),
                        "tokens_used": context_result.tokens_used,
                        "quality_score": quality_score,
                        "timestamp": datetime.now().isoformat(),
                    })
                    
                    print(f"  ✅ Quality: {quality_score:.1f}% | Citations: {len(context_result.citations)}")
                else:
                    print(f"  ❌ API error: {response.status_code}")
                    results.append({
                        "query": query,
                        "error": f"API error: {response.status_code}",
                        "timestamp": datetime.now().isoformat(),
                    })
                    
            except Exception as e:
                print(f"  ❌ Error: {e}")
                results.append({
                    "query": query,
                    "error": str(e),
                    "timestamp": datetime.now().isoformat(),
                })
        
        return {
            "results": results,
            "timestamp": datetime.now().isoformat(),
            "total_queries": len(queries),
            "successful": len([r for r in results if "error" not in r]),
        }
    
    def evaluate_answer_quality(self, query: str, answer: str, context: Any) -> float:
        """Simple quality evaluation"""
        answer_lower = answer.lower()
        checks = {
            "has_content": len(answer) > 20,
            "mentions_code": any(word in answer_lower for word in ['function', 'struct', 'module', 'code', 'implementation']),
            "has_citations": context.citations and len(context.citations) > 0,
            "not_hallucinating": not any(word in answer_lower for word in ["i don't know", "cannot determine", "not provided"]),
        }
        return sum(checks.values()) / len(checks) * 100
    
    def get_sample_queries(self, count: int = 10) -> List[str]:
        """Get sample queries for evaluation"""
        # Use a subset of queries for background evaluation
        return [
            "How does the compile function work?",
            "What is the WorkingSet structure?",
            "How does span extraction work?",
            "What is the overall architecture?",
            "How does the context compiler integrate components?",
            "What is the MMR algorithm used for?",
            "How does token budget packing work?",
            "How are embeddings generated?",
            "What database operations are used?",
            "How do I compile a query?",
        ][:count]
    
    def run_evaluation(self):
        """Run a single evaluation"""
        print(f"\n{'='*70}")
        print(f"BACKGROUND EVALUATION - {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
        print(f"{'='*70}\n")
        
        queries = self.get_sample_queries(count=10)  # Quick evaluation
        results = self.evaluate_with_api(queries)
        
        # Save results
        self.results_file.parent.mkdir(exist_ok=True)
        with open(self.results_file, 'w') as f:
            json.dump(results, f, indent=2)
        
        # Print summary
        print(f"\n{'='*70}")
        print("EVALUATION SUMMARY")
        print(f"{'='*70}")
        print(f"Total queries: {results['total_queries']}")
        print(f"Successful: {results['successful']}/{results['total_queries']}")
        if results['results']:
            avg_quality = sum(r.get('quality_score', 0) for r in results['results'] if 'quality_score' in r) / len([r for r in results['results'] if 'quality_score' in r])
            print(f"Average quality: {avg_quality:.1f}%")
        print(f"Results saved to: {self.results_file}")
        print()
    
    def watch_and_evaluate(self):
        """Watch codebase and re-evaluate on changes"""
        print("Starting background evaluator...")
        print(f"Watching: {', '.join(self.watch_paths)}")
        print(f"Evaluation interval: {self.eval_interval}s")
        print(f"Press Ctrl+C to stop\n")
        
        self.running = True
        
        # Initial evaluation
        self.run_evaluation()
        
        # Watch for changes
        last_hash = self.get_codebase_hash()
        
        while self.running:
            time.sleep(self.eval_interval)
            
            if not self.running:
                break
            
            # Check if codebase changed
            current_hash = self.get_codebase_hash()
            if current_hash != last_hash:
                print(f"\n🔄 Codebase changed, re-running evaluation...")
                self.run_evaluation()
                last_hash = current_hash
            else:
                print(f"⏳ No changes detected (checking every {self.eval_interval}s)")
    
    def get_codebase_hash(self) -> str:
        """Get hash of watched codebase files"""
        hasher = hashlib.md5()
        
        for watch_path in self.watch_paths:
            path = Path(watch_path)
            if path.exists():
                if path.is_file():
                    try:
                        hasher.update(path.read_bytes())
                    except:
                        pass
                elif path.is_dir():
                    for file_path in path.rglob("*"):
                        if file_path.is_file() and file_path.suffix in ['.rs', '.py', '.ts', '.js']:
                            try:
                                hasher.update(file_path.read_bytes())
                            except:
                                pass
        
        return hasher.hexdigest()
    
    def start_daemon(self):
        """Start as daemon process"""
        if self.pid_file.exists():
            print(f"⚠️ Evaluator already running (PID: {self.pid_file.read_text().strip()})")
            return False
        
        # Fork to background
        pid = os.fork()
        if pid == 0:
            # Child process
            os.setsid()
            os.chdir("/")
            os.umask(0)
            
            # Redirect stdio
            sys.stdin = open(os.devnull, 'r')
            sys.stdout = open(os.devnull, 'w')
            sys.stderr = open(os.devnull, 'w')
            
            # Run evaluator
            self.watch_and_evaluate()
        else:
            # Parent process
            self.pid_file.write_text(str(pid))
            print(f"✅ Background evaluator started (PID: {pid})")
            print(f"   Check status: avocado eval status")
            print(f"   Stop: avocado eval stop")
            return True
    
    def stop_daemon(self):
        """Stop daemon process"""
        if not self.pid_file.exists():
            print("❌ Evaluator not running")
            return False
        
        pid = int(self.pid_file.read_text().strip())
        try:
            os.kill(pid, signal.SIGTERM)
            self.pid_file.unlink()
            print(f"✅ Evaluator stopped (PID: {pid})")
            return True
        except ProcessLookupError:
            print("❌ Process not found (may have already stopped)")
            self.pid_file.unlink()
            return False
    
    def status(self):
        """Check status"""
        if not self.pid_file.exists():
            print("❌ Evaluator not running")
            return
        
        pid = int(self.pid_file.read_text().strip())
        try:
            os.kill(pid, 0)  # Check if process exists
            print(f"✅ Evaluator running (PID: {pid})")
            
            if self.results_file.exists():
                with open(self.results_file) as f:
                    results = json.load(f)
                print(f"   Last evaluation: {results.get('timestamp', 'Unknown')}")
                print(f"   Results: {results.get('successful', 0)}/{results.get('total_queries', 0)} successful")
        except ProcessLookupError:
            print("❌ Process not running (stale PID file)")
            self.pid_file.unlink()


def main():
    """CLI interface"""
    import argparse
    
    parser = argparse.ArgumentParser(description="Background evaluation service")
    parser.add_argument("command", choices=["start", "stop", "status", "run"], help="Command to run")
    parser.add_argument("--watch", action="store_true", help="Watch for changes")
    parser.add_argument("--interval", type=int, default=300, help="Evaluation interval (seconds)")
    parser.add_argument("--url", default="http://localhost:8765", help="AvocadoDB server URL")
    
    args = parser.parse_args()
    
    evaluator = BackgroundEvaluator(
        avocadodb_url=args.url,
        eval_interval=args.interval,
    )
    
    if not evaluator.setup():
        sys.exit(1)
    
    if args.command == "start":
        if args.watch:
            evaluator.watch_and_evaluate()
        else:
            evaluator.start_daemon()
    elif args.command == "stop":
        evaluator.stop_daemon()
    elif args.command == "status":
        evaluator.status()
    elif args.command == "run":
        evaluator.run_evaluation()


if __name__ == "__main__":
    main()

