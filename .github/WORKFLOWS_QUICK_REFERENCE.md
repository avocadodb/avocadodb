# GitHub Actions Workflows - Quick Reference

## Workflow Triggers Summary

| Workflow | Push | PR | Schedule | Manual | Path Filters |
|----------|------|-----|----------|--------|--------------|
| **rust.yml** | ✅ main/master | ✅ | ❌ | ❌ | `**.rs`, `**/Cargo.toml`, `Cargo.lock` |
| **python.yml** | ✅ main/master | ✅ | ❌ | ❌ | `sdks/python/**`, `integrations/**/*.py` |
| **typescript.yml** | ✅ main/master | ✅ | ❌ | ❌ | `sdks/typescript/**` |
| **integration.yml** | ✅ main/master | ✅ | ❌ | ❌ | All files |
| **security.yml** | ✅ main/master | ✅ | ✅ Weekly Mon 9AM | ✅ | All files |
| **benchmark.yml** | ❌ | ✅ benches/** | ✅ Weekly Sun 3AM | ✅ | `benches/**`, `avocado-core/**` |

## Quick Commands

### Run Workflows Locally (Act)
```bash
# Install act
brew install act  # macOS
# or
curl https://raw.githubusercontent.com/nektos/act/master/install.sh | sudo bash

# Run specific workflow
act -W .github/workflows/rust.yml
act -W .github/workflows/python.yml

# Run all workflows
act
```

### Manually Trigger Workflows
```bash
# Using GitHub CLI
gh workflow run security.yml
gh workflow run benchmark.yml

# With inputs
gh workflow run benchmark.yml -f compare_with=feature-branch
```

### View Workflow Status
```bash
# List all runs
gh run list

# View specific run
gh run view <run-id>

# Watch a run
gh run watch
```

### Download Artifacts
```bash
# List artifacts
gh run list --limit 1
gh run view <run-id>

# Download
gh run download <run-id>
gh run download <run-id> -n benchmark-results
```

## Workflow Job Matrix

### Rust CI
```
test (ubuntu-latest, stable)
test (ubuntu-latest, beta)
test (macos-latest, stable)
test (macos-latest, beta)
clippy (ubuntu-latest)
fmt (ubuntu-latest)
build (ubuntu-latest, x86_64-unknown-linux-gnu)
build (macos-latest, x86_64-apple-darwin)
build (macos-latest, aarch64-apple-darwin)
coverage (ubuntu-latest)
```

### Python CI
```
test (3.9)
test (3.10)
test (3.11)
test (3.12)
lint (3.11)
integration-langchain (3.9)
integration-langchain (3.11)
integration-langchain (3.12)
integration-llamaindex (3.9)
integration-llamaindex (3.11)
integration-llamaindex (3.12)
check-format (3.11)
```

### TypeScript CI
```
build (18)
build (20)
lint (20)
typecheck (20)
test (18)
test (20)
package-check (20)
```

## Common Issues & Fixes

### Cache Issues
```bash
# Clear GitHub Actions cache (via API)
gh api repos/{owner}/{repo}/actions/caches -X DELETE

# Or manually in GitHub UI:
# Actions → Caches → Delete
```

### Re-run Failed Jobs
```bash
# Re-run failed jobs only
gh run rerun <run-id> --failed

# Re-run all jobs
gh run rerun <run-id>
```

### Skip CI on Commits
```bash
git commit -m "docs: update README [skip ci]"
# or
git commit -m "docs: update README [ci skip]"
```

## Environment Variables

### All Workflows
- `CARGO_TERM_COLOR: always`
- `RUST_BACKTRACE: 1` (Rust workflows)

### Required Secrets
- `CODECOV_TOKEN` - Coverage uploads

## Artifact Retention Periods

| Artifact Type | Retention |
|---------------|-----------|
| Build binaries | 7 days |
| Security reports | 30 days |
| Stress test results | 30 days |
| Benchmark results | 90 days |
| Benchmark baseline | 365 days |

## Coverage Flags

| Flag | Path | Purpose |
|------|------|---------|
| `rust` | Core Rust code | Main codebase coverage |
| `python-sdk` | `sdks/python/` | Python SDK coverage |
| `typescript-sdk` | `sdks/typescript/` | TypeScript SDK coverage |
| `langchain-integration` | `integrations/langchain-avocadodb/` | LangChain integration |
| `llamaindex-integration` | `integrations/llama-index-avocadodb/` | LlamaIndex integration |

## Status Badges

Add to README.md:

```markdown
# CI/CD Status

[![Rust CI](https://github.com/{owner}/{repo}/actions/workflows/rust.yml/badge.svg)](https://github.com/{owner}/{repo}/actions/workflows/rust.yml)
[![Python CI](https://github.com/{owner}/{repo}/actions/workflows/python.yml/badge.svg)](https://github.com/{owner}/{repo}/actions/workflows/python.yml)
[![TypeScript CI](https://github.com/{owner}/{repo}/actions/workflows/typescript.yml/badge.svg)](https://github.com/{owner}/{repo}/actions/workflows/typescript.yml)
[![Integration Tests](https://github.com/{owner}/{repo}/actions/workflows/integration.yml/badge.svg)](https://github.com/{owner}/{repo}/actions/workflows/integration.yml)
[![Security Scanning](https://github.com/{owner}/{repo}/actions/workflows/security.yml/badge.svg)](https://github.com/{owner}/{repo}/actions/workflows/security.yml)
[![codecov](https://codecov.io/gh/{owner}/{repo}/branch/master/graph/badge.svg)](https://codecov.io/gh/{owner}/{repo})
```

## Useful GitHub CLI Commands

```bash
# List workflows
gh workflow list

# View workflow details
gh workflow view rust.yml

# Enable/disable workflow
gh workflow enable rust.yml
gh workflow disable rust.yml

# View logs
gh run view --log

# Cancel a run
gh run cancel <run-id>

# List jobs in a run
gh run view <run-id> --log

# Download specific artifact
gh run download <run-id> --name rust-audit-report
```

## Debugging Workflow Runs

### Enable Debug Logging
Add secrets to repository:
- `ACTIONS_STEP_DEBUG: true`
- `ACTIONS_RUNNER_DEBUG: true`

### SSH into Runner (tmate)
Add this step to workflow:
```yaml
- name: Setup tmate session
  uses: mxschmitt/action-tmate@v3
  if: ${{ failure() }}
```

## Performance Tips

1. **Use path filters** - Workflows only run when needed
2. **Cache aggressively** - All workflows cache dependencies
3. **Parallel jobs** - Matrix runs jobs in parallel
4. **fail-fast: false** - One failure doesn't stop others
5. **Artifact cleanup** - Old artifacts auto-deleted

## Contact

For CI/CD issues:
1. Check workflow logs
2. Review this guide
3. Check GitHub Actions documentation
4. Open an issue in the repository
