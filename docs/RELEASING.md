# Release Guide

This document describes the release process for AvocadoDB and its integrations.

## Table of Contents

- [Overview](#overview)
- [Version Numbering](#version-numbering)
- [Pre-Release Checklist](#pre-release-checklist)
- [Release Process](#release-process)
  - [Core Release](#core-release)
  - [Integration Releases](#integration-releases)
- [Manual Release Steps](#manual-release-steps)
- [Secrets Configuration](#secrets-configuration)
- [Testing Releases](#testing-releases)
- [Post-Release Tasks](#post-release-tasks)
- [Rollback Procedures](#rollback-procedures)
- [Troubleshooting](#troubleshooting)

## Overview

AvocadoDB uses automated GitHub Actions workflows to handle releases across multiple distribution channels:

- **Binary Releases**: GitHub Releases with binaries for Linux, macOS (Intel & ARM), and Windows
- **PyPI Packages**: `langchain-avocadodb` and `llama-index-avocadodb` Python packages
- **npm Package**: `avocadodb` TypeScript/JavaScript SDK
- **Docker Images**: Automated builds via separate Docker workflow

## Version Numbering

We follow [Semantic Versioning (semver)](https://semver.org/):

```
MAJOR.MINOR.PATCH[-PRERELEASE]
```

- **MAJOR**: Incompatible API changes
- **MINOR**: New functionality, backward compatible
- **PATCH**: Bug fixes, backward compatible
- **PRERELEASE**: Optional pre-release identifier (alpha, beta, rc)

### Examples

- `1.0.0` - Major release
- `1.1.0` - Minor release with new features
- `1.1.1` - Patch release with bug fixes
- `2.0.0-beta.1` - Pre-release version
- `2.0.0-rc.1` - Release candidate

### Tag Naming Conventions

Different components use different tag prefixes:

- **Core/Binaries**: `v1.0.0`
- **LangChain**: `langchain-v1.0.0`
- **LlamaIndex**: `llamaindex-v1.0.0`
- **TypeScript/npm**: `npm-v1.0.0`

## Pre-Release Checklist

Before creating a release, ensure:

- [ ] All tests are passing on CI
- [ ] Documentation is up to date
- [ ] CHANGELOG.md is updated (or will be generated)
- [ ] Breaking changes are documented
- [ ] Examples are tested and working
- [ ] Security vulnerabilities are addressed
- [ ] Dependencies are up to date
- [ ] No known critical bugs
- [ ] Code review is complete
- [ ] Branch is up to date with master

## Release Process

### Core Release

The core release includes the Rust binary (CLI and server) and creates a GitHub Release.

#### Using the Release Script (Recommended)

```bash
# Interactive release
./scripts/release.sh
```

The script will:
1. Check for uncommitted changes
2. Prompt for new version
3. Show pre-release checklist
4. Update version.txt and Cargo.toml
5. Commit changes
6. Create and push git tag
7. Trigger release workflow

#### Manual Steps

```bash
# 1. Update version
echo "1.0.0" > version.txt
sed -i '' 's/^version = .*/version = "1.0.0"/' Cargo.toml

# 2. Commit changes
git add version.txt Cargo.toml
git commit -m "chore: Bump version to 1.0.0"

# 3. Create tag
git tag -a v1.0.0 -m "Release v1.0.0"

# 4. Push
git push origin master
git push origin v1.0.0
```

The `v*.*.*` tag push will trigger the `.github/workflows/release.yml` workflow, which:
- Builds binaries for all platforms
- Creates release archives with checksums
- Generates release notes from commits
- Creates GitHub Release
- Uploads all assets

### Integration Releases

Release Python and TypeScript integrations separately from the core.

#### Using the Integration Script (Recommended)

```bash
# Interactive release
./scripts/release-integrations.sh

# Or specify version
./scripts/release-integrations.sh 1.0.0
```

The script will:
1. Prompt for which integrations to release
2. Update respective version files
3. Create appropriate tags
4. Push to trigger workflows

#### Manual LangChain Release

```bash
# Update version
cd integrations/langchain-avocadodb
poetry version 1.0.0

# Commit and tag
git add pyproject.toml
git commit -m "chore(langchain): Bump version to 1.0.0"
git tag -a langchain-v1.0.0 -m "Release langchain-avocadodb v1.0.0"
git push origin master
git push origin langchain-v1.0.0
```

#### Manual LlamaIndex Release

```bash
# Update version
cd integrations/llama-index-avocadodb
poetry version 1.0.0

# Commit and tag
git add pyproject.toml
git commit -m "chore(llamaindex): Bump version to 1.0.0"
git tag -a llamaindex-v1.0.0 -m "Release llama-index-avocadodb v1.0.0"
git push origin master
git push origin llamaindex-v1.0.0
```

#### Manual TypeScript Release

```bash
# Update version
cd sdks/typescript
npm version 1.0.0 --no-git-tag-version

# Commit and tag
git add package.json
git commit -m "chore(typescript): Bump version to 1.0.0"
git tag -a npm-v1.0.0 -m "Release avocadodb (npm) v1.0.0"
git push origin master
git push origin npm-v1.0.0
```

## Manual Release Steps

### Triggering Workflows Manually

All release workflows support manual dispatch for testing:

```bash
# Core release
gh workflow run release.yml -f tag=v1.0.0

# LangChain
gh workflow run publish-langchain.yml -f version=1.0.0 -f test_pypi=false

# LlamaIndex
gh workflow run publish-llamaindex.yml -f version=1.0.0 -f test_pypi=false

# TypeScript
gh workflow run publish-npm.yml -f version=2.0.0 -f test_registry=false
```

### Generating Changelog

```bash
# Generate changelog for version
gh workflow run changelog.yml -f version=1.0.0

# Specify tag range
gh workflow run changelog.yml -f version=1.0.0 -f from_tag=v0.9.0 -f to_tag=v1.0.0
```

This creates a PR with the updated CHANGELOG.md.

## Secrets Configuration

The following GitHub secrets must be configured in your repository:

### PyPI Publishing

**Secret Name**: `PYPI_TOKEN`

**How to obtain**:
1. Go to https://pypi.org/manage/account/
2. Scroll to "API tokens"
3. Click "Add API token"
4. Name: "AvocadoDB GitHub Actions"
5. Scope: Select specific projects or "Entire account"
6. Copy the token (starts with `pypi-`)
7. Add to GitHub: Settings > Secrets > Actions > New repository secret

**Test PyPI** (optional):

**Secret Name**: `TEST_PYPI_TOKEN`

1. Go to https://test.pypi.org/manage/account/
2. Follow same steps as above

### npm Publishing

**Secret Name**: `NPM_TOKEN`

**How to obtain**:
1. Log in to https://www.npmjs.com/
2. Click on your profile > Access Tokens
3. Click "Generate New Token" > "Classic Token"
4. Select "Automation" type
5. Copy the token
6. Add to GitHub secrets

### GitHub Token

**Secret Name**: `GITHUB_TOKEN`

This is automatically provided by GitHub Actions. No configuration needed.

## Testing Releases

### Dry Run Releases

Test releases without publishing:

```bash
# PyPI dry run (Test PyPI)
gh workflow run publish-langchain.yml -f version=1.0.0 -f test_pypi=true
gh workflow run publish-llamaindex.yml -f version=1.0.0 -f test_pypi=true

# npm dry run
gh workflow run publish-npm.yml -f version=2.0.0 -f test_registry=true
```

### Local Testing

#### Test Binary Builds

```bash
# Build for current platform
cargo build --release --bin avocado

# Test the binary
./target/release/avocado --version
./target/release/avocado --help
```

#### Test PyPI Packages

```bash
# LangChain
cd integrations/langchain-avocadodb
poetry install
poetry run pytest
poetry build
twine check dist/*

# LlamaIndex
cd integrations/llama-index-avocadodb
poetry install
poetry run pytest
poetry build
twine check dist/*
```

#### Test npm Package

```bash
cd sdks/typescript
npm ci
npm run build
npm pack --dry-run
```

## Post-Release Tasks

After a successful release:

1. **Verify Releases**
   - Check GitHub Release page
   - Verify binary downloads work
   - Test PyPI packages: `pip install langchain-avocadodb==X.Y.Z`
   - Test npm package: `npm install avocadodb@X.Y.Z`

2. **Update Documentation**
   - Update docs site with new version
   - Update getting started guides
   - Update example code

3. **Announce Release**
   - Blog post (if major release)
   - Twitter/social media
   - Discord/community channels
   - Email newsletter

4. **Monitor**
   - Watch for bug reports
   - Monitor download statistics
   - Track issue tracker

5. **Tag Dependencies**
   - Update dependent projects
   - Notify integration users

## Rollback Procedures

### If Release Workflow Fails

1. **Cancel the workflow** (if still running)
2. **Delete the tag** locally and remotely:
   ```bash
   git tag -d v1.0.0
   git push origin :refs/tags/v1.0.0
   ```
3. **Fix the issue** in the code or workflow
4. **Try again** with the same or new version

### If Package is Published with Issues

#### PyPI

You **cannot delete** a version from PyPI, but you can:

1. **Yank the release**:
   ```bash
   # Using twine
   twine upload --skip-existing --repository-url https://pypi.org dist/*

   # Or via PyPI web interface
   # Go to package page > Manage > Options > Yank
   ```

2. **Release a patch version** with fixes:
   ```bash
   # Release 1.0.1 with fixes
   ./scripts/release-integrations.sh 1.0.1
   ```

#### npm

You can unpublish within 72 hours:

```bash
# Unpublish specific version
npm unpublish avocadodb@1.0.0

# Or deprecate
npm deprecate avocadodb@1.0.0 "This version has issues, please use 1.0.1"
```

#### GitHub Release

Delete the release and tag:

1. Go to Releases page
2. Click on the release
3. Click "Delete" (you may need to delete assets first)
4. Delete the tag:
   ```bash
   git tag -d v1.0.0
   git push origin :refs/tags/v1.0.0
   ```

## Troubleshooting

### Common Issues

#### "Tag already exists"

```bash
# Delete and recreate tag
git tag -d v1.0.0
git push origin :refs/tags/v1.0.0
git tag -a v1.0.0 -m "Release v1.0.0"
git push origin v1.0.0
```

#### "cargo build failed"

- Check Cargo.lock is committed
- Verify all dependencies are available
- Check for platform-specific issues
- Review build logs in GitHub Actions

#### "PyPI authentication failed"

- Verify `PYPI_TOKEN` secret is set correctly
- Ensure token has correct permissions
- Check token hasn't expired
- Try regenerating the token

#### "npm publish failed"

- Verify `NPM_TOKEN` secret is correct
- Check package name isn't taken
- Ensure version doesn't already exist
- Verify package.json is valid

#### "Tests failed during release"

- Don't proceed with release
- Fix failing tests first
- Run tests locally: `cargo test --all`
- Check CI logs for details

#### "Permission denied" on scripts

```bash
chmod +x scripts/*.sh
```

### Getting Help

- **Issues**: https://github.com/avocadodb/avocadodb/issues
- **Discussions**: https://github.com/avocadodb/avocadodb/discussions
- **Discord**: [Link to Discord if available]

## Release Checklist Template

Use this checklist for each release:

```markdown
## Release v1.0.0 Checklist

### Pre-Release
- [ ] All CI checks passing
- [ ] CHANGELOG.md updated
- [ ] Documentation reviewed
- [ ] Examples tested
- [ ] Breaking changes documented
- [ ] Version numbers updated

### Release
- [ ] Core release (v1.0.0 tag pushed)
- [ ] Binaries built successfully
- [ ] GitHub Release created
- [ ] LangChain released to PyPI
- [ ] LlamaIndex released to PyPI
- [ ] TypeScript SDK released to npm
- [ ] Docker image updated

### Post-Release
- [ ] Releases verified
- [ ] Documentation updated
- [ ] Announcement prepared
- [ ] Social media posts
- [ ] Community notified
- [ ] Monitoring in place

### Issues
- List any issues encountered during release
```

---

For questions or issues with the release process, please open an issue or discussion on GitHub.
