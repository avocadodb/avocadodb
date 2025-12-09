# CI/CD Infrastructure Setup for AvocadoDB

This document provides an overview of the comprehensive CI/CD infrastructure set up for AvocadoDB using GitHub Actions.

## Overview

The CI/CD pipeline consists of 6 workflow files covering:
- Rust testing and builds
- Python SDK and integrations testing
- TypeScript SDK testing
- End-to-end integration tests
- Security scanning
- Performance benchmarking

---

## Workflow Files

### 1. Rust CI (`rust.yml`)

**Triggers:** Push and PR to main/master (when Rust files change)

**Jobs:**
- **Test Suite** - Runs on Ubuntu and macOS with Rust stable and beta
  - Executes `cargo test --all`
  - Runs doc tests
  - Matrix: 2 OS × 2 Rust versions = 4 test jobs
  
- **Clippy** - Linting with zero warnings tolerance
  - `cargo clippy --all-targets --all-features -- -D warnings`
  
- **Rustfmt** - Code formatting check
  - `cargo fmt --all -- --check`
  
- **Build** - Creates release binaries for multiple platforms
  - Linux x86_64
  - macOS x86_64
  - macOS ARM64 (Apple Silicon)
  - Uploads artifacts with 7-day retention
  
- **Coverage** - Code coverage with cargo-tarpaulin
  - Uploads to Codecov with `rust` flag

**Caching:**
- Cargo registry
- Cargo index
- Target directory (per job type)

---

### 2. Python CI (`python.yml`)

**Triggers:** Push and PR (when Python files change)

**Jobs:**
- **Test Suite** - Runs pytest with coverage
  - Matrix: Python 3.9, 3.10, 3.11, 3.12
  - Coverage uploaded to Codecov per version
  
- **Lint & Type Check** - Code quality checks
  - Black (formatting check)
  - Flake8 (linting with max line length 120)
  - MyPy (type checking)
  
- **LangChain Integration** - Tests langchain-avocadodb package
  - Matrix: Python 3.9, 3.11, 3.12
  - Separate coverage flag: `langchain-integration`
  
- **LlamaIndex Integration** - Tests llama-index-avocadodb package
  - Matrix: Python 3.9, 3.11, 3.12
  - Separate coverage flag: `llamaindex-integration`
  
- **Package Metadata Verification**
  - Builds packages with `python -m build`
  - Validates with `twine check`
  - Checks SDK + both integrations

**Caching:**
- pip cache (automatic with setup-python)

---

### 3. TypeScript CI (`typescript.yml`)

**Triggers:** Push and PR (when TypeScript files change)

**Jobs:**
- **Build** - Compiles TypeScript SDK
  - Matrix: Node.js 18, 20
  - Uploads dist artifacts
  
- **Lint & Format Check**
  - ESLint (if configured)
  - Prettier check (if configured)
  - Gracefully continues if not configured
  
- **Type Check** - TypeScript compiler check
  - `tsc --noEmit`
  
- **Test Suite** - Jest tests
  - Matrix: Node.js 18, 20
  - Auto-detects test directories
  
- **Package Verification**
  - Creates npm package with `npm pack`
  - Validates package contents

**Caching:**
- npm cache (automatic with setup-node)

---

### 4. Integration Tests (`integration.yml`)

**Triggers:** Push and PR to main/master

**Jobs:**
- **E2E Tests** - End-to-end testing
  - Builds and starts avocado-server
  - Runs integration tests from `tests` package
  - Waits for server health check
  - Properly stops server after tests
  
- **Python SDK Integration**
  - Starts server in background
  - Runs Python SDK integration tests
  - Tests session management features
  
- **TypeScript SDK Integration**
  - Starts server in background
  - Runs TypeScript integration tests (if available)
  
- **CLI Commands Test**
  - Tests CLI basic commands (--version, --help)
  - Tests ingest command
  - Tests ask command
  
- **Framework Integrations**
  - Tests LangChain integration with live server
  - Tests LlamaIndex integration with live server

**Key Features:**
- All jobs start avocado-server in background
- Health checks ensure server is ready
- Always stops server (even on failure)
- Tests real client-server interactions

---

### 5. Security Scanning (`security.yml`)

**Triggers:** Push, PR, weekly schedule (Mondays 9 AM UTC), manual dispatch

**Jobs:**
- **CodeQL Analysis** - Advanced security scanning
  - Separate jobs for Rust, Python, JavaScript/TypeScript
  - Uses security-and-quality queries
  - Results published to Security tab
  
- **Cargo Audit** - Rust dependency vulnerabilities
  - Checks for known security advisories
  - Generates JSON report
  - Uploads report as artifact (30-day retention)
  
- **Cargo Deny** - License and security compliance
  - Checks advisories
  - Validates licenses
  
- **Python Safety** - Python dependency security
  - Checks SDK and both integrations
  - Generates safety reports
  - 30-day artifact retention
  
- **Python Bandit** - Security linting
  - Scans Python code for security issues
  - Generates JSON report
  
- **NPM Audit** - TypeScript/JavaScript dependencies
  - Checks for vulnerabilities
  - Generates audit report
  
- **Dependency Review** - PR-only check
  - Reviews new dependencies in PRs
  - Fails on moderate+ severity issues
  
- **Secret Scanning** - TruffleHog
  - Scans for accidentally committed secrets
  - Only checks verified secrets

**Report Artifacts:**
- Rust audit report (30 days)
- Python safety report (30 days)
- Bandit security report (30 days)
- NPM audit report (30 days)

---

### 6. Benchmarks (`benchmark.yml`)

**Triggers:** Manual dispatch, weekly schedule (Sundays 3 AM UTC), PR (if benchmark files change)

**Jobs:**
- **Benchmark** - Standard benchmark execution
  - Runs embedding benchmarks from avocado-cli
  - Runs session benchmarks
  - Stores results (90-day retention)
  - Compares with previous baseline
  - Saves new baseline on schedule/manual runs
  
- **Benchmark Comparison (PR)** - PR-specific comparison
  - Runs benchmarks on PR code
  - Runs benchmarks on base branch
  - Uses `critcmp` to compare results
  - Posts comparison as PR comment
  
- **Stress Test** - Load testing (manual/scheduled only)
  - Starts avocado-server
  - Simulates 100 concurrent requests
  - Collects server metrics
  - Uploads stress test results (30-day retention)

**Manual Dispatch Options:**
- `compare_with`: Git ref to compare against (default: master)

---

## Codecov Configuration (`codecov.yml`)

**Coverage Targets:**
- Project: 80% (±2% threshold)
- Patch: 70% (±5% threshold)

**Ignored Paths:**
- Test files
- Examples
- Benchmarks
- Build artifacts (target, dist, node_modules)

**Flags:**
- `rust` - Core Rust codebase
- `python-sdk` - Python SDK
- `typescript-sdk` - TypeScript SDK
- `langchain-integration` - LangChain integration
- `llamaindex-integration` - LlamaIndex integration

**PR Comments:**
- Shows diff, files, and footer
- Requires head coverage (not base)
- Annotations enabled

---

## Caching Strategy

All workflows implement aggressive caching to speed up CI runs:

### Rust Workflows
- Cargo registry: `~/.cargo/registry`
- Cargo index: `~/.cargo/git`
- Build artifacts: `target/`
- Cache keys include Cargo.lock hash for invalidation

### Python Workflows
- pip cache: Automatic via `setup-python` action
- Shared across all Python jobs

### TypeScript Workflows
- npm cache: Automatic via `setup-node` action
- Uses package-lock.json for cache key

---

## Matrix Strategies

### Multi-Version Testing

**Rust:**
- OS: Ubuntu Latest, macOS Latest
- Rust: Stable, Beta
- Total: 4 combinations

**Python:**
- Versions: 3.9, 3.10, 3.11, 3.12
- SDK tested on all 4 versions
- Integrations tested on 3.9, 3.11, 3.12

**TypeScript:**
- Node.js: 18, 20
- Tests and builds on both versions

**Build Targets:**
- Linux x86_64
- macOS x86_64
- macOS ARM64 (Apple Silicon)

---

## Security Scanning Coverage

### Languages Covered
- Rust (CodeQL, cargo-audit, cargo-deny)
- Python (CodeQL, Safety, Bandit)
- JavaScript/TypeScript (CodeQL, npm audit)

### Scan Types
- **Static Analysis**: CodeQL for all languages
- **Dependency Vulnerabilities**: cargo-audit, Safety, npm audit
- **License Compliance**: cargo-deny
- **Security Linting**: Bandit for Python
- **Secret Detection**: TruffleHog
- **Dependency Review**: GitHub native (PR only)

### Scheduling
- Weekly scans every Monday at 9 AM UTC
- On every push and PR
- Manual dispatch available

---

## Required GitHub Secrets

Add these secrets in GitHub repository settings (Settings → Secrets and variables → Actions):

### Required
- `CODECOV_TOKEN` - For uploading coverage reports to Codecov
  - Get from: https://codecov.io/gh/[your-org]/avacadodb

### Optional (for future enhancements)
- `NPM_TOKEN` - For publishing to npm (future)
- `PYPI_TOKEN` - For publishing to PyPI (future)
- `DOCKER_USERNAME` - For Docker Hub (if containerization added)
- `DOCKER_PASSWORD` - For Docker Hub (if containerization added)

---

## Verification Checklist for First CI Run

### Pre-Push Checks
- [ ] Add `CODECOV_TOKEN` to GitHub repository secrets
- [ ] Verify all Cargo.toml files are valid
- [ ] Ensure setup.py and pyproject.toml are valid
- [ ] Check package.json is valid
- [ ] Review security scanning permissions

### First Push Verification
- [ ] **Rust CI**: All 4 test matrix jobs pass (2 OS × 2 Rust versions)
- [ ] **Rust CI**: Clippy passes with no warnings
- [ ] **Rust CI**: Formatting check passes
- [ ] **Rust CI**: 3 release binaries built successfully
- [ ] **Rust CI**: Coverage uploaded to Codecov

- [ ] **Python CI**: Tests pass on all 4 Python versions
- [ ] **Python CI**: Black, Flake8, MyPy all pass
- [ ] **Python CI**: LangChain integration tests pass
- [ ] **Python CI**: LlamaIndex integration tests pass
- [ ] **Python CI**: All 3 packages build and validate

- [ ] **TypeScript CI**: Builds on Node 18 and 20
- [ ] **TypeScript CI**: Type checking passes
- [ ] **TypeScript CI**: Package builds successfully

- [ ] **Integration**: Server starts and health check succeeds
- [ ] **Integration**: E2E tests complete
- [ ] **Integration**: Python SDK integration tests pass
- [ ] **Integration**: CLI commands work

- [ ] **Security**: CodeQL scans complete for all 3 languages
- [ ] **Security**: Dependency audits run (may have warnings)
- [ ] **Security**: Reports uploaded to artifacts

### Expected Warnings/Failures (First Run)
These are normal and can be addressed iteratively:
- ESLint/Prettier may not be configured (continues gracefully)
- Jest tests may not exist yet (continues gracefully)
- Some security advisories may exist (reports generated)
- Some integrations tests may be incomplete

### Coverage Expectations
- Initial coverage may be below 80% target
- Coverage should be visible on Codecov dashboard
- Each component (rust, python-sdk, etc.) tracked separately

---

## Workflow Optimization Features

### Fail-Fast Disabled
Matrix jobs run independently - one failure doesn't stop others

### Conditional Execution
- Benchmark comparison only runs on PRs
- Stress tests only on manual/scheduled runs
- Dependency review only on PRs
- Secret scanning checks against base branch

### Path Filtering
Workflows only trigger when relevant files change:
- Rust workflow: `**.rs`, `**/Cargo.toml`, `Cargo.lock`
- Python workflow: `sdks/python/**`, `integrations/**/*.py`
- TypeScript workflow: `sdks/typescript/**`

### Continue on Error
Security scans use `continue-on-error: true` to:
- Generate reports even with findings
- Not block development
- Provide visibility without blocking

---

## Monitoring and Maintenance

### Regular Reviews
- Check security scan artifacts weekly
- Review benchmark trends monthly
- Update dependencies regularly
- Monitor coverage trends

### Artifact Retention
- Build artifacts: 7 days
- Security reports: 30 days
- Stress test results: 30 days
- Benchmark results: 90 days
- Benchmark baseline: 365 days

### When to Trigger Manual Workflows

**Benchmarks:**
- Before major releases
- After performance optimizations
- To compare branches: use `compare_with` input

**Security Scans:**
- After updating dependencies
- Before releases
- When investigating security issues

---

## Troubleshooting Common Issues

### Build Failures
1. Check cache invalidation - may need to clear caches
2. Verify Cargo.lock is committed
3. Check Rust version compatibility

### Coverage Upload Failures
1. Verify `CODECOV_TOKEN` is set
2. Check token hasn't expired
3. Review Codecov dashboard for errors

### Integration Test Failures
1. Check server startup logs
2. Verify health check endpoint responds
3. Ensure server.pid cleanup works

### Security Scan False Positives
1. Review artifact reports
2. Add exemptions to cargo-deny.toml if needed
3. Update vulnerable dependencies

---

## Next Steps

1. **Enable Branch Protection**
   - Require status checks to pass
   - Require Rust CI, Python CI, TypeScript CI
   - Require security scans

2. **Set up Codecov Integration**
   - Link repository to Codecov
   - Configure coverage comments on PRs

3. **Add Release Workflow**
   - Automate versioning
   - Publish to crates.io, PyPI, npm
   - Create GitHub releases

4. **Add Deployment Workflow**
   - Deploy to staging/production
   - Docker container builds
   - Documentation deployment

5. **Configure Dependabot**
   - Automated dependency updates
   - Security vulnerability alerts

---

## Summary Statistics

- **Total Workflows:** 6
- **Total Jobs:** 28+
- **Total Lines of YAML:** 1,377
- **Languages Tested:** Rust, Python, TypeScript
- **Python Versions:** 4 (3.9-3.12)
- **Node.js Versions:** 2 (18, 20)
- **Rust Versions:** 2 (stable, beta)
- **Build Platforms:** 3 (Linux x64, macOS x64, macOS ARM64)
- **Security Scanners:** 8 (CodeQL×3, cargo-audit, cargo-deny, Safety, Bandit, npm audit)
- **Coverage Tracking:** 5 flags (rust, python-sdk, typescript-sdk, langchain, llamaindex)

---

**Created:** 2025-11-17  
**Maintained by:** AvocadoDB Team  
**For Questions:** See repository issues or discussions
