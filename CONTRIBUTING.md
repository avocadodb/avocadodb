# Contributing to AvocadoDB

Thank you for your interest in contributing to AvocadoDB! We're building a high-performance, lightweight vector database in Rust, and we welcome contributions from developers of all skill levels.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Setup](#development-setup)
- [Running Tests](#running-tests)
- [Submitting Changes](#submitting-changes)
- [Code Style Guidelines](#code-style-guidelines)
- [Commit Message Conventions](#commit-message-conventions)
- [Getting Help](#getting-help)

## Code of Conduct

This project adheres to the Contributor Covenant [Code of Conduct](CODE_OF_CONDUCT.md). By participating, you are expected to uphold this code. Please report unacceptable behavior to the project maintainers.

## Getting Started

1. **Fork the repository** on GitHub
2. **Clone your fork** locally:
   ```bash
   git clone https://github.com/YOUR_USERNAME/avocadodb.git
   cd avocadodb
   ```
3. **Add upstream remote**:
   ```bash
   git remote add upstream https://github.com/avocadodb/avocadodb.git
   ```
4. **Create a feature branch**:
   ```bash
   git checkout -b feature/your-feature-name
   ```

## Development Setup

### Prerequisites

- **Rust**: Install via [rustup](https://rustup.rs/) (latest stable version)
- **Python**: 3.8+ (for Python bindings and testing)
- **Node.js**: 16+ (for TypeScript/JavaScript bindings)
- **Cargo**: Comes with Rust installation

### Building AvocadoDB

```bash
# Build the entire project
cargo build

# Build in release mode
cargo build --release

# Build specific components
cargo build -p avocado-cli
```

### Setting Up Development Environment

```bash
# Install development dependencies
cargo install cargo-watch cargo-tarpaulin

# For Python development
pip install -e ./python

# For TypeScript development
cd typescript
npm install
npm run build
```

## Running Tests

We provide a single, CI-safe entrypoint that must stay green on every PR. It uses local embeddings only (no network or API keys) and requires no running server.

### Quick (CI-safe) run

```bash
./scripts/run-tests.sh
```

This runs:
- avocado-core unit tests
- Determinism tests (local embeddings, repeated runs; serial execution)

Exit code is non-zero on failure.

### Full Rust test matrix (optional)

```bash
# All workspace tests (may include ignored tests)
cargo test

# Server HTTP API tests (require a running server and are ignored by default)
# 1) In one shell:
#    PORT=8765 BIND_ADDR=127.0.0.1 cargo run -p avocado-server
# 2) In another shell:
cargo test -p avocado-server --test session_api_tests -- --ignored --test-threads=1

# Benchmarks (optional)
cargo bench
```

### Python Tests (optional)

```bash
cd python
pytest tests/
pytest tests/ -v  # verbose output
```

### TypeScript Tests (optional)

```bash
cd typescript
npm test
npm run test:coverage
```

### Performance Benchmarks

```bash
# Run CLI benchmarks
cargo bench -p avocado-cli

# Run core benchmarks
cargo bench -p avocado-core
```

## Submitting Changes

### Pull Request Process

1. **Sync with upstream**:
   ```bash
   git fetch upstream
   git rebase upstream/master
   ```

2. **Make your changes** and commit them following our [commit conventions](#commit-message-conventions)

3. **Run tests and linters**:
   ```bash
   cargo test
   cargo fmt --check
   cargo clippy -- -D warnings
   ```

4. **Push to your fork**:
   ```bash
   git push origin feature/your-feature-name
   ```

5. **Create a Pull Request** on GitHub with:
   - Clear description of changes
   - Reference to related issues
   - Test results
   - Documentation updates (if applicable)

### PR Requirements

- All tests must pass
- Code coverage should not decrease
- Code must be formatted according to style guidelines
- Documentation must be updated for new features
- Commit messages must follow conventions
- No merge conflicts with `master` branch

## Code Style Guidelines

### Rust

- Use `rustfmt` for code formatting:
  ```bash
  cargo fmt
  ```
- Follow Rust API guidelines: https://rust-lang.github.io/api-guidelines/
- Run `clippy` and address all warnings:
  ```bash
  cargo clippy -- -D warnings
  ```
- Write idiomatic Rust code
- Add documentation comments (`///`) for public APIs
- Keep functions focused and testable

### Python

- Use `black` for code formatting:
  ```bash
  black python/
  ```
- Follow PEP 8 style guide
- Use type hints for function signatures
- Write docstrings for all public functions
- Use `mypy` for type checking:
  ```bash
  mypy python/
  ```

### TypeScript

- Use `prettier` for code formatting:
  ```bash
  npm run format
  ```
- Use ESLint:
  ```bash
  npm run lint
  ```
- Enable strict mode in `tsconfig.json`
- Write JSDoc comments for public APIs
- Use TypeScript's type system effectively

### General Guidelines

- Write clear, self-documenting code
- Add comments for complex logic
- Keep line length under 100 characters
- Use meaningful variable and function names
- Avoid premature optimization
- Write tests for new features and bug fixes

## Commit Message Conventions

We follow the [Conventional Commits](https://www.conventionalcommits.org/) specification:

```
<type>(<scope>): <subject>

<body>

<footer>
```

### Types

- **feat**: New feature
- **fix**: Bug fix
- **docs**: Documentation changes
- **style**: Code style changes (formatting, no logic change)
- **refactor**: Code refactoring
- **perf**: Performance improvements
- **test**: Adding or updating tests
- **chore**: Maintenance tasks, dependency updates
- **ci**: CI/CD changes

### Examples

```
feat(core): Add support for cosine similarity search

Implement cosine similarity distance metric for vector searches.
Includes optimized SIMD operations for better performance.

Closes #123
```

```
fix(cli): Resolve panic on empty query vector

Add validation to ensure query vectors are not empty before
processing. Return clear error message to user.

Fixes #456
```

```
docs(readme): Update installation instructions

Add Homebrew installation option and clarify build requirements.
```

### Commit Best Practices

- Use present tense ("Add feature" not "Added feature")
- Use imperative mood ("Move cursor to..." not "Moves cursor to...")
- First line should be 50 characters or less
- Reference issues and pull requests when relevant
- Explain **why** not just **what** in the body

## Getting Help

- **GitHub Discussions**: Ask questions, share ideas, or discuss features
- **GitHub Issues**: Report bugs or request features (use templates)
- **Documentation**: Check our [docs/](docs/) folder for guides
- **Examples**: See [examples/](examples/) for usage patterns

### Before Asking

1. Search existing issues and discussions
2. Check the documentation
3. Review closed issues for similar problems
4. Try the latest version from `master`

## Development Tips

### Useful Commands

```bash
# Watch and auto-rebuild on changes
cargo watch -x build

# Run tests on file changes
cargo watch -x test

# Generate and open documentation
cargo doc --open --no-deps

# Check for outdated dependencies
cargo outdated

# Update dependencies
cargo update
```

### Project Structure

```
avocadodb/
├── avocado-core/       # Core vector database implementation
├── avocado-cli/        # Command-line interface
├── avocado-server/     # HTTP/gRPC server
├── python/             # Python bindings
├── typescript/         # TypeScript bindings
├── docs/               # Documentation
├── examples/           # Example applications
└── tests/              # Integration tests
```

### Performance Considerations

- Profile before optimizing (`cargo flamegraph`)
- Write benchmarks for performance-critical code
- Consider memory allocations in hot paths
- Use SIMD when appropriate for vector operations
- Test with realistic dataset sizes

## Recognition

Contributors are recognized in:
- GitHub's contributor graph
- Release notes
- Special acknowledgments for significant contributions

## License

By contributing to AvocadoDB, you agree that your contributions will be licensed under the [MIT License](LICENSE).

---

Thank you for contributing to AvocadoDB! Your efforts help make vector databases more accessible and performant for everyone.
