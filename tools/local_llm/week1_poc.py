#!/usr/bin/env python3
"""
Week 1: Ultra-minimal proof of concept - Can run TODAY
Start with absolute minimum: 1 file, 1 query, 1 model

This POC tests if AvocadoDB + local LLM can answer codebase questions
with deterministic, citation-backed responses.

Usage:
    python tools/local_llm/week1_poc.py

Requirements:
    pip install transformers torch avocado
    # Or: pip install -e sdks/python
"""

import sys
import os
from pathlib import Path

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


def test_minimal_poc():
    """Test with absolute minimum to prove concept works."""
    
    print("🥑 AvocadoDB + Local LLM POC")
    print("=" * 60)
    print()
    
    # 1. Index just ONE file first (not everything)
    print("Step 1: Indexing single file...")
    db_path = ".avocado/db.sqlite"
    
    # Check if AvocadoDB server is running
    try:
        # Default port is 8765, but client defaults to 8080
        db = AvocadoDB(url="http://localhost:8765")
        print("✅ Connected to AvocadoDB server")
    except Exception as e:
        print(f"❌ Could not connect to AvocadoDB server: {e}")
        print("   Start server with: ./target/release/avocado-server")
        print("   Server should be running on http://localhost:8765")
        return None
    
    # Index just compiler.rs
    compiler_file = "avocado-core/src/compiler.rs"
    if not os.path.exists(compiler_file):
        print(f"❌ File not found: {compiler_file}")
        return None
    
    print(f"   Indexing: {compiler_file}")
    try:
        with open(compiler_file, 'r') as f:
            content = f.read()
        
        # Check if file already indexed
        stats_before = db.stats()
        try:
            result = db.ingest(
                path=compiler_file,
                content=content
            )
            # ingest returns dict with 'spans_created' (from server)
            span_count = result.get('spans_created', 0)
            print(f"✅ Indexed: {span_count} spans created")
        except Exception as ingest_error:
            # File might already be indexed, check stats
            stats_after = db.stats()
            if stats_after.get('spans_count', 0) > stats_before.get('spans_count', 0):
                print(f"✅ File already indexed (or indexed just now)")
            else:
                # Try with a unique path to avoid conflicts
                import uuid
                unique_path = f"{compiler_file}.{uuid.uuid4().hex[:8]}"
                result = db.ingest(path=unique_path, content=content)
                span_count = result.get('spans_created', 0)
                print(f"✅ Indexed with unique path: {span_count} spans created")
                compiler_file = unique_path  # Use unique path for query
    except Exception as e:
        print(f"❌ Failed to index file: {e}")
        print(f"   Error type: {type(e).__name__}")
        # Continue anyway - file might already be indexed
        print("   Continuing with existing database content...")
    
    print()
    
    # 2. Load tiny model
    print("Step 2: Loading TinyLlama model...")
    try:
        # Use simpler pipeline initialization
        model = pipeline(
            "text-generation",
            model="TinyLlama/TinyLlama-1.1B-Chat-v1.0",
            device=0 if torch.cuda.is_available() else -1,  # -1 for CPU
        )
        print("✅ Model loaded")
    except Exception as e:
        print(f"❌ Failed to load model: {e}")
        print("   This may take a few minutes on first run (downloading model)")
        print(f"   Error details: {type(e).__name__}")
        return None
    
    print()
    
    # 3. Test ONE query
    print("Step 3: Testing query...")
    query = "How does the compile function work?"
    print(f"   Query: {query}")
    
    try:
        # Get context from AvocadoDB
        context_result = db.compile(
            query=query,
            budget=1024
        )
        print(f"✅ Context retrieved: {len(context_result.spans)} spans, {context_result.tokens_used} tokens")
        print(f"   Citations: {len(context_result.citations)}")
    except Exception as e:
        print(f"❌ Failed to compile context: {e}")
        return None
    
    # Format prompt
    # WorkingSet has .text attribute
    context_text = context_result.text
    # Truncate context if too long for prompt
    max_context_length = 1000
    if len(context_text) > max_context_length:
        context_text = context_text[:max_context_length] + "..."
    
    prompt = f"""Based on this code:

{context_text}

Question: {query}

Answer:"""
    
    print()
    print("Step 4: Generating answer with local model...")
    try:
        # Generate answer (deterministic: no sampling)
        answer = model(
            prompt,
            max_new_tokens=200,
            do_sample=False,  # Deterministic generation
            pad_token_id=model.tokenizer.eos_token_id,
            return_full_text=False  # Don't include prompt in output
        )
        
        # Extract answer (pipeline returns list of dicts)
        if isinstance(answer, list) and len(answer) > 0:
            if 'generated_text' in answer[0]:
                generated_text = answer[0]['generated_text']
                # Remove prompt if included
                if prompt in generated_text:
                    answer_text = generated_text[len(prompt):].strip()
                else:
                    answer_text = generated_text.strip()
            else:
                answer_text = str(answer[0])
        else:
            answer_text = str(answer)
        
        print("✅ Answer generated")
        print()
        print("-" * 60)
        print("ANSWER:")
        print("-" * 60)
        print(answer_text)
        print("-" * 60)
        print()
        
    except Exception as e:
        print(f"❌ Failed to generate answer: {e}")
        return None
    
    # 4. Test determinism
    print("Step 5: Testing determinism...")
    try:
        answer1 = model(
            prompt,
            max_new_tokens=200,
            do_sample=False,
            pad_token_id=model.tokenizer.eos_token_id,
            return_full_text=False
        )
        answer2 = model(
            prompt,
            max_new_tokens=200,
            do_sample=False,
            pad_token_id=model.tokenizer.eos_token_id,
            return_full_text=False
        )
        
        # Extract text from both answers
        def extract_text(ans):
            if isinstance(ans, list) and len(ans) > 0:
                if 'generated_text' in ans[0]:
                    text = ans[0]['generated_text']
                    return text[len(prompt):].strip() if prompt in text else text.strip()
                return str(ans[0])
            return str(ans)
        
        text1 = extract_text(answer1)
        text2 = extract_text(answer2)
        
        is_deterministic = (text1 == text2)
        
        if is_deterministic:
            print("✅ Deterministic: Same answer on both runs")
        else:
            print("⚠️  Non-deterministic: Different answers")
            print(f"   Run 1: {text1[:100]}...")
            print(f"   Run 2: {text2[:100]}...")
    except Exception as e:
        print(f"⚠️  Could not test determinism: {e}")
        is_deterministic = None
    
    print()
    print("=" * 60)
    print("POC RESULTS")
    print("=" * 60)
    print(f"✅ File indexed: {compiler_file}")
    print(f"✅ Query answered: {query}")
    print(f"✅ Citations: {len(context_result.citations)}")
    print(f"✅ Context spans: {len(context_result.spans)}")
    print(f"✅ Deterministic: {is_deterministic if is_deterministic is not None else 'Unknown'}")
    print()
    
    # Decision
    if is_deterministic and len(context_result.citations) > 0:
        print("🎉 POC SUCCESS!")
        print()
        print("Next steps:")
        print("  1. Review answer quality (does it make sense?)")
        print("  2. If good → run poc_10_queries.py")
        print("  3. If not → improve RAG or model selection")
        return True
    else:
        print("⚠️  POC needs improvement")
        print()
        if not is_deterministic:
            print("  - Fix determinism (check temperature=0, do_sample=False)")
        if len(context_result['citations']) == 0:
            print("  - Check citation generation")
        print("  - Review answer quality")
        return False


if __name__ == "__main__":
    print()
    success = test_minimal_poc()
    print()
    sys.exit(0 if success else 1)

