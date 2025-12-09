## Description

<!-- Provide a clear and concise description of what this PR does -->

### Summary of Changes

<!-- List the key changes in bullet points -->
-
-
-

### Motivation and Context

<!-- Why is this change needed? What problem does it solve? -->
<!-- If it fixes an open issue, please link to the issue here -->

Fixes #(issue)

## Type of Change

<!-- Mark the relevant option with an "x" -->

- [ ] Bug fix (non-breaking change which fixes an issue)
- [ ] New feature (non-breaking change which adds functionality)
- [ ] Breaking change (fix or feature that would cause existing functionality to not work as expected)
- [ ] Performance improvement
- [ ] Code refactoring (no functional changes)
- [ ] Documentation update
- [ ] Build/CI configuration change
- [ ] Dependency update
- [ ] Other (please describe):

## Component

<!-- Which component(s) does this PR affect? -->

- [ ] Core (`avocado-core`)
- [ ] CLI (`avocado-cli`)
- [ ] Server (`avocado-server`)
- [ ] Python bindings
- [ ] TypeScript bindings
- [ ] Documentation
- [ ] CI/CD
- [ ] Other:

## Testing

### Test Coverage

- [ ] Unit tests added/updated
- [ ] Integration tests added/updated
- [ ] Benchmarks added/updated (if performance-related)
- [ ] Manual testing performed

### Test Results

```bash
# Paste relevant test output here
cargo test
```

### Testing Checklist

- [ ] All existing tests pass
- [ ] New tests cover the changes
- [ ] Edge cases are tested
- [ ] Error handling is tested
- [ ] No test coverage regression

## Performance Impact

<!-- If applicable, describe any performance implications -->

- [ ] No performance impact
- [ ] Performance improvement (include benchmarks)
- [ ] Potential performance regression (explain why acceptable)

### Benchmark Results (if applicable)

```
# Paste benchmark results here
cargo bench
```

## Documentation

- [ ] Code is self-documenting with clear variable/function names
- [ ] Added/updated inline code comments for complex logic
- [ ] Added/updated rustdoc comments for public APIs
- [ ] Updated README.md (if applicable)
- [ ] Updated relevant documentation in `docs/` folder
- [ ] Added/updated examples (if new feature)
- [ ] Updated CHANGELOG.md (if applicable)

## Breaking Changes

<!-- If this PR includes breaking changes, describe them here -->

### Breaking Change Description

<!-- What breaks? How should users migrate? -->

- [ ] No breaking changes
- [ ] Breaking changes described below

<!-- If breaking changes exist, provide:
1. What changed
2. Why it was necessary
3. Migration guide for users
-->

## Checklist

<!-- Mark items with an "x" when completed -->

### Code Quality

- [ ] Code follows the project's style guidelines
- [ ] `cargo fmt` has been run
- [ ] `cargo clippy` passes with no warnings
- [ ] No compiler warnings
- [ ] Code is DRY (Don't Repeat Yourself)
- [ ] Functions are focused and single-purpose
- [ ] Error handling is appropriate

### Python (if applicable)

- [ ] Code formatted with `black`
- [ ] Type hints added
- [ ] `mypy` type checking passes
- [ ] Docstrings added for public APIs

### TypeScript (if applicable)

- [ ] Code formatted with `prettier`
- [ ] `eslint` passes
- [ ] Type definitions updated
- [ ] JSDoc comments added

### Review Readiness

- [ ] Self-reviewed my own code
- [ ] Removed debug logs and commented-out code
- [ ] No merge conflicts with `master`
- [ ] PR title follows conventional commit format
- [ ] Branch is up to date with `master`

### Security

- [ ] No hardcoded secrets or credentials
- [ ] No SQL injection vulnerabilities (if applicable)
- [ ] Input validation added where needed
- [ ] No unsafe code added (or justified if necessary)

## Related Issues and PRs

<!-- Link to related issues, PRs, or discussions -->

- Related to #
- Depends on #
- Blocked by #

## Screenshots (if applicable)

<!-- Add screenshots for UI changes or visual improvements -->

## Additional Notes

<!-- Any additional information that reviewers should know -->

## Deployment Notes

<!-- Special considerations for deployment (if any) -->

- [ ] No special deployment steps required
- [ ] Requires database migration
- [ ] Requires configuration changes
- [ ] Requires dependency updates

---

## For Reviewers

### Focus Areas

<!-- What should reviewers pay special attention to? -->

-

### Questions for Reviewers

<!-- Any specific questions or concerns? -->

-

---

**By submitting this PR, I confirm that:**

- [ ] I have read and followed the [CONTRIBUTING.md](../CONTRIBUTING.md) guidelines
- [ ] My code follows the project's code of conduct
- [ ] I have tested my changes thoroughly
- [ ] I agree to license my contributions under the MIT License
