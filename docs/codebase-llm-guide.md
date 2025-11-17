# Training a Codebase-Aware LLM for Agent Queries

**Goal**: Create a lightweight, fast LLM that can answer questions about the AvocadoDB codebase for use by other agents in their reasoning chains.

## Strategy: Hybrid RAG + Fine-Tuning

### Option 1: Pure RAG (Recommended for MVP)

**Use AvocadoDB to index itself** - meta but effective!

```bash
# 1. Index the entire codebase
avocado init
avocado ingest ./avocado-core/src --recursive
avocado ingest ./avocado-server/src --recursive
avocado ingest ./avocado-cli/src --recursive
avocado ingest ./docs --recursive
avocado ingest README.md .vision/vision.md

# 2. Create a lightweight wrapper service
```

**Advantages:**

- ✅ No training needed
- ✅ Always up-to-date (just re-ingest on changes)
- ✅ Deterministic answers
- ✅ Perfect citations
- ✅ Fast to deploy

**Implementation:**

```python
# codebase_qa_service.py
from avocado import AvocadoDB
from vllm import LLM, SamplingParams
import os

class CodebaseQA:
    def __init__(self):
        # AvocadoDB for retrieval
        self.db = AvocadoDB(server_url="http://localhost:8765")

        # Small model for synthesis (7B-13B range)
        self.llm = LLM(
            model="deepseek-ai/DeepSeek-Coder-6.7B-Instruct",  # or CodeLlama-7B
            tensor_parallel_size=1,  # Adjust for your GPU
            gpu_memory_utilization=0.8
        )
        self.sampling_params = SamplingParams(
            temperature=0.1,  # Low for factual answers
            max_tokens=512,
            top_p=0.95
        )

    async def answer(self, question: str) -> dict:
        # Step 1: Retrieve relevant context using AvocadoDB
        context = self.db.compile(
            query=question,
            token_budget=4000,  # Leave room for model response
            config={"enable_mmr": True}
        )

        # Step 2: Synthesize answer with small model
        prompt = f"""You are a codebase expert for AvocadoDB. Answer the question using ONLY the provided context.

Context:
{context.text}

Citations:
{self._format_citations(context.citations)}

Question: {question}

Answer (be concise, cite sources):"""

        outputs = self.llm.generate([prompt], self.sampling_params)
        answer = outputs[0].outputs[0].text

        return {
            "answer": answer,
            "citations": context.citations,
            "context_used": len(context.spans)
        }

    def _format_citations(self, citations):
        return "\n".join([
            f"[{i+1}] {c.artifact_path}:{c.start_line}-{c.end_line}"
            for i, c in enumerate(citations)
        ])
```

### Option 2: Fine-Tuned Small Model (For Better Performance)

**When to use**: When you need faster responses (<100ms) and can't afford RAG latency.

**Model Selection:**

| Model                     | Size | Speed  | Code Quality | Recommendation   |
| ------------------------- | ---- | ------ | ------------ | ---------------- |
| **DeepSeek-Coder-1.3B**   | 1.3B | ⚡⚡⚡ | ⭐⭐⭐       | Best for speed   |
| **CodeLlama-7B-Instruct** | 7B   | ⚡⚡   | ⭐⭐⭐⭐     | Balanced         |
| **StarCoder2-7B**         | 7B   | ⚡⚡   | ⭐⭐⭐⭐     | Good alternative |
| **DeepSeek-Coder-6.7B**   | 6.7B | ⚡⚡   | ⭐⭐⭐⭐⭐   | Best quality     |

**Training Data Preparation:**

```python
# prepare_training_data.py
from avocado import AvocadoDB
import json

def generate_qa_pairs():
    """Generate Q&A pairs from codebase"""
    db = AvocadoDB()

    # Questions agents might ask
    questions = [
        "How does the vector index caching work?",
        "What is the span extraction algorithm?",
        "How does deterministic sorting ensure reproducibility?",
        "What are the database schema migrations?",
        "How does the compiler handle token budgeting?",
        "What is the MMR diversification algorithm?",
        "How are embeddings generated and stored?",
        "What is the architecture of the HTTP server?",
        "How does the CLI handle batch ingestion?",
        "What error handling patterns are used?",
    ]

    training_data = []
    for q in questions:
        # Get context
        context = db.compile(q, token_budget=6000)

        # Generate answer (use GPT-4 or Claude for high-quality answers)
        answer = generate_answer(q, context)  # Your LLM call here

        training_data.append({
            "instruction": q,
            "input": context.text[:2000],  # Truncate if needed
            "output": answer,
            "citations": [c.artifact_path for c in context.citations]
        })

    return training_data

# Save in format for fine-tuning
# Format: Alpaca, ShareGPT, or your framework's format
```

**Fine-Tuning with vLLM-Compatible Framework:**

```bash
# Using Unsloth (fast LoRA fine-tuning)
pip install unsloth transformers datasets

# Fine-tune script
python fine_tune_codebase.py \
    --model_name "deepseek-ai/DeepSeek-Coder-1.3B-Instruct" \
    --dataset codebase_qa.json \
    --output_dir ./models/avocadodb-qa-1.3b \
    --lora_r 16 \
    --lora_alpha 32 \
    --num_epochs 3 \
    --batch_size 4 \
    --learning_rate 2e-4
```

**Inference with vLLM:**

```python
from vllm import LLM, SamplingParams

# Load fine-tuned model
llm = LLM(
    model="./models/avocadodb-qa-1.3b",
    tensor_parallel_size=1,
    gpu_memory_utilization=0.9,
    enable_lora=True,  # If using LoRA
    max_lora_rank=16
)

sampling_params = SamplingParams(
    temperature=0.1,
    max_tokens=512,
    top_p=0.95
)

# Fast inference
def answer(question: str) -> str:
    prompt = f"Question: {question}\nAnswer:"
    outputs = llm.generate([prompt], sampling_params)
    return outputs[0].outputs[0].text
```

### Option 3: Hybrid Approach (Best of Both Worlds)

**Combine fine-tuned model + RAG for accuracy:**

```python
class HybridCodebaseQA:
    def __init__(self):
        self.db = AvocadoDB()
        self.llm = LLM(model="avocadodb-qa-1.3b")

    async def answer(self, question: str) -> dict:
        # Fast path: Try fine-tuned model first
        quick_answer = self.llm.generate([question], sampling_params)[0].outputs[0].text

        # Verify with RAG if confidence is low (optional)
        if self._needs_verification(question, quick_answer):
            context = self.db.compile(question, token_budget=2000)
            # Cross-check or enhance answer
            verified_answer = self._verify_with_context(quick_answer, context)
            return {"answer": verified_answer, "source": "hybrid"}

        return {"answer": quick_answer, "source": "fine-tuned"}
```

## Recommended Architecture for Agent Integration

```
┌─────────────────┐
│  Agent System   │
│  (Reasoning)    │
└────────┬────────┘
         │
         │ query: "How does caching work?"
         ▼
┌─────────────────────────────────────┐
│   Codebase QA Service               │
│   ┌──────────────┐  ┌─────────────┐│
│   │  AvocadoDB   │  │  vLLM       ││
│   │  (RAG)       │→ │  (Synthesis)││
│   └──────────────┘  └─────────────┘│
└────────┬────────────────────────────┘
         │
         │ answer + citations
         ▼
┌─────────────────┐
│  Agent uses in  │
│  next step      │
└─────────────────┘
```

## Implementation Steps

### Step 1: Setup vLLM Service

```bash
# Install vLLM
pip install vllm

# Start inference server
python -m vllm.entrypoints.openai.api_server \
    --model deepseek-ai/DeepSeek-Coder-1.3B-Instruct \
    --port 8000 \
    --tensor-parallel-size 1
```

### Step 2: Index Codebase with AvocadoDB

```bash
# Start AvocadoDB server
./target/release/avocado-server &

# Index everything
avocado ingest ./avocado-core/src --recursive
avocado ingest ./avocado-server/src --recursive
avocado ingest ./avocado-cli/src --recursive
avocado ingest ./docs --recursive
avocado ingest README.md .vision/vision.md .implementation/plan.yaml
```

### Step 3: Create Agent Tool

```python
# agent_tool.py
from typing import Optional
from avocado import AvocadoDB
import openai  # or vllm client

class CodebaseQueryTool:
    """Tool for agents to query the codebase"""

    def __init__(self):
        self.db = AvocadoDB(server_url="http://localhost:8765")
        self.client = openai.OpenAI(
            base_url="http://localhost:8000/v1",  # vLLM server
            api_key="dummy"  # vLLM doesn't need real key
        )

    def query(self, question: str, max_tokens: int = 512) -> dict:
        """
        Query the codebase and return answer with citations.

        Args:
            question: Natural language question about the codebase
            max_tokens: Maximum tokens in response

        Returns:
            {
                "answer": str,
                "citations": List[Citation],
                "confidence": float
            }
        """
        # Retrieve context
        context = self.db.compile(
            query=question,
            token_budget=4000,
            config={"enable_mmr": True}
        )

        # Generate answer
        response = self.client.chat.completions.create(
            model="deepseek-coder",  # Your model name
            messages=[
                {
                    "role": "system",
                    "content": "You are a codebase expert. Answer questions using ONLY the provided context. Always cite sources."
                },
                {
                    "role": "user",
                    "content": f"""Context:
{context.text}

Citations:
{self._format_citations(context.citations)}

Question: {question}

Answer:"""
                }
            ],
            temperature=0.1,
            max_tokens=max_tokens
        )

        answer = response.choices[0].message.content

        return {
            "answer": answer,
            "citations": context.citations,
            "context_spans": len(context.spans),
            "tokens_used": context.tokens_used
        }

    def _format_citations(self, citations):
        return "\n".join([
            f"[{i+1}] {c.artifact_path}:{c.start_line}-{c.end_line}"
            for i, c in enumerate(citations)
        ])

# Usage in agent
tool = CodebaseQueryTool()
result = tool.query("How does vector index caching work?")
print(result["answer"])
```

## Performance Optimization

### For Speed (Sub-100ms responses):

1. **Use smallest model**: DeepSeek-Coder-1.3B or CodeLlama-3B
2. **Quantize**: Use AWQ or GPTQ quantization
3. **Cache common queries**: Redis cache for frequent questions
4. **Batch processing**: Process multiple queries together

```python
# Quantized model loading
from vllm import LLM

llm = LLM(
    model="deepseek-ai/DeepSeek-Coder-1.3B-Instruct",
    quantization="awq",  # or "gptq"
    tensor_parallel_size=1
)
```

### For Accuracy:

1. **Larger context window**: Use 8K+ token budget
2. **Hybrid retrieval**: Combine semantic + lexical search
3. **Post-processing**: Verify answers against source code
4. **Fine-tuning**: Train on codebase-specific patterns

## Cost Analysis

| Approach       | Setup Time | Inference Speed | Accuracy   | Maintenance                       |
| -------------- | ---------- | --------------- | ---------- | --------------------------------- |
| **Pure RAG**   | 1 hour     | 200-500ms       | ⭐⭐⭐⭐   | Low (re-ingest on changes)        |
| **Fine-Tuned** | 1-2 days   | 50-100ms        | ⭐⭐⭐⭐⭐ | Medium (retrain on major changes) |
| **Hybrid**     | 1-2 days   | 100-200ms       | ⭐⭐⭐⭐⭐ | Medium                            |

## Recommendation

**Start with Pure RAG (Option 1)** because:

- ✅ Fastest to deploy
- ✅ Always accurate (uses actual code)
- ✅ No training infrastructure needed
- ✅ Easy to update (just re-ingest)

**Upgrade to Fine-Tuning (Option 2)** if:

- You need <100ms response times
- You have many repetitive queries
- You want to reduce API costs
- You have GPU resources available

## Example Agent Integration

```python
# In your agent system
from agent_tool import CodebaseQueryTool

class ReasoningAgent:
    def __init__(self):
        self.codebase_qa = CodebaseQueryTool()
        self.llm = ...  # Your main agent LLM

    async def reason(self, task: str):
        # Step 1: Query codebase for relevant info
        codebase_info = self.codebase_qa.query(
            f"What do I need to know about {task}?"
        )

        # Step 2: Use in reasoning
        reasoning_prompt = f"""
        Task: {task}

        Codebase Context:
        {codebase_info['answer']}

        Citations: {codebase_info['citations']}

        Based on this information, what should I do?
        """

        response = await self.llm.complete(reasoning_prompt)
        return response
```

## Next Steps

1. **Start simple**: Deploy RAG-based solution first
2. **Measure**: Track query patterns and response times
3. **Optimize**: Fine-tune if needed based on usage
4. **Scale**: Add caching, batching, quantization as needed
