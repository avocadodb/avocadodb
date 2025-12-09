# AvocadoDB v{VERSION}

<!-- Brief description of this release -->

## What's New in v{VERSION}

### Features

<!-- List new features -->
- **Feature Name**: Description of the new feature
- **Another Feature**: Description

### Bug Fixes

<!-- List bug fixes -->
- Fixed issue with...
- Resolved problem where...

### Performance Improvements

<!-- List performance improvements -->
- Improved query performance by X%
- Reduced memory usage in...

### Documentation

<!-- List documentation improvements -->
- Added guide for...
- Updated examples to...

### Maintenance

<!-- List internal improvements, dependency updates, etc. -->
- Updated dependencies
- Improved test coverage
- Refactored...

### Breaking Changes

<!-- List any breaking changes -->
- **BREAKING**: Description of breaking change
- **Migration Guide**: Steps to migrate from previous version

<!-- If no breaking changes, remove this section -->

## Installation

### Binary Releases

Download the appropriate binary for your platform:

#### Linux (x86_64)
```bash
wget https://github.com/avocadodb/avocadodb/releases/download/v{VERSION}/avocado-cli-{VERSION}-linux-x86_64.tar.gz
tar -xzf avocado-cli-{VERSION}-linux-x86_64.tar.gz
cd avocado-cli-{VERSION}-linux-x86_64
sudo mv avocado /usr/local/bin/
```

#### macOS (Intel)
```bash
wget https://github.com/avocadodb/avocadodb/releases/download/v{VERSION}/avocado-cli-{VERSION}-macos-x86_64.tar.gz
tar -xzf avocado-cli-{VERSION}-macos-x86_64.tar.gz
cd avocado-cli-{VERSION}-macos-x86_64
sudo mv avocado /usr/local/bin/
```

#### macOS (Apple Silicon)
```bash
wget https://github.com/avocadodb/avocadodb/releases/download/v{VERSION}/avocado-cli-{VERSION}-macos-aarch64.tar.gz
tar -xzf avocado-cli-{VERSION}-macos-aarch64.tar.gz
cd avocado-cli-{VERSION}-macos-aarch64
sudo mv avocado /usr/local/bin/
```

#### Windows (x86_64)
Download `avocado-cli-{VERSION}-windows-x86_64.zip` and extract to a directory in your PATH.

### Verify Checksums

```bash
# Download checksum file
wget https://github.com/avocadodb/avocadodb/releases/download/v{VERSION}/checksums.txt

# Verify (Linux/macOS)
sha256sum -c checksums.txt
```

### Docker

```bash
docker pull ghcr.io/avocadodb/avocadodb:{VERSION}
```

Run the server:

```bash
docker run -p 8765:8765 ghcr.io/avocadodb/avocadodb:{VERSION}
```

### Python Integrations

#### LangChain

```bash
pip install langchain-avocadodb=={VERSION}
```

```python
from langchain_avocadodb import AvocadoDBVectorStore

# Use with LangChain
vector_store = AvocadoDBVectorStore(
    base_url="http://localhost:8765",
    collection_name="my_docs"
)
```

#### LlamaIndex

```bash
pip install llama-index-avocadodb=={VERSION}
```

```python
from llama_index_avocadodb import AvocadoDBVectorStore

# Use with LlamaIndex
vector_store = AvocadoDBVectorStore(
    base_url="http://localhost:8765",
    collection_name="my_docs"
)
```

### TypeScript/JavaScript SDK

#### npm

```bash
npm install avocadodb@{VERSION}
```

#### yarn

```bash
yarn add avocadodb@{VERSION}
```

#### pnpm

```bash
pnpm add avocadodb@{VERSION}
```

```typescript
import { AvocadoDBClient } from 'avocadodb';

const client = new AvocadoDBClient({
  baseUrl: 'http://localhost:8765'
});
```

## Quick Start

1. **Start the server**:
   ```bash
   avocado server start
   ```

2. **Create a collection**:
   ```bash
   avocado collection create my_docs
   ```

3. **Add documents**:
   ```bash
   avocado add my_docs ./documents/
   ```

4. **Query with context**:
   ```bash
   avocado query my_docs "What are the key features?"
   ```

## Documentation

- [Getting Started Guide](https://docs.avocadodb.com/getting-started)
- [API Reference](https://docs.avocadodb.com/api)
- [Integration Guides](https://docs.avocadodb.com/integrations)
- [Examples](https://github.com/avocadodb/avocadodb/tree/master/examples)

## Upgrade Guide

### From v{PREVIOUS_VERSION}

<!-- Provide specific upgrade instructions if needed -->

1. Stop the existing server
2. Replace the binary with the new version
3. Review breaking changes (if any)
4. Restart the server
5. Update client libraries

```bash
# Update Python packages
pip install --upgrade langchain-avocadodb llama-index-avocadodb

# Update TypeScript SDK
npm update avocadodb
```

<!-- If no specific upgrade steps needed, just say: -->
<!-- No special upgrade steps required. Simply replace the binary and restart. -->

## Known Issues

<!-- List any known issues with workarounds -->
<!-- If none, remove this section or say: -->
No known issues at this time.

## Contributors

Thank you to all contributors who made this release possible!

<!-- Auto-generated or manually list contributors -->

## Full Changelog

See [CHANGELOG.md](https://github.com/avocadodb/avocadodb/blob/master/CHANGELOG.md) for complete details.

**Full Commit History**: https://github.com/avocadodb/avocadodb/compare/v{PREVIOUS_VERSION}...v{VERSION}

---

Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>
