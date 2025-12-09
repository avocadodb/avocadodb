# Release Quick Start Guide

Quick reference for creating AvocadoDB releases.

## Prerequisites

- [ ] All tests passing
- [ ] On master branch
- [ ] No uncommitted changes
- [ ] GitHub secrets configured (PYPI_TOKEN, NPM_TOKEN)

## Core Release (Binaries)

```bash
# Interactive release
./scripts/release.sh

# What you'll be asked:
# 1. New version (e.g., 1.0.0)
# 2. Pre-release checklist confirmation
# 3. Push to remote confirmation
```

**Result**: Binaries for Linux, macOS (Intel & ARM), Windows published to GitHub Releases

## Integration Release (Python & TypeScript)

```bash
# All integrations at once
./scripts/release-integrations.sh 1.0.0

# What you'll be asked:
# 1. Which integrations to release
# 2. Confirmation
# 3. Push to remote confirmation
```

**Result**: Packages published to PyPI and npm

## Just Bump Version (No Release)

```bash
# Interactive version bump
./scripts/bump-version.sh

# What you'll be asked:
# 1. Bump type (major/minor/patch/custom)
# 2. What to update (core/integrations/all)
# 3. Confirmation
```

**Result**: Version files updated (not committed)

## Manual Workflows

```bash
# Core release
gh workflow run release.yml -f tag=v1.0.0

# LangChain (to PyPI)
gh workflow run publish-langchain.yml -f version=1.0.0

# LlamaIndex (to PyPI)
gh workflow run publish-llamaindex.yml -f version=1.0.0

# TypeScript (to npm)
gh workflow run publish-npm.yml -f version=2.0.0

# Generate changelog
gh workflow run changelog.yml -f version=1.0.0
```

## Testing Releases

```bash
# PyPI test releases
gh workflow run publish-langchain.yml -f version=1.0.0 -f test_pypi=true
gh workflow run publish-llamaindex.yml -f version=1.0.0 -f test_pypi=true

# npm dry run
gh workflow run publish-npm.yml -f version=2.0.0 -f test_registry=true
```

## Complete Release Flow

```bash
# 1. Ensure everything is ready
git checkout master
git pull origin master
cargo test --all

# 2. Bump version (optional)
./scripts/bump-version.sh

# 3. Core release
./scripts/release.sh
# Enter version when prompted (e.g., 1.0.0)
# Confirm checklist
# Confirm push

# 4. Wait for workflow
# Monitor: https://github.com/avocadodb/avocadodb/actions
# Wait for all 4 binary builds to complete

# 5. Release integrations
./scripts/release-integrations.sh 1.0.0
# Select "4. All of the above"
# Confirm push

# 6. Generate changelog (optional)
gh workflow run changelog.yml -f version=1.0.0
# Review and merge the PR it creates

# 7. Verify releases
# - GitHub: https://github.com/avocadodb/avocadodb/releases
# - PyPI: https://pypi.org/project/langchain-avocadodb/
# - PyPI: https://pypi.org/project/llama-index-avocadodb/
# - npm: https://www.npmjs.com/package/avocadodb

# 8. Test installations
pip install langchain-avocadodb==1.0.0
pip install llama-index-avocadodb==1.0.0
npm install avocadodb@2.0.0
```

## Rollback

```bash
# Delete tag locally and remotely
git tag -d v1.0.0
git push origin :refs/tags/v1.0.0

# Delete GitHub release
gh release delete v1.0.0 --yes

# For PyPI/npm: Release patch version with fix
```

## Troubleshooting

```bash
# Check script syntax
bash -n scripts/release.sh

# Make scripts executable
chmod +x scripts/*.sh

# View workflow runs
gh run list --workflow=release.yml

# View workflow logs
gh run view <run-id> --log
```

## File Locations

- Workflows: `.github/workflows/`
- Scripts: `scripts/`
- Docs: `docs/RELEASING.md`
- Templates: `.github/RELEASE_TEMPLATE.md`
- Version: `version.txt`
- Changelog: `CHANGELOG.md`

## Version Tags

- Core: `v1.0.0`
- LangChain: `langchain-v1.0.0`
- LlamaIndex: `llamaindex-v1.0.0`
- TypeScript: `npm-v2.0.0`

## Need Help?

- Full guide: `docs/RELEASING.md`
- Implementation report: `.github/RELEASE_AUTOMATION_REPORT.md`
- Issues: https://github.com/avocadodb/avocadodb/issues

---

Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>
