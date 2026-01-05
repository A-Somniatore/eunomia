# Eunomia – Development Roadmap

> **Version**: 1.5.0  
> **Created**: 2026-01-04  
> **Last Updated**: 2026-01-05  
> **Target Completion**: Week 20 (MVP Integration with Archimedes MVP)

> ✅ **CTO REVIEW (2026-01-05)**: Phase E0 complete. Shared types migration resolved.
> All Eunomia types now use `themis-platform-types` for schema compatibility.

---

## 🔄 themis-platform-types v0.2.1 Production Readiness (Coming Soon)

> **When**: Before production release
> **Status**: Development Complete - Pending Publish

### New Production Guarantees (v0.2.1)

1. **Thread Safety** - `Send + Sync` compile-time assertions for all types
2. **MSRV Testing** - CI validates Rust 1.75 compatibility
3. **Schema Validation** - JSON schemas validated against Rust types in CI
4. **Serialization Testing** - Property-based roundtrip tests for policy types
5. **Fallible Constructor** - `RequestId::try_new()` for edge cases
6. **Security Lint** - `#[must_use = "security bug"]` on `PolicyDecision`

### Impact on Eunomia

- **Audit Logging**: `Versioned<PolicyDecision>` provides schema version for stored decisions
- **Policy Testing**: `eunomia-test` should use `try_build()` for cleaner error messages
- **Performance**: Types verified thread-safe for async policy evaluation

```rust
// In eunomia-test fixtures
let input = PolicyInput::builder()
    .caller(mock_user("test-user"))
    .service("test-service")
    .operation_id("testOp")
    .method("GET")
    .path("/test")
    .request_id(RequestId::try_new().unwrap_or_default())
    .try_build()
    .expect("test fixture should be valid");
```

---

## 🔄 themis-platform-types v0.2.0 Migration (Required)

> **When**: Week 9 (during E2 Testing Framework phase)
> **Effort**: ~2 hours
> **Breaking Changes**: Yes

### Migration Checklist

- [ ] Update `Cargo.toml` to `themis-platform-types = "0.2.0"`
- [ ] Replace `build()` calls with `try_build()?` (build() is deprecated)
- [ ] Update error handling to use `BuilderError` instead of `&'static str`
- [ ] Add wildcard arms to match statements on `CallerIdentity`, `ErrorCode`
- [ ] Update `eunomia-test` mock builders to handle new error type

### Code Changes Required

```rust
// Before (v0.1.0) - in mock_identity.rs
let input = PolicyInput::builder()
    .caller(caller)
    .service("my-service")
    .try_build()?; // Returns Result<_, &'static str>

// After (v0.2.0)
use themis_platform_types::BuilderError;
let input = PolicyInput::builder()
    .caller(caller)
    .service("my-service")
    .try_build()?; // Returns Result<_, BuilderError>

// Match statements need wildcard (enums are now #[non_exhaustive])
match caller {
    CallerIdentity::User(u) => handle_user(u),
    CallerIdentity::Spiffe(s) => handle_service(s),
    CallerIdentity::ApiKey(k) => handle_api_key(k),
    CallerIdentity::Anonymous => handle_anon(),
    _ => unreachable!("unknown identity type"), // Future-proof
}
```

### New Features Available

- `Versioned<T>` wrapper for policy decision auditing
- `SchemaMetadata` for version tracking in bundles
- Proper `BuilderError` with descriptive field names
- Fixed SemVer pre-release comparison for policy versioning

---

## Key Decisions

| Decision                                                         | Impact                                              |
| ---------------------------------------------------------------- | --------------------------------------------------- |
| [ADR-008](../../docs/decisions/008-archimedes-full-framework.md) | Archimedes is full framework replacement (40 weeks) |
| [ADR-004](../../docs/decisions/004-regorus-for-rego-parsing.md)  | Use Regorus for Rego parsing and evaluation         |
| [ADR-002](../../docs/decisions/002-opa-for-authorization.md)     | OPA/Rego as the policy language                     |
| [ADR-003](../../docs/decisions/003-push-based-policies.md)       | Hybrid push/pull policy distribution                |
| [ADR-007](../../docs/decisions/007-apache-2-license.md)          | Apache 2.0 license                                  |

**Resolved Open Questions:**

- ✅ **Cache encryption key management**: Use K8s Secrets + external-secrets-operator for rotation
- ✅ **Cross-region replication**: OCI registry geo-replication, single-region control plane for MVP

**PolicyInput Schema (from `themis-platform-types`):**

- `caller` (CallerIdentity: spiffe, user, api_key, anonymous)
- `service`, `operation_id` (from Themis contract)
- `method`, `path`, `headers`, `timestamp`, `environment`
- `context` (Map for resource attributes, extracted parameters)

**Note**: Policies should use `input.operation_id` and `input.context`, NOT `input.action` or `input.resource`.

---

## Overview

Eunomia is the authorization policy platform for the Themis ecosystem. Development runs **in parallel with Themis** during the first 12 weeks, then integrates with Archimedes.

### Timeline Summary

| Phase                       | Duration | Weeks | Description                               |
| --------------------------- | -------- | ----- | ----------------------------------------- |
| E0: Shared Types            | 1 week   | 1     | Integrate `themis-platform-types`         |
| E1: Foundation              | 3 weeks  | 2-4   | Core types, Rego parsing, validation      |
| E2: Testing Framework       | 4 weeks  | 5-8   | Test runner, fixtures, bundle compilation |
| E3: Registry & Distribution | 4 weeks  | 9-12  | Registry client, push distribution        |
| E4: Archimedes Integration  | 4 weeks  | 17-20 | Full integration with Archimedes OPA      |

**Total**: ~16 weeks of active development (with gap weeks 13-16 for Archimedes catch-up)

### Cross-Component Timeline Alignment

**MVP Timeline (Weeks 1-20):**

```
         Week: 1   2   3   4   5   6   7   8   9  10  11  12  13  14  15  16  17  18  19  20
 Themis:      [T0][---T1---][--T2--][------T3------][--T4--][--T5--]
 Eunomia:     [E0][---E1---][------E2------][------E3------]        (gap)        [------E4------]
 Archimedes:  [A0][---A1---][----------A2----------][----------A3----------][A4][------A5------]
```

**Full Framework Timeline (Weeks 21-40):**

```
         Week: 21  22  23  24  25  26  27  28  29  30  31  32  33  34  35  36  37  38  39  40
 Archimedes:  [------A6------][------A7------][------A8------][------A9------][-----A10------]
                  Router         FastAPI        WebSocket         CLI        Multi-Lang
                Extractors       Parity          SSE/Tasks       DevExp       SDKs
```

**Key Coordination Points**:

- Week 1: All components depend on `themis-platform-types` (created by Themis)
- Week 12: Themis artifacts available for Archimedes validation
- Week 12: Eunomia bundles available for Archimedes OPA
- Weeks 13-16: Eunomia team can support Archimedes integration or work on stretch goals
- Week 17-20: Full end-to-end MVP integration testing
- Week 40: Archimedes full framework release (replaces Axum/FastAPI/Boost)

---

## Phase E0: Shared Platform Types (Week 1) ✅ COMPLETE

> **Note**: The `themis-platform-types` crate is created by Themis team in Week 1.
> Eunomia integrates it to ensure schema compatibility.

### Week 1: Integrate Shared Types

> ✅ **Completed (2026-01-05)**: `themis-platform-types` integration complete!
> All shared types now come from the platform crate. Local modules deprecated.

- [x] Add `themis-platform-types` dependency to `eunomia-core`
  > ✅ **Completed**: Added as workspace dependency, re-exported from `eunomia-core`
- [x] Migrate `CallerIdentity` to use shared definition
  > ✅ **Completed**: Re-exported from `themis-platform-types`, local module deprecated
- [x] Migrate `PolicyInput` to use shared definition
  > ✅ **Completed**: Re-exported from `themis-platform-types`, local module deprecated
- [x] Migrate `PolicyDecision` to use shared definition
  > ✅ **Completed**: Re-exported as `PolicyDecision`, `AuthorizationDecision` alias deprecated
- [x] Update existing code to use shared types
  > ✅ **Completed**: Updated `eunomia-test` mock builders to use new API
- [x] Verify JSON serialization matches spec
  > ✅ **Completed**: All 331 tests passing

**Migration Notes:**

- `CallerIdentity` now uses tuple variants with inner structs (`User(UserIdentity)`, etc.)
- `CallerIdentity::user()` takes `(user_id, email)` instead of `(user_id, roles)`
- Use `CallerIdentity::spiffe_full()` for full SPIFFE details
- `CallerIdentity::api_key()` takes `(key_id, name)` instead of `(key_id, scopes)`
- `is_spiffe()` renamed to `is_service()`
- `identity_type()` renamed to `identifier()`
- Local modules (`identity`, `input`, `decision`) are deprecated but preserved for compatibility

### Phase E0 Milestone

**Criteria**: Eunomia uses `themis-platform-types` for all shared types

> ✅ **Status**: COMPLETE. All shared types migrated to `themis-platform-types`.

---

## Phase E1: Foundation (Weeks 2-4)

### Week 2: Project Setup & Core Types

- [x] Create `eunomia` repository structure
  > **Completed 2026-01-04**: Created workspace root with Cargo.toml
- [x] Set up Cargo workspace:
  ```
  crates/
  ├── eunomia-core/       # Core types
  ├── eunomia-compiler/   # Policy compilation
  ├── eunomia-test/       # Testing framework
  └── eunomia-cli/        # CLI application
  ```
  > **Completed 2026-01-04**: All four crates created with proper module structure
- [x] Configure CI pipeline (GitHub Actions)
  > **Completed 2026-01-04**: Added `.github/workflows/ci.yml` with check, test, fmt, clippy, docs, and audit jobs. Added release workflow.
- [x] Define policy data models
  > **Completed 2026-01-04**: Implemented in `eunomia-core`:
  >
  > - `Policy` model with metadata support
  > - `Bundle` model with builder pattern
  > - `AuthorizationDecision` type
  > - `PolicyInput` schema with builder
  > - `CallerIdentity` types (SPIFFE, User, ApiKey, Anonymous)
  >
  > **Note**: These will be migrated to `themis-platform-types` in Week 1
- [x] Write initial documentation
  > **Completed 2026-01-04**: Added rustdoc for all public APIs, README for eunomia-core
- [x] Add property-based testing with proptest
  > **Completed 2026-01-04**: Added comprehensive proptest tests for:
  >
  > - `CallerIdentity` serialization roundtrips
  > - `PolicyInput` serialization roundtrips
  > - `AuthorizationDecision` consistency
  > - `Bundle` construction and serialization
  > - `Policy` is_test detection
  >
  > Tests cover edge cases with random valid inputs
- [x] Implement validation framework
  > **Completed 2026-01-04**: Added `validation` module with:
  >
  > - `ValidationError` and `ValidationErrorKind` types
  > - `ValidationErrors` collection
  > - `Validate` trait for type validation
  > - Comprehensive unit tests for validation
- [x] Fix all Clippy warnings (pedantic, nursery)
  > **Completed 2026-01-04**: All crates pass `cargo clippy -- -D warnings`

### Week 3: Rego Parsing

> 📝 **Decision**: Using `regorus` crate (pure Rust OPA implementation).
> See [ADR-004](../../../docs/decisions/004-regorus-for-rego-parsing.md) for rationale.

- [x] Integrate Rego parser using `regorus` crate
  > **Completed**: Added `regorus = "0.3"` dependency, created `RegoEngine` wrapper
- [x] **Spike: Validate regorus capabilities** against our policy patterns
  > **Completed**: Tested with sample policies, works well with our patterns
- [x] Implement policy file loading
  > **Completed**: `RegoEngine::add_policy_from_file()` with PolicyInfo tracking
- [x] Add static analysis for common errors
  > **Completed**: Created `Linter` with security and style rules:
  >
  > - `security/default-deny`: Require default allow := false
  > - `security/no-hardcoded-secrets`: Detect hardcoded credentials
  > - `security/no-wildcard-allow`: Warn on unconditional allow
  > - `style/explicit-imports`: Recommend explicit imports
- [x] Validate policy structure
  > **Completed**: Created `PolicyValidator` combining syntax, analysis, and linting
- [x] Test with sample policies
  > **Completed**: Created example policies for users-service and orders-service
  >
  > - Demonstrates user, SPIFFE, API key authorization patterns
  > - Includes comprehensive test files (\*\_test.rego)
  > - Added common/authz.rego with reusable helpers
- [x] Document policy file conventions
  > **Completed**: Added Section 13 to spec.md with:
  >
  > - Directory structure conventions
  > - Package naming conventions
  > - METADATA comment format
  > - Import conventions
  > - Default deny pattern requirements
  > - Test file conventions
  > - Input schema documentation
  > - Linting rules documentation

### Week 4: Policy Validation

- [x] Implement policy syntax checking
  > **Completed**: `PolicyValidator` handles syntax checking via `RegoEngine`
- [x] Add semantic validation
  > **Completed**: Created `SemanticValidator` in `semantic.rs`:
  >
  > - Deep policy analysis beyond syntax checking
  > - `SemanticIssue`, `SemanticSeverity`, `SemanticCategory` types
  > - Operation ID validation against service contracts
  > - Unused rule detection to identify dead code
  > - Deprecated input field detection
  > - Rule reference checking
- [x] Create mock operationId support (for testing without Themis)
  > **Completed**: Created `MockServiceContract`:
  >
  > - Define service contracts with operation IDs
  > - Validate policies against expected operations
  > - Predefined contracts: `users_service_contract()`, `orders_service_contract()`
  > - `InputSchema` for validating authorization input structure
- [x] Implement validation error reporting
  > **Completed**: `SemanticIssue` provides structured reporting with:
  >
  > - Severity levels (Error, Warning, Info)
  > - Category classification (OperationId, Unused, Deprecated, Schema, Reference)
  > - Rule names, descriptions, and suggestions
- [x] Add structured error messages
  > **Completed**: All validation errors include:
  >
  > - Clear descriptions of what's wrong
  > - Context about affected policies/rules
  > - Suggestions for how to fix issues

### Phase E1 Milestone

**Criteria**: Rego policies can be parsed, validated, and loaded

> ✅ **Status**: Phase E1 Complete!
>
> - Week 2 complete: Project setup, core types, validation framework
> - Week 3 complete: Rego parsing with regorus, linting rules
> - Week 4 complete: Semantic validation with mock contracts

---

## Phase E2: Testing Framework (Weeks 5-8)

### Week 5: Test Runner

- [x] Implement test case discovery
  > **Completed**: Created `TestDiscovery` with:
  >
  > - Recursive directory scanning for `*_test.rego` files
  > - Package name extraction from test files
  > - Test rule detection (`test_*` prefixed rules)
  > - Associated policy file discovery
  > - Fixture file discovery (JSON/YAML)
  > - `DiscoveryConfig` for customization (patterns, recursion)
- [x] Parse `*_test.rego` files
  > **Completed**: `TestDiscovery` parses test files to extract:
  >
  > - Package names
  > - Test rule names
  > - Source file paths
  > - Associates tests with `DiscoveredTest` and `TestSuite`
- [x] Execute OPA eval for tests
  > **Completed**: `TestRunner::run_suite()` executes tests using `RegoEngine`:
  >
  > - Loads policy files into engine
  > - Evaluates each test rule
  > - Reports pass/fail with detailed errors
  > - Supports fail-fast mode
  > - Handles native Rego tests (self-contained)
- [x] Collect pass/fail results
  > **Completed**: `TestResults` aggregates test outcomes:
  >
  > - `passed()` / `failed()` counts
  > - `all_passed()` check
  > - `failures()` iterator for detailed errors
  > - Duration tracking per test
- [x] Generate test report (console output)
  > **Completed**: `ConsoleReporter` with:
  >
  > - Color-coded pass/fail output
  > - Test duration display
  > - Error message formatting
  > - Summary with pass/fail counts
- [x] Add `eunomia test` CLI command
  > **Completed**: Full CLI implementation with:
  >
  > - Test discovery from directory
  > - Fail-fast mode (`-f`)
  > - Parallel execution flag (`-p`)
  > - Verbose output (`-v`)
  > - No-color mode (`--no-color`)
  > - Filter option (`--filter`)

### Week 6: Test Fixtures & Import Resolution

> **Design Decisions**: See [design.md Section 11.0](design.md#110-design-decisions)

- [x] **Fix import resolution** - Load all `.rego` files so imports work
  > **Completed**: All `.rego` policy files are now loaded into the RegoEngine before test execution.
  > Discovery finds all policy files in directory tree and runner loads them.
- [x] Support data files for policies (`data.json`)
  > **Completed**: `TestSuite` now stores `data_files: HashMap<PathBuf, serde_json::Value>`.
  > Discovery finds `data.json`/`data.yaml` files, runner loads them via `engine.add_data()`.
- [x] Add mock identity helpers (`MockIdentity` builder)
  > **Completed**: Created `mock_identity` module with:
  >
  > - `MockUser` builder with factory methods: `admin()`, `viewer()`, `editor()`, `guest()`, `super_admin()`
  > - `MockSpiffe` builder with factories: `users_service()`, `orders_service()`, `gateway()`
  > - `MockApiKey` builder with factories: `read_only()`, `full_access()`, `read_service()`, `write_service()`
  > - Fluent APIs for customization
- [x] Implement fixture loading from JSON/YAML
  > **Completed**: `FixtureSet::from_json_file()` and `from_yaml_file()` for loading.
  > Runner integrates with `run_discovered_fixtures()` and `run_all()` methods.
  > Auto-detects policy files based on fixture naming conventions.
- [x] Create test utilities library
  > **Completed**: Created `test_utils` module with:
  >
  > - `InputBuilder` - Fluent builder for policy input JSON
  > - Assertion helpers: `assert_allowed()`, `assert_denied()`, `assert_all_passed()`
  > - Policy generators: `simple_allow_policy()`, `role_based_policy()`, `scope_based_policy()`
- [x] Document testing patterns
  > **Completed**: Created comprehensive `docs/testing-guide.md` covering:
  >
  > - Native Rego tests and fixture-based tests
  > - Mock identity builders
  > - Test utilities and assertions
  > - Best practices and troubleshooting
- [x] Add example test files with fixtures
  > **Completed**: Added `examples/policies/users-service/authz_fixtures.json` with 8 test fixtures.
  > Updated examples README with fixture documentation.

### Week 7: Bundle Compilation

- [x] Implement OPA bundle compilation
  > **Completed**: `Bundler` compiles policies into OPA-compatible bundles:
  >
  > - `add_policy_dir()` loads all `.rego` files from directory
  > - `add_data_dir()` loads `data.json`/`data.yaml` files
  > - Validates policies with `Analyzer` (optional)
  > - Optimizes policies with `Optimizer` (optional)
- [x] Generate bundle manifest
  > **Completed**: `Bundle::generate_manifest()` creates OPA-compatible `.manifest`:
  >
  > - Standard OPA fields: `revision`, `roots`
  > - Eunomia extensions under `metadata.eunomia`: version, service, git_commit, created_at
  > - Checksum under `metadata.checksum` with SHA-256 value
- [x] Include policy data files
  > **Completed**: `Bundler::add_data_file()` and `add_data_dir()` methods.
  > Data files are included in tar.gz bundle.
- [x] Add metadata (version, timestamp, git commit)
  > **Completed**: Bundle includes:
  >
  > - `version`: Semantic version from `Bundler::version()`
  > - `created_at`: RFC 3339 timestamp
  > - `git_commit`: Optional commit SHA from `Bundler::git_commit()`
- [x] Create tar.gz bundle format
  > **Completed**: `Bundle::write_to_file()` creates OPA-compatible tar.gz:
  >
  > - `.manifest` JSON at root
  > - Policy files organized by package namespace
  > - Data files included at appropriate paths
  > - `Bundle::from_file()` for reading bundles back
- [x] Test bundle structure
  > **Completed**: Comprehensive tests added:
  >
  > - `test_bundle_roundtrip_bytes` - full serialization/deserialization
  > - `test_bundler_compile_to_file` - file export and reload
  > - `test_compute_checksum_deterministic` - checksum consistency
  > - `test_generate_manifest` - manifest format validation
  > - `test_bundler_add_policy_dir` - directory loading

### Week 8: Bundle Signing & CLI

- [x] Implement Ed25519 signing
  > **Completed**: Added `signing` module to `eunomia-core`:
  >
  > - `SigningKeyPair`: Ed25519 key generation, base64 export/import
  > - `BundleSigner`: Sign bundle checksums with private key
  > - `BundleVerifier`: Verify signatures with public keys
  > - `SignedBundle`: Bundle with attached signatures
  > - `SignatureFile`: OPA-compatible `.signatures.json` format
  > - Algorithm: Ed25519 (RFC 8032) via `ed25519-dalek` crate
- [x] Generate bundle signature
  > **Completed**: `BundleSigner::sign()` creates signatures:
  >
  > - Signs SHA-256 checksum (hex string) of bundle
  > - Produces base64-encoded Ed25519 signature
  > - Returns `BundleSignature` with key_id, algorithm, value
- [x] Add public key management
  > **Completed**: Key management features:
  >
  > - `SigningKeyPair::generate()` for new key pairs
  > - `public_key_base64()` and `private_key_base64()` for export
  > - `from_private_key_base64()` for key restoration
  > - Human-readable key IDs for signature identification
- [x] Implement `eunomia build` command
  > **Completed**: Full build command implementation:
  >
  > - `Bundler::add_policy_dir()` for recursive policy loading
  > - `Bundler::compile_to_file()` for bundle output
  > - Shows bundle metadata (name, version, policies, checksum)
  > - Progress output with checkmarks
- [x] Add `eunomia sign` command
  > **Completed**: Sign command with full functionality:
  >
  > - `--key-file` for private key file path
  > - `--key` / `EUNOMIA_SIGNING_KEY` env var support
  > - `--key-id` for signature identification
  > - `--generate-key` flag for Ed25519 key pair generation
  > - Outputs `.sig` file alongside bundle
- [x] Write signing documentation
  > **Completed**: Added Section 8.0.5 to design.md:
  >
  > - Algorithm choice rationale (Ed25519 vs RSA/ECDSA)
  > - Signed content (SHA-256 checksum)
  > - Signature format (OPA-compatible JSON)
  > - Key management approach
  > - CLI integration examples

### Phase E2 Milestone

**Criteria**: Policy tests run and report results, signed bundles are created

> ✅ **Status**: Phase E2 Complete!
>
> - Week 5 complete: Test runner with discovery, execution, reporting, CLI
> - Week 6 complete: Fixtures, import resolution, mock identities, test utilities
> - Week 7 complete: Bundle compilation with manifest, checksums, tar.gz format
> - Week 8 complete: Ed25519 signing, CLI build/sign commands, key management

---

## Phase E3: Registry & Distribution (Weeks 9-12)

### Week 9: Registry Client

- [x] Design registry API (OCI-compatible)
  > **Completed**: Created `eunomia-registry` crate with OCI Distribution Spec support:
  >
  > - `RegistryConfig`: URL, namespace, auth, timeout, TLS settings
  > - `RegistryAuth`: None, Basic, Bearer, AWS ECR (stub), GCP Artifact (stub)
  > - `TlsConfig`: mTLS support with client cert/key
  > - OCI types: `Manifest`, `Descriptor`, `MediaType`, `TagList`
- [x] Implement bundle registry client
  > **Completed**: `RegistryClient` with full OCI Distribution API:
  >
  > - `exists()`: Check if bundle exists (HEAD manifest)
  > - `list_tags()`: List available versions
  > - `fetch_manifest()`: Get OCI manifest
  > - `fetch_blob()`: Download blob by digest
  > - `upload_blob()`: Push blob with chunked uploads
  > - `push_manifest()`: Push OCI manifest
  > - Auth header generation for all methods
- [x] Add publish functionality
  > **Completed**: `RegistryClient::publish()` and CLI `eunomia publish`:
  >
  > - Serializes bundle to bytes
  > - Computes SHA-256 digest
  > - Uploads blob layer
  > - Creates OCI manifest with annotations
  > - CLI supports --registry, --service, --version, --token, --username/--password
- [x] Add fetch functionality
  > **Completed**: `RegistryClient::fetch()` and CLI `eunomia fetch`:
  >
  > - Fetches manifest to find bundle layer
  > - Downloads blob by digest
  > - Verifies size and checksum
  > - Parses bundle from bytes
  > - CLI supports --registry, --service, --version, --output, --info-only
- [x] Implement versioning support
  > **Completed**: `VersionQuery` and `VersionResolver`:
  >
  > - `Latest`: Resolves to highest semver tag
  > - `Major(u64)`: Matches highest v{major}.x.x
  > - `Minor(u64, u64)`: Matches highest v{major}.{minor}.x
  > - `Exact(String)`: Matches exact tag
  > - `Digest(String)`: Uses sha256 digest directly
  > - Semantic version sorting for resolution
- [x] Add caching layer
  > **Completed**: `BundleCache` with file-based caching:
  >
  > - LRU eviction based on access time
  > - Configurable max_size_bytes and max_age
  > - `get()` / `put()` / `invalidate()` operations
  > - `prune()` for cleanup with stats
  > - Cross-platform cache directory via `dirs` crate

### Week 10: Control Plane API

> 🔄 **In Progress (2026-01-05)**: Core distributor crate created, gRPC API designed.

- [x] Design gRPC API (protobuf)
  > **Completed**: Created `proto/control_plane.proto` with:
  > - `ControlPlane` service: DeployPolicy, RollbackPolicy, GetPolicyStatus, ListInstances, etc.
  > - `PolicyReceiver` service: UpdatePolicy, GetCurrentPolicy, HealthCheck
  > - Comprehensive message types for deployment operations
- [x] Add deployment state tracking
  > **Completed**: Created `state.rs` with:
  > - `DeploymentState` enum: Pending, InProgress, Completed, Failed, RolledBack, Cancelled
  > - `DeploymentInfo` for tracking deployment metadata
  > - `DeploymentTracker` for async state management
- [x] Implement health checks
  > **Completed**: Created `health.rs` with:
  > - `HealthState` enum for instance health status
  > - `HealthCheck` struct with policy version tracking
  > - `HealthTracker` for managing health state transitions
- [x] Create control plane service scaffold
  > **Completed**: Created `eunomia-distributor` crate with:
  > - `Distributor` main struct with deployment logic
  > - Deployment strategies: Immediate, Canary, Rolling
  > - `PolicyPusher` for pushing bundles to instances
  > - `DeploymentScheduler` with priority queue
  > - 84 tests passing
- [ ] Implement bundle management endpoints (gRPC server implementation)
  > **Pending**: Actual tonic gRPC server implementation (protobuf types currently manual)

### Week 11: Instance Discovery

> 🔄 **Mostly Complete**: Static and DNS discovery implemented, K8s pending.

- [x] Track Archimedes instances
  > **Completed**: Created `instance.rs` with:
  > - `Instance` struct with id, endpoint, metadata, status
  > - `InstanceEndpoint` for host/port/TLS configuration
  > - `InstanceMetadata` for K8s labels, annotations, namespace
  > - `InstanceStatus` enum for health tracking
- [x] Add instance health monitoring
  > **Completed**: Health monitoring integrated into `Distributor`:
  > - `PolicyPusher::health_check()` for individual instances
  > - `HealthTracker` for state transitions
  > - Configurable thresholds (healthy_threshold, unhealthy_threshold)
- [x] Implement instance grouping
  > **Completed**: Created `discovery.rs` with:
  > - `Discovery` trait for pluggable discovery sources
  > - `StaticDiscovery` for manual endpoint configuration
  > - `CombinedDiscovery` for aggregating multiple sources
  > - `CachedDiscovery` for TTL-based caching
- [x] Implement DNS service discovery
  > **Completed**: Created `DnsDiscovery` with:
  > - Uses hickory-resolver for DNS lookups
  > - Supports A (IPv4) and AAAA (IPv6) records
  > - Custom resolver configuration support
  > - Instance metadata with DNS host and resolved IP
  > - 6 tests covering localhost, nonexistent hosts, mixed scenarios
- [ ] Implement Kubernetes service discovery
  > **Stub Ready**: K8s discovery source defined in `DiscoverySource::Kubernetes`
  > Will integrate with k8s-openapi when needed
- [x] Test discovery mechanisms
  > **Completed**: 6 DNS discovery tests added, integration with CombinedDiscovery

### Week 12: Push Distribution

> 🔄 **Partially Complete**: Core push logic implemented, CLI pending.

- [x] Implement push scheduler
  > **Completed**: Created `scheduler.rs` with:
  > - `DeploymentScheduler` with max concurrent deployments
  > - `DeploymentPriority` enum: Low, Normal, High, Critical
  > - Queue management with capacity limits
- [x] Add parallel distribution
  > **Completed**: Distributor uses tokio for concurrent pushes:
  > - `deploy_immediate()` pushes to all instances in parallel
  > - `deploy_canary()` validates subset before full rollout
  > - `deploy_rolling()` processes in batches
- [x] Implement acknowledgment handling
  > **Completed**: `PushResult` tracks success/failure per instance:
  > - Attempt counting for retry tracking
  > - Duration measurement for latency monitoring
  > - Error messages preserved for debugging
- [x] Add retry logic with exponential backoff
  > **Completed**: `PolicyPusher::push()` implements:
  > - Configurable max_retries
  > - Retry delay between attempts
  > - `is_retryable()` check for transient errors
- [x] Track distribution status
  > **Completed**: `DeploymentTracker` provides:
  > - Per-instance result tracking
  > - Service-level status aggregation
  > - Active deployment listing
- [x] Add `eunomia push` CLI command
  > **Completed**: Full CLI implementation with:
  > - Immediate, canary, and rolling deployment strategies
  > - Static endpoint discovery (--endpoints flag)
  > - Dry-run mode for deployment preview
  > - JSON output format option
  > - Auto-rollback and max-failures configuration
  > - Converted main.rs to async with tokio runtime

### Phase E3 Milestone

**Criteria**: Control plane is operational, bundles can be pushed to instances

> ✅ **Status**: Phase E3 Complete!
>
> - Week 9: Registry client with OCI support, versioning, caching
> - Week 10: Control plane API design, state tracking, health checks
> - Week 11: Instance discovery with Static and DNS sources, K8s stub ready
> - Week 12: Push distribution with scheduler, parallel deployment, CLI
> - Week 13: CTO review items complete - deprecated types removed
> - Week 14: Documentation & examples complete - authoring guide, examples, migration guide
>
> **Remaining for E4 integration:**
> - gRPC server implementation using tonic (when needed for control plane service)
> - Kubernetes service discovery (when K8s cluster available)

---

## Phase E4: Archimedes Integration (Weeks 17-20)

### Gap Weeks (13-16): Stretch Goals & Support

> **Note**: While Archimedes completes OPA integration, Eunomia team works on:

**Week 13: CTO Review Action Items** ✅ COMPLETE

> **Reference**: [CTO Project Review (2026-01-05)](~/Documents/projects/Startups/ThemisPlatform/docs/reviews/2026-01-05-cto-project-review.md)

- [x] Remove deprecated local type modules from `eunomia-core`:
  - [x] Delete `src/identity.rs` (local CallerIdentity - now using shared)
  - [x] Delete `src/input.rs` (local PolicyInput - now using shared)
  - [x] Delete `src/decision.rs` (local AuthorizationDecision - now using shared)
  - [x] Update `src/lib.rs` to remove deprecated module declarations
  - [x] Ensure all re-exports come from `themis-platform-types`
- [x] Update `eunomia-test` mock builders if affected by module removal
  > **Note**: Mock builders already use themis-platform-types, no changes needed
- [x] Run full test suite to verify no regressions
  > **Completed**: 420+ tests passing across workspace
- [x] Update documentation to remove references to deprecated modules
  > **Completed**: lib.rs docs updated, AuthorizationDecision alias removed

**Week 14: Documentation & Examples** ✅ COMPLETE

- [x] Create comprehensive policy authoring guide
  > **Completed**: Created `docs/policy-authoring-guide.md` with:
  > - Getting started section with CLI installation
  > - Policy structure and naming conventions
  > - Input schema documentation with all caller types
  > - Writing rules with common patterns
  > - Testing, building, signing, and deployment guides
  > - Best practices and troubleshooting sections
  > - Quick reference for CLI commands and input fields
- [x] Build example policy repository with common patterns
  > **Completed**: Added three comprehensive examples:
  > - `examples/policies/rbac-service/` - RBAC with hierarchical roles
  > - `examples/policies/multi-tenant/` - Multi-tenant SaaS with isolation
  > - `examples/policies/api-gateway/` - Scope-based API key authorization
  > - All examples include authz.rego, authz_test.rego, and README.md
- [x] Document testing best practices
  > **Completed**: Enhanced `docs/testing-guide.md` with:
  > - Test role hierarchies (#6)
  > - Test tenant isolation for multi-tenant apps (#7)
  > - Test scope requirements for API keys (#8)
  > - Test time-based constraints (#9)
  > - Comprehensive coverage checklist (#10)
- [x] Create policy migration guide for existing services
  > **Completed**: Created `docs/migration-guide.md` with:
  > - Migration overview with stages diagram
  > - Assessment phase with inventory template
  > - Design phase with input schema mapping
  > - Implementation phase with code translation examples
  > - Testing phase with shadow testing approach
  > - Deployment phase with feature flags and rollback plan
  > - Common migration scenarios (Spring Security, Express.js, IAM-style)
  > - Troubleshooting section and complete checklist

**Week 15-16: Stretch Goals (if time permits)**

- [ ] Design multi-cluster policy distribution (for future)
- [ ] Prototype policy inheritance patterns
- [ ] Research external data integration (IdP roles)
- [ ] Support Archimedes team with OPA integration questions

### Week 17: Push Integration

- [ ] Test bundle push to Archimedes instances
- [ ] Verify mTLS authentication
- [ ] Test hot-reload scenarios
- [ ] Validate signature verification

### Week 18: Rollback Controller

- [ ] Implement rollback triggers
- [ ] Add automatic rollback on health failures
- [ ] Implement `eunomia rollback` CLI command
- [ ] Add rollback audit logging

### Week 19: End-to-End Testing

- [ ] Full authorization flow testing
- [ ] Performance benchmarks
- [ ] Load testing with multiple instances
- [ ] Error scenario testing

### Week 20: Documentation & Polish

- [ ] Update all documentation
- [ ] Add example policy repository
- [ ] Create troubleshooting guide
- [ ] Performance tuning guide
- [ ] Release v1.0.0

### Phase E4 Milestone

**Criteria**: Full Archimedes integration complete, production-ready

---

## Milestones Summary

| Milestone        | Target  | Criteria                                         |
| ---------------- | ------- | ------------------------------------------------ |
| E1: Parsing      | Week 4  | Rego policies parsed correctly                   |
| E2: Testing      | Week 8  | Policy tests execute and report, bundles created |
| E3: Distribution | Week 12 | Control plane operational, push working          |
| E4: Integrated   | Week 20 | Full Archimedes integration complete             |

---

## Deliverables

### CLI Commands

- `eunomia test` - Run policy tests
- `eunomia build` - Compile bundle
- `eunomia sign` - Sign bundle
- `eunomia publish` - Publish to registry
- `eunomia push` - Push to instances
- `eunomia rollback` - Rollback policy
- `eunomia status` - Check deployment status

### Crates

- `eunomia-core` - Core types and traits
- `eunomia-compiler` - Rego parsing and bundle compilation
- `eunomia-test` - Testing framework
- `eunomia-registry` - Bundle registry client
- `eunomia-distributor` - gRPC push distribution
- `eunomia-audit` - Audit logging

### Services

- `eunomia-control-plane` - Central management service

---

## Dependencies on Other Components

| Dependency                 | Required For                | Available   |
| -------------------------- | --------------------------- | ----------- |
| None                       | Core development (E1-E3)    | Immediately |
| Archimedes OPA integration | Push testing (E4)           | Week 17     |
| Themis contracts           | Real operationId validation | Week 12     |

---

## Risk Mitigation

### Technical Risks

1. **OPA Integration Complexity**
   - _Mitigation_: Start with OPA CLI wrapper, migrate to native later
2. **gRPC Performance**

   - _Mitigation_: Benchmark early, optimize as needed

3. **Bundle Size**
   - _Mitigation_: Implement compression, monitor bundle sizes

### Schedule Risks

1. **Archimedes Delays**

   - _Mitigation_: Can mock Archimedes endpoints for push testing

2. **OPA Updates**
   - _Mitigation_: Pin OPA version, test upgrades separately
