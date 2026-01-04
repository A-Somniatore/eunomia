# Contributing to Eunomia

Thank you for your interest in contributing to Eunomia! This document provides guidelines and instructions for contributing.

## Development Setup

### Prerequisites

- Rust 1.75+ (latest stable recommended)
- OPA binary installed
- Git

### Getting Started

```bash
# Navigate to the eunomia workspace
cd eunomia

# Build the project (when code is added)
cargo build

# Run tests
cargo test
```

## Development Workflow

### Before Making Changes

1. Create a feature branch from `main`
2. Ensure all existing tests pass
3. Read relevant documentation in `docs/`

### Making Changes

1. **Write tests first** – Follow TDD principles
2. **Keep commits small** – One logical change per commit
3. **Document as you go** – Update docs with code changes
4. **Format and lint** – Run `cargo fmt` and `cargo clippy`

### Before Pushing

Run the full check suite:

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test
cargo doc --no-deps
```

## Code Standards

### Testing Requirements

Every change must include tests:

- **New feature** → Unit tests + integration tests
- **Bug fix** → Regression test that fails before fix
- **Refactor** → Ensure existing tests pass
- **New file** → Corresponding test module

### Documentation Requirements

Every change must include documentation:

- **New function** → Rustdoc with examples
- **New module** → Module-level documentation
- **API change** → Update all affected docs
- **New feature** → Update relevant design docs

### Code Style

- Use `rustfmt` with default settings
- No clippy warnings allowed
- No `.unwrap()` in library code
- Use `thiserror` for error types

## Commit Messages

Follow this format:

```
type(scope): short description

- Detail 1
- Detail 2

Refs: #issue-number
```

**Types:** `feat`, `fix`, `docs`, `refactor`, `test`, `chore`

## Pull Request Process

1. Ensure all checks pass
2. Update documentation
3. Request review from maintainers
4. Address feedback promptly
5. Squash commits if requested

## Questions?

Open an issue for questions or discussions.
