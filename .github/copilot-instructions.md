# Copilot Instructions for Eunomia

## Project Overview

**Eunomia** is the authorization policy platform for the Themis ecosystem. It provides Git-backed OPA/Rego policy management, testing frameworks, bundle compilation, and hybrid push/pull distribution to Archimedes services.

## Technology Stack

| Area          | Technology           |
| ------------- | -------------------- |
| Language      | Rust (latest stable) |
| Async Runtime | Tokio                |
| Policy Engine | OPA/Rego             |
| gRPC          | Tonic                |
| Serialization | Serde                |
| CLI           | Clap                 |
| Testing       | Built-in + proptest  |

## Development Workflow

### CRITICAL: Always Review Before Changes

Before making any code changes, **always review these documents**:

- `docs/roadmap.md` - Current phase, tasks, and priorities
- `docs/design.md` - Architecture decisions and patterns
- `docs/spec.md` - Requirements and specifications

This ensures changes align with project direction and existing decisions.

## Development Guidelines

### Terminal Commands

- **Never use `2>&1`** for redirecting stderr - let errors display naturally
- Prefer simple commands without complex shell redirections
- Use `cargo` commands directly without output redirection
- **Avoid long-running subprocesses** that may not terminate (e.g., watch modes)
- When running tests, prefer `cargo test` over `cargo test -p <crate>` if the subprocess hangs

### CRITICAL: Avoid Interactive Commands

These commands will hang the terminal and must be avoided:

- **Git commits**: Always use `git commit -m "message"` (never bare `git commit`)
- **Git pager**: Use `git --no-pager log` or `git --no-pager diff`
- **Confirmation prompts**: Use `-y` or `--yes` flags when available
- **Interactive editors**: Never run commands that open vim/nano/editors
- **Read prompts**: Avoid commands that wait for stdin input
- **Cargo watch**: Don't use `cargo watch` or similar watch modes

**Safe patterns:**

```bash
# Good - non-interactive
git add -A && git commit -m "feat: add feature"
git --no-pager log --oneline -10
cargo test --no-fail-fast

# Bad - will hang
git commit          # Opens editor
git log            # Opens pager
read -p "Continue?"  # Waits for input
```

### Code Formatting

- Use `rustfmt` with default settings
- Run `cargo fmt` before every commit
- Use `cargo clippy` and fix all warnings (treat warnings as errors in CI)

### Linting Rules

```toml
# Cargo.toml or .cargo/config.toml
[lints.rust]
unsafe_code = "forbid"
missing_docs = "warn"

[lints.clippy]
all = "warn"
pedantic = "warn"
nursery = "warn"
```

### Naming Conventions

- Types: `PascalCase`
- Functions/methods: `snake_case`
- Constants: `SCREAMING_SNAKE_CASE`
- Modules: `snake_case`
- Crates: `kebab-case` (e.g., `eunomia-core`)

## Testing Requirements

### CRITICAL: Test-Driven Development

**Every change MUST include tests.** This is non-negotiable.

1. **Before writing code** → Write a failing test first (TDD)
2. **New features** → Add unit tests + integration tests
3. **Bug fixes** → Add regression test that fails before fix
4. **Refactors** → Ensure existing tests still pass
5. **New files** → Every new `.rs` file needs corresponding tests

### Test Structure

```rust
// Unit tests in same file
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature_name() {
        // Arrange
        let input = ...;

        // Act
        let result = function_under_test(input);

        // Assert
        assert_eq!(result, expected);
    }
}

// Integration tests in tests/ directory
// tests/integration_test.rs
```

### Policy Testing

For Rego policy tests:

```rust
#[test]
fn test_policy_allows_admin() {
    let policy = load_policy("authz.rego");
    let input = json!({
        "caller": { "type": "user", "roles": ["admin"] },
        "action": "write"
    });
    assert!(evaluate_policy(&policy, input).allowed);
}

#[test]
fn test_policy_denies_unauthorized() {
    let policy = load_policy("authz.rego");
    let input = json!({
        "caller": { "type": "user", "roles": [] },
        "action": "write"
    });
    assert!(!evaluate_policy(&policy, input).allowed);
}
```

### Run Tests Before Every Push

```bash
# Run ALL of these before pushing
cargo test                        # Unit + integration tests
cargo clippy -- -D warnings       # Linting (fail on warnings)
cargo fmt --check                 # Formatting check
cargo doc --no-deps               # Ensure docs build
```

## Documentation Requirements

### CRITICAL: Document After Every Change

**After writing tests and implementing a change, you MUST add documentation.** This is mandatory, not optional.

#### Development Workflow: Test → Implement → Document

1. Write failing test
2. Implement the change
3. Verify tests pass
4. **Add in-code documentation (rustdoc)**
5. **Update `docs/` for significant changes**

### In-Code Documentation (Always Required)

1. **New function** → Add rustdoc with examples
2. **New module** → Add module-level documentation
3. **New crate** → Update crate-level docs and README
4. **API changes** → Update all affected rustdoc immediately
5. **New types** → Document all public structs, enums, traits

### Documentation in `docs/` (Required for Significant Changes)

Update the `docs/` folder when changes affect:

1. **Contracts & Constraints**

   - New policy validation rules → Update `docs/spec.md`
   - Changed Rego parsing behavior → Update `docs/design.md`
   - New policy input/output schemas → Update both

2. **Breaking Changes**

   - Any breaking change → Document in `docs/` with migration guide
   - Changed bundle format → Update `docs/design.md`

3. **New Features**

   - New CLI commands → Update `docs/roadmap.md` and README
   - New distribution mechanisms → Update `docs/design.md`
   - New testing capabilities → Document in `docs/spec.md`

4. **Architecture Changes**
   - New crates → Update `docs/design.md`
   - Changed control plane API → Update architecture diagrams

### What Counts as "Significant"?

- Changes to public API surface
- New constraints or validation rules
- Policy behavior changes that affect users
- New capabilities or features
- Deprecations or removals

### Rustdoc Standards

````rust
/// Brief one-line description.
///
/// Longer description if needed, explaining the purpose
/// and any important details.
///
/// # Arguments
///
/// * `policy_input` - The authorization request to evaluate
///
/// # Returns
///
/// The authorization decision with reason
///
/// # Errors
///
/// Returns `PolicyError` if:
/// - Policy file cannot be loaded
/// - Rego evaluation fails
///
/// # Examples
///
/// ```
/// use eunomia_core::evaluate;
///
/// let decision = evaluate(&policy, input)?;
/// assert!(decision.allowed);
/// ```
pub fn evaluate(policy: &Policy, input: PolicyInput) -> Result<Decision, PolicyError> {
    // implementation
}
````

## Git Practices

### Commit Often, Push Frequently

- Make small, focused commits
- Each commit should be a logical unit of work
- Push at least at end of each work session
- Never leave work only on local machine

### Commit Message Format

```
type(scope): short description

- Detail 1
- Detail 2

Refs: #issue-number (if applicable)
```

**Types:**

- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation only
- `refactor`: Code change that neither fixes bug nor adds feature
- `test`: Adding or updating tests
- `chore`: Build, CI, tooling changes

**Examples:**

```
feat(compiler): add bundle signature verification

- Verify Ed25519 signatures on bundle load
- Reject unsigned bundles in production mode
- Add signature_required config option

test(compiler): add signature verification tests

- Test valid signature acceptance
- Test invalid signature rejection
- Test unsigned bundle handling
```

### Branch Strategy

- `main` – Always deployable, protected
- `feat/description` – Feature branches
- `fix/description` – Bug fix branches
- `docs/description` – Documentation branches

## Dependency Management

### Use Latest Stable Versions

- Always use the latest stable version of dependencies
- Run `cargo update` regularly
- Check for security advisories with `cargo audit`

### Minimize Dependencies

- Prefer well-maintained, audited crates
- Evaluate necessity before adding new dependencies
- Pin versions in `Cargo.lock`

## Error Handling

- Use `Result<T, E>` for fallible operations
- Use `thiserror` for defining error types
- **No `.unwrap()` in library code** (only tests/examples)
- Provide context with error chaining

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PolicyError {
    #[error("Failed to load policy from {path}: {source}")]
    LoadError {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("Policy evaluation failed: {reason}")]
    EvaluationError { reason: String },
}
```

## Security Practices

- Never commit secrets or credentials
- Use environment variables for configuration
- All external input is untrusted (validate everything)
- Follow principle of least privilege
- Log security-relevant events
- Sign all policy bundles

## Performance Considerations

- Profile before optimizing
- Document performance-critical paths
- Add benchmarks for hot paths (`cargo bench`)
- Consider memory allocation patterns

## Project Structure

```
eunomia/
├── .github/
│   └── copilot-instructions.md   # This file
├── docs/
│   ├── design.md                 # Implementation design
│   ├── spec.md                   # Specification
│   └── roadmap.md                # Development roadmap
├── README.md
└── CONTRIBUTING.md
```

## Key Reminders

1. **Test everything** – If it's not tested, it's broken
2. **Document immediately** – Don't defer documentation
3. **Format and lint** – Run `cargo fmt` and `cargo clippy` always
4. **Small commits** – Atomic, focused changes
5. **Latest versions** – Keep dependencies up to date
6. **No unsafe code** – Unless absolutely necessary and documented
7. **Error handling** – No `.unwrap()` in production code
8. **Security first** – All inputs are untrusted

## CI Checklist

Before creating a PR, ensure:

- [ ] All tests pass (`cargo test`)
- [ ] No clippy warnings (`cargo clippy -- -D warnings`)
- [ ] Code is formatted (`cargo fmt --check`)
- [ ] Docs build (`cargo doc --no-deps`)
- [ ] No security vulnerabilities (`cargo audit`)
- [ ] Documentation updated for changes
- [ ] Commit messages follow format
