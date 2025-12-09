import sys
import os
import argparse
from pathlib import Path
from typing import List

from avocado.client import AvocadoDB  # HTTP mode client
from avocado.manager import AvocadoDBManager  # daemon manager


def _print_citations(ws) -> None:
    if not ws.citations:
        return
    print("\nCitations:")
    for i, c in enumerate(ws.citations, 1):
        print(f"  [{i}] {c.artifact_path} (lines {c.start_line}-{c.end_line})")


def cmd_serve(args: argparse.Namespace) -> int:
    port = 8765
    mgr = AvocadoDBManager(port=port)
    ok = mgr.ensure_running()
    print(f"✓ AvocadoDB daemon {'running' if ok else 'started'} at http://127.0.0.1:{port}")
    return 0


def _iter_files(root: Path) -> List[Path]:
    exts = {".md", ".rs", ".ts", ".py", ".go", ".js", ".tsx", ".jsx", ".sql", ".proto", ".sh", ".toml", ".yaml", ".yml", ".json"}
    skip = {"node_modules", "dist", "build", ".terraform", "target", ".next", ".avocado", "venv", ".venv", "__pycache__"}
    max_bytes = 2_000_000
    out: List[Path] = []
    for r, ds, fs in os.walk(root):
        ds[:] = [d for d in ds if d not in skip]
        for f in fs:
            p = Path(r) / f
            try:
                if p.suffix.lower() in exts and p.stat().st_size < max_bytes:
                    out.append(p)
            except OSError:
                pass
    return out


def cmd_ingest(args: argparse.Namespace) -> int:
    db = AvocadoDB()  # HTTP mode, project=PWD
    path = Path(args.path)
    if path.is_dir():
        if not args.recursive:
            print("Directory provided; add --recursive to ingest all files")
            return 2
        files = _iter_files(path)
        print(f"Found {len(files)} files")
        ok = err = 0
        for i, p in enumerate(files, 1):
            try:
                res = db.ingest(str(p))
                ok += 1
            except Exception as e:
                err += 1
            if i % 100 == 0 or i == len(files):
                print(f"[{i}/{len(files)}] ok={ok} err={err}")
        print(f"✓ Ingest completed: ok={ok} err={err}")
        return 0 if err == 0 else 1
    else:
        res = db.ingest(str(path))
        print(f"✓ Ingested {path} → spans={res.get('spans_created', 0)}")
        return 0


def cmd_compile(args: argparse.Namespace) -> int:
    db = AvocadoDB()
    ws = db.compile(args.query, budget=args.budget)
    print(ws.text)
    print("\n" + "─" * 60)
    print(f"Tokens:   {ws.tokens_used} / {args.budget}")
    print(f"Compiled: {len(ws.spans)} spans")
    print(f"Time:     {ws.compilation_time_ms}ms")
    _print_citations(ws)
    return 0


def cmd_stats(args: argparse.Namespace) -> int:
    db = AvocadoDB()
    s = db.stats()
    print(f"Artifacts: {s.get('artifacts_count') or s.get('artifacts')}")
    print(f"Spans:     {s.get('spans_count') or s.get('spans')}")
    print(f"Tokens:    {s.get('total_tokens') or s.get('tokens')}")
    return 0


def cmd_clear(args: argparse.Namespace) -> int:
    # Call HTTP /clear?project=$PWD
    import requests
    db = AvocadoDB()
    url = f"{db.url}/clear"
    r = requests.delete(url, params={"project": db.project_path})
    r.raise_for_status()
    print("✓ Cleared all data for project")
    return 0


def cmd_session_start(args: argparse.Namespace) -> int:
    db = AvocadoDB()
    s = db.create_session(user_id=args.user, title=args.title)
    print(s.id)
    # Save as current session
    cs = Path(".avocado/current_session")
    cs.parent.mkdir(parents=True, exist_ok=True)
    cs.write_text(s.id, encoding="utf-8")
    print(f"✓ Active session: {s.id}")
    return 0


def _load_active_session_id() -> str:
    cs = Path(".avocado/current_session")
    if cs.exists():
        return cs.read_text(encoding="utf-8").strip()
    raise SystemExit("No active session. Use: avacado-cli session start --user <id> --title <t>")


def cmd_session_compile(args: argparse.Namespace) -> int:
    db = AvocadoDB()
    sid = args.session or _load_active_session_id()
    s = db.get_session(sid)
    res = s.compile(args.query, {"budget": args.budget})
    ws = res.workingSet
    print(ws.text)
    print("\n" + "─" * 60)
    print(f"Tokens:   {ws.tokensUsed} / {args.budget}")
    print(f"Compiled: {len(ws.spans)} spans")
    return 0


def cmd_session_history(args: argparse.Namespace) -> int:
    db = AvocadoDB()
    sid = args.session or _load_active_session_id()
    s = db.get_session(sid)
    h = s.get_history(max_tokens=args.max_tokens)
    print(h)
    return 0


def cmd_session_replay(args: argparse.Namespace) -> int:
    db = AvocadoDB()
    sid = args.session or _load_active_session_id()
    s = db.get_session(sid)
    r = s.replay()
    print(f"Session: {r.session.id} turns={len(r.turns)}")
    return 0


def cli_main(argv=None) -> int:
    ap = argparse.ArgumentParser(prog="avocado-cli (SDK wrapper)")
    sp = ap.add_subparsers(dest="cmd", required=True)

    p_serve = sp.add_parser("serve", help="Start or ensure AvocadoDB daemon is running")
    p_serve.set_defaults(fn=cmd_serve)

    p_ing = sp.add_parser("ingest", help="Ingest path (file or directory with --recursive)")
    p_ing.add_argument("path")
    p_ing.add_argument("--recursive", action="store_true")
    p_ing.set_defaults(fn=cmd_ingest)

    p_cmp = sp.add_parser("compile", help="Compile context for a query")
    p_cmp.add_argument("query")
    p_cmp.add_argument("--budget", type=int, default=8000)
    p_cmp.set_defaults(fn=cmd_compile)

    p_stat = sp.add_parser("stats", help="Show database statistics")
    p_stat.set_defaults(fn=cmd_stats)

    p_clear = sp.add_parser("clear", help="Clear data for current project")
    p_clear.set_defaults(fn=cmd_clear)

    p_sess = sp.add_parser("session", help="Session management commands")
    ssp = p_sess.add_subparsers(dest="scmd", required=True)

    s_start = ssp.add_parser("start", help="Create a new session")
    s_start.add_argument("--user", help="User ID", default=None)
    s_start.add_argument("--title", help="Session title", default=None)
    s_start.set_defaults(fn=cmd_session_start)

    s_compile = ssp.add_parser("compile", help="Compile a query in the active or specified session")
    s_compile.add_argument("query")
    s_compile.add_argument("--budget", type=int, default=8000)
    s_compile.add_argument("--session", help="Session ID", default=None)
    s_compile.set_defaults(fn=cmd_session_compile)

    s_hist = ssp.add_parser("history", help="Show session history")
    s_hist.add_argument("--session", help="Session ID", default=None)
    s_hist.add_argument("--max-tokens", type=int, default=None)
    s_hist.set_defaults(fn=cmd_session_history)

    s_replay = ssp.add_parser("replay", help="Replay session")
    s_replay.add_argument("--session", help="Session ID", default=None)
    s_replay.set_defaults(fn=cmd_session_replay)

    args = ap.parse_args(argv)
    return args.fn(args)


if __name__ == "__main__":
    sys.exit(cli_main())



