# {PACKAGE_NAME} v{VERSION}

<!-- Integration-specific release notes -->

## Overview

{PACKAGE_NAME} v{VERSION} - Integration with AvocadoDB for {FRAMEWORK_NAME}.

This release provides {brief description of what this integration enables}.

## What's New

### Features

<!-- List new features -->
- New feature 1
- New feature 2

### Bug Fixes

<!-- List bug fixes -->
- Fixed issue with...
- Resolved problem where...

### Improvements

<!-- List improvements -->
- Better error handling
- Improved documentation
- Performance optimizations

### Breaking Changes

<!-- List any breaking changes -->
<!-- If none, remove this section -->

## Installation

### PyPI

```bash
pip install {PACKAGE_NAME}=={VERSION}
```

### With specific dependencies

```bash
# For specific Python version
pip install {PACKAGE_NAME}=={VERSION} --python-version 3.11

# With extras (if available)
pip install {PACKAGE_NAME}[dev]=={VERSION}
```

## Quick Start

### Basic Usage

```python
from {PACKAGE_IMPORT} import {CLASS_NAME}

# Initialize
store = {CLASS_NAME}(
    base_url="http://localhost:8765",
    collection_name="my_collection"
)

# Use with {FRAMEWORK_NAME}
# ... framework-specific code ...
```

### With AvocadoDB Server

Make sure AvocadoDB server is running:

```bash
# Start server
avocado server start

# Or with Docker
docker run -p 8765:8765 ghcr.io/avocadodb/avocadodb:latest
```

## Features

### Deterministic Context Retrieval

{PACKAGE_NAME} provides deterministic, citation-backed retrieval:

- Consistent results across queries
- Source attribution with citations
- Document provenance tracking
- Session management support

### Session Management

Track conversation history and context:

```python
# Create a session
session = store.create_session(
    session_id="my-conversation",
    metadata={"user": "example"}
)

# Query with session context
results = store.query(
    query="What did we discuss about X?",
    session_id="my-conversation"
)
```

### Framework Integration

Seamlessly integrates with {FRAMEWORK_NAME}:

- Native {FRAMEWORK_NAME} interfaces
- Type-safe operations
- Async support (where applicable)
- Full compatibility with {FRAMEWORK_NAME} ecosystem

## Compatibility

- **Python**: 3.9, 3.10, 3.11, 3.12
- **{FRAMEWORK_NAME}**: {VERSION_RANGE}
- **AvocadoDB**: v{MIN_VERSION}+

## Migration Guide

### From v{PREVIOUS_VERSION}

<!-- Provide migration steps if needed -->

```python
# Before
# ... old code ...

# After
# ... new code ...
```

<!-- If no migration needed -->
No breaking changes. Drop-in replacement for v{PREVIOUS_VERSION}.

## Examples

### Example 1: Basic Retrieval

```python
from {PACKAGE_IMPORT} import {CLASS_NAME}

# Initialize
store = {CLASS_NAME}(
    base_url="http://localhost:8765",
    collection_name="docs"
)

# Add documents
docs = [...]  # Your documents
store.add_documents(docs)

# Retrieve
results = store.similarity_search(
    query="example query",
    k=5
)

for result in results:
    print(f"Content: {result.page_content}")
    print(f"Source: {result.metadata['source']}")
```

### Example 2: With Session

```python
# Create session-aware retrieval
session_id = store.create_session()

# Query with context
results = store.similarity_search(
    query="Follow-up question",
    k=5,
    session_id=session_id
)
```

### Example 3: Advanced Usage

```python
# Custom filtering and metadata
results = store.similarity_search(
    query="specific query",
    k=10,
    filter={"category": "technical"},
    include_metadata=True
)
```

## Documentation

- [Integration Guide](https://docs.avocadodb.com/integrations/{INTEGRATION_NAME})
- [API Reference](https://docs.avocadodb.com/api/python/{PACKAGE_NAME})
- [Examples](https://github.com/avocadodb/avocadodb/tree/master/integrations/{PACKAGE_DIR}/examples)
- [{FRAMEWORK_NAME} Documentation]({FRAMEWORK_DOCS_URL})

## Resources

### Code Examples

Find complete examples in the repository:
- [Basic Usage](https://github.com/avocadodb/avocadodb/tree/master/integrations/{PACKAGE_DIR}/examples/basic)
- [Session Management](https://github.com/avocadodb/avocadodb/tree/master/integrations/{PACKAGE_DIR}/examples/sessions)
- [Advanced Patterns](https://github.com/avocadodb/avocadodb/tree/master/integrations/{PACKAGE_DIR}/examples/advanced)

### Notebooks

Interactive Jupyter notebooks:
- [Getting Started Notebook](https://github.com/avocadodb/avocadodb/tree/master/integrations/{PACKAGE_DIR}/notebooks/getting-started.ipynb)
- [Advanced Features](https://github.com/avocadodb/avocadodb/tree/master/integrations/{PACKAGE_DIR}/notebooks/advanced.ipynb)

## Troubleshooting

### Common Issues

#### Connection Errors

```python
# Ensure server is running
# Check the base_url is correct
store = {CLASS_NAME}(
    base_url="http://localhost:8765",  # Verify port
    timeout=30  # Increase timeout if needed
)
```

#### Import Errors

```bash
# Ensure package is installed
pip install {PACKAGE_NAME}=={VERSION}

# Check Python version
python --version  # Should be 3.9+
```

#### Version Conflicts

```bash
# Upgrade all dependencies
pip install --upgrade {PACKAGE_NAME} {FRAMEWORK_NAME}
```

## Development

### Setup Development Environment

```bash
# Clone repository
git clone https://github.com/avocadodb/avocadodb.git
cd avocadodb/integrations/{PACKAGE_DIR}

# Install with Poetry
poetry install --with dev,test

# Run tests
poetry run pytest

# Run linting
poetry run black src/
poetry run ruff check src/
```

### Running Tests

```bash
# All tests
poetry run pytest

# Specific test file
poetry run pytest tests/test_vector_store.py

# With coverage
poetry run pytest --cov={PACKAGE_IMPORT}
```

## Contributing

We welcome contributions! See [CONTRIBUTING.md](https://github.com/avocadodb/avocadodb/blob/master/CONTRIBUTING.md) for guidelines.

## Support

- **Issues**: [GitHub Issues](https://github.com/avocadodb/avocadodb/issues)
- **Discussions**: [GitHub Discussions](https://github.com/avocadodb/avocadodb/discussions)
- **Documentation**: [docs.avocadodb.com](https://docs.avocadodb.com)

## License

MIT License - See [LICENSE](https://github.com/avocadodb/avocadodb/blob/master/LICENSE) for details.

## Changelog

See [CHANGELOG.md](https://github.com/avocadodb/avocadodb/blob/master/integrations/{PACKAGE_DIR}/CHANGELOG.md) for complete version history.

---

**Package**: `{PACKAGE_NAME}`
**Version**: `{VERSION}`
**Release Date**: `{RELEASE_DATE}`

---

Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>
