#!/usr/bin/env python3
import json
import sys
import time
import argparse
import os
from urllib.parse import urlparse
import requests
from pathlib import Path

def compile_query(base_url: str, project: str, query: str, budget: int = 8000,
                  semantic_weight: float = 0.7, lexical_weight: float = 0.3,
                  enable_mmr: bool = True) -> dict:
    t0 = time.time()
    r = requests.post(f"{base_url.rstrip('/')}/compile", json={
        "query": query,
        "token_budget": budget,
        "config": {
            "semantic_weight": semantic_weight,
            "lexical_weight": lexical_weight,
            "enable_mmr": enable_mmr,
        },
        "project": project,
    })
    latency_ms = int((time.time() - t0) * 1000)
    r.raise_for_status()
    data = r.json()["working_set"]
    return {"ws": data, "latency_ms": latency_ms}

def recall_at_terms(text: str, terms: list[str]) -> float:
    if not terms:
        return 1.0
    tl = text.lower()
    hits = sum(1 for t in terms if t.lower() in tl)
    return hits / len(terms)

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--base-url", default="http://localhost:8765")
    ap.add_argument("--project", default=str(Path.cwd()))
    ap.add_argument("--dataset", required=False, default=str(Path(__file__).parent / "samples" / "code_qa.jsonl"))
    ap.add_argument("--budget", type=int, default=8000)
    args = ap.parse_args()

    # SSRF guard: allow only http(s) and localhost/127.0.0.1 by default.
    raw_url = args.base_url.strip()
    parsed = urlparse(raw_url)
    if parsed.scheme not in ("http", "https"):
        print(f"[ERROR] Invalid scheme in --base-url: {parsed.scheme}", file=sys.stderr)
        sys.exit(2)
    allowed_hosts = {h.strip() for h in os.environ.get("AVOCADO_EVALS_ALLOWED_HOSTS", "localhost,127.0.0.1").split(",")}
    hostname = parsed.hostname or ""
    if hostname not in allowed_hosts:
        print(f"[ERROR] Host not allowed for --base-url: {hostname}. Allowed: {sorted(allowed_hosts)}", file=sys.stderr)
        sys.exit(2)
    base_url = f"{parsed.scheme}://{hostname}"
    if parsed.port:
        base_url += f":{parsed.port}"

    project = args.project
    ds_path = Path(args.dataset)
    if not ds_path.exists():
        print(f"Dataset not found: {ds_path}", file=sys.stderr)
        sys.exit(1)

    total = 0
    avoc_sum = 0.0
    lex_sum = 0.0
    avoc_lat = 0
    lex_lat = 0

    with ds_path.open() as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            ex = json.loads(line)
            q = ex["question"]
            terms = ex.get("answer_terms", [])

            # Avocado default
            try:
                av = compile_query(base_url, project, q, budget=args.budget,
                                   semantic_weight=0.7, lexical_weight=0.3, enable_mmr=True)
                avoc_lat += av["latency_ms"]
                av_rec = recall_at_terms(av["ws"]["text"], terms)
            except Exception as e:
                print(f"[WARN] avocado compile failed: {e}", file=sys.stderr)
                av_rec = 0.0

            # Lexical-only baseline
            try:
                lx = compile_query(base_url, project, q, budget=args.budget,
                                   semantic_weight=0.0, lexical_weight=1.0, enable_mmr=False)
                lex_lat += lx["latency_ms"]
                lex_rec = recall_at_terms(lx["ws"]["text"], terms)
            except Exception as e:
                print(f"[WARN] lexical compile failed: {e}", file=sys.stderr)
                lex_rec = 0.0

            total += 1
            avoc_sum += av_rec
            lex_sum += lex_rec

    if total == 0:
        print(json.dumps({"error": "empty dataset"}, indent=2))
        sys.exit(1)

    summary = {
        "total": total,
        "recall_terms_avg": {
            "avocado": round(avoc_sum / total, 3),
            "lexical_baseline": round(lex_sum / total, 3),
        },
        "latency_ms_avg": {
            "avocado": int(avoc_lat / total),
            "lexical_baseline": int(lex_lat / total),
        },
    }
    print(json.dumps(summary, indent=2))

if __name__ == "__main__":
    main()

