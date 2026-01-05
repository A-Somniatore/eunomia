# Eunomia – Development Roadmap

> **Version**: 1.2.0  
> **Created**: 2026-01-04  
> **Last Updated**: 2026-01-05  
> **Target Completion**: Week 20 (MVP Integration with Archimedes MVP)

---

## Key Decisions

| Decision                                                        | Impact                                              |
| --------------------------------------------------------------- | --------------------------------------------------- |
| [ADR-008](../../docs/decisions/008-archimedes-full-framework.md)| Archimedes is full framework replacement (40 weeks) |
| [ADR-004](../../docs/decisions/004-regorus-for-rego-parsing.md) | Use Regorus for Rego parsing and evaluation         |
| [ADR-002](../../docs/decisions/002-opa-for-authorization.md)    | OPA/Rego as the policy language                     |
| [ADR-003](../../docs/decisions/003-push-based-policies.md)      | Hybrid push/pull policy distribution                |
| [ADR-007](../../docs/decisions/007-apache-2-license.md)         | Apache 2.0 license                                  |

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

## Phase E0: Shared Platform Types (Week 1) ⭐ COORDINATION

> **Note**: The `themis-platform-types` crate is created by Themis team in Week 1.
> Eunomia integrates it to ensure schema compatibility.

### Week 1: Integrate Shared Types

> ✅ **Update (2026-01-04)**: `themis-platform-types` crate is now available!
> Local type implementations in `eunomia-core` are ready for migration.

- [ ] Add `themis-platform-types` dependency to `eunomia-core`
  > ⏳ **Ready**: Shared crate available, migration pending
- [ ] Migrate `CallerIdentity` to use shared definition
  > ⏳ **Ready**: Local implementation matches spec, ready for migration
- [ ] Migrate `PolicyInput` to use shared definition
  > ⏳ **Ready**: Local implementation matches spec, ready for migration
- [ ] Migrate `PolicyDecision` to use shared definition
  > ⏳ **Ready**: Local implementation matches spec, ready for migration
- [ ] Update existing code to use shared types
- [ ] Verify JSON serialization matches spec

### Phase E0 Milestone

**Criteria**: Eunomia uses `themis-platform-types` for all shared types

> ⏳ **Status**: Local implementations complete. Migration to shared crate pending.

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
  > - Deep policy analysis beyond syntax checking
  > - `SemanticIssue`, `SemanticSeverity`, `SemanticCategory` types
  > - Operation ID validation against service contracts
  > - Unused rule detection to identify dead code
  > - Deprecated input field detection
  > - Rule reference checking
- [x] Create mock operationId support (for testing without Themis)
  > **Completed**: Created `MockServiceContract`:
  > - Define service contracts with operation IDs
  > - Validate policies against expected operations
  > - Predefined contracts: `users_service_contract()`, `orders_service_contract()`
  > - `InputSchema` for validating authorization input structure
- [x] Implement validation error reporting
  > **Completed**: `SemanticIssue` provides structured reporting with:
  > - Severity levels (Error, Warning, Info)
  > - Category classification (OperationId, Unused, Deprecated, Schema, Reference)
  > - Rule names, descriptions, and suggestions
- [x] Add structured error messages
  > **Completed**: All validation errors include:
  > - Clear descriptions of what's wrong
  > - Context about affected policies/rules
  > - Suggestions for how to fix issues

### Phase E1 Milestone

**Criteria**: Rego policies can be parsed, validated, and loaded

> ✅ **Status**: Phase E1 Complete!
> - Week 2 complete: Project setup, core types, validation framework
> - Week 3 complete: Rego parsing with regorus, linting rules
> - Week 4 complete: Semantic validation with mock contracts

---

## Phase E2: Testing Framework (Weeks 5-8)

### Week 5: Test Runner

- [x] Implement test case discovery
  > **Completed**: Created `TestDiscovery` with:
  > - Recursive directory scanning for `*_test.rego` files
  > - Package name extraction from test files
  > - Test rule detection (`test_*` prefixed rules)
  > - Associated policy file discovery
  > - Fixture file discovery (JSON/YAML)
  > - `DiscoveryConfig` for customization (patterns, recursion)
- [x] Parse `*_test.rego` files
  > **Completed**: `TestDiscovery` parses test files to extract:
  > - Package names
  > - Test rule names
  > - Source file paths
  > - Associates tests with `DiscoveredTest` and `TestSuite`
- [x] Execute OPA eval for tests
  > **Completed**: `TestRunner::run_suite()` executes tests using `RegoEngine`:
  > - Loads policy files into engine
  > - Evaluates each test rule
  > - Reports pass/fail with detailed errors
  > - Supports fail-fast mode
  > - Handles native Rego tests (self-contained)
- [x] Collect pass/fail results
  > **Completed**: `TestResults` aggregates test outcomes:
  > - `passed()` / `failed()` counts
  > - `all_passed()` check
  > - `failures()` iterator for detailed errors
  > - Duration tracking per test
- [x] Generate test report (console output)
  > **Completed**: `ConsoleReporter` with:
  > - Color-coded pass/fail output
  > - Test duration display
  > - Error message formatting
  > - Summary with pass/fail counts
- [x] Add `eunomia test` CLI command
  > **Completed**: Full CLI implementation with:
  > - Test discovery from directory
  > - Fail-fast mode (`-f`)
  > - Parallel execution flag (`-p`)
  > - Verbose output (`-v`)
  > - No-color mode (`--no-color`)
  > - Filter option (`--filter`)
  > - Note: Native Rego tests with imports require loading dependencies (Week 6)

### Known Limitation (Week 5)

The test runner currently evaluates test files in isolation. Tests that use `import` statements
(like `import data.authz`) will fail because the imported modules aren't loaded. This will be
addressed in Week 6 with policy data file support and proper dependency loading.

### Week 6: Test Fixtures & Import Resolution

> **Design Decisions**: See [design.md Section 11.0](design.md#110-design-decisions)

- [ ] **Fix import resolution** - Load all `.rego` files so imports work
  > Load all policy files into single RegoEngine before test execution
- [ ] Implement fixture loading from JSON/YAML
  > Support `*_fixtures.json` and `*_fixtures.yaml` files
- [ ] Support data files for policies (`data.json`)
  > Load static data files into policy evaluation context
- [ ] Add mock identity helpers (`MockIdentity` builder)
  > Convenience builders for user, SPIFFE, and API key identities
- [ ] Create test utilities library
  > Common assertions, input builders, result matchers
- [ ] Document testing patterns
  > Add testing guide to docs/
- [ ] Add example test files with fixtures
  > Working examples using fixture-based testing

### Week 7: Bundle Compilation

- [ ] Implement OPA bundle compilation
- [ ] Generate bundle manifest
- [ ] Include policy data files
- [ ] Add metadata (version, timestamp, git commit)
- [ ] Create tar.gz bundle format
- [ ] Test bundle structure

### Week 8: Bundle Signing & CLI

- [ ] Implement Ed25519 signing
- [ ] Generate bundle signature
- [ ] Add public key management
- [ ] Implement `eunomia build` command
- [ ] Add `eunomia sign` command
- [ ] Write signing documentation

### Phase E2 Milestone

**Criteria**: Policy tests run and report results, signed bundles are created

---

## Phase E3: Registry & Distribution (Weeks 9-12)

### Week 9: Registry Client

- [ ] Design registry API (OCI-compatible)
- [ ] Implement bundle registry client
- [ ] Add publish functionality
- [ ] Add fetch functionality
- [ ] Implement versioning support
- [ ] Add caching layer

### Week 10: Control Plane API

- [ ] Design gRPC API (protobuf)
- [ ] Implement bundle management endpoints
- [ ] Add deployment state tracking
- [ ] Implement health checks
- [ ] Create control plane service scaffold

### Week 11: Instance Discovery

- [ ] Implement Kubernetes service discovery
- [ ] Track Archimedes instances
- [ ] Add instance health monitoring
- [ ] Implement instance grouping
- [ ] Test discovery mechanisms

### Week 12: Push Distribution

- [ ] Implement push scheduler
- [ ] Add parallel distribution
- [ ] Implement acknowledgment handling
- [ ] Add retry logic with exponential backoff
- [ ] Track distribution status
- [ ] Add `eunomia push` CLI command

### Phase E3 Milestone

**Criteria**: Control plane is operational, bundles can be pushed to instances

---

## Phase E4: Archimedes Integration (Weeks 17-20)

### Gap Weeks (13-16): Stretch Goals & Support

> **Note**: While Archimedes completes OPA integration, Eunomia team works on:

**Week 13-14: Documentation & Examples**

- [ ] Create comprehensive policy authoring guide
- [ ] Build example policy repository with common patterns
- [ ] Document testing best practices
- [ ] Create policy migration guide for existing services

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
