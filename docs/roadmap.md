# Eunomia – Development Roadmap

> **Version**: 1.0.0  
> **Created**: 2026-01-04  
> **Target Completion**: Week 20 (MVP)

---

## Overview

Eunomia is the authorization policy platform for the Themis ecosystem. Development runs **in parallel with Themis** during the first 12 weeks, then integrates with Archimedes.

### Timeline Summary

| Phase                       | Duration | Weeks | Description                               |
| --------------------------- | -------- | ----- | ----------------------------------------- |
| E1: Foundation              | 4 weeks  | 1-4   | Core types, Rego parsing, validation      |
| E2: Testing Framework       | 4 weeks  | 5-8   | Test runner, fixtures, bundle compilation |
| E3: Registry & Distribution | 4 weeks  | 9-12  | Registry client, push distribution        |
| E4: Archimedes Integration  | 4 weeks  | 17-20 | Full integration with Archimedes OPA      |

**Total**: ~16 weeks of active development (with gap for Archimedes catch-up)

---

## Phase E1: Foundation (Weeks 1-4)

### Week 1: Project Setup

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
- [x] Write initial documentation
  > **Completed 2026-01-04**: Added rustdoc for all public APIs, README for eunomia-core

### Week 2: Core Types

- [ ] Implement `Policy` model
- [ ] Implement `Bundle` model
- [ ] Implement `AuthorizationDecision` type
- [ ] Implement `PolicyInput` schema
- [ ] Implement `CallerIdentity` types (SPIFFE, User, ApiKey, Anonymous)
- [ ] Write serialization tests

### Week 3: Rego Parsing

- [ ] Integrate Rego parser (OPA library or WASM)
- [ ] Implement policy file loading
- [ ] Add static analysis for common errors
- [ ] Validate policy structure
- [ ] Test with sample policies
- [ ] Document policy file conventions

### Week 4: Policy Validation

- [ ] Implement policy syntax checking
- [ ] Add semantic validation
- [ ] Create mock operationId support (for testing without Themis)
- [ ] Implement validation error reporting
- [ ] Add structured error messages

### Phase E1 Milestone

**Criteria**: Rego policies can be parsed, validated, and loaded

---

## Phase E2: Testing Framework (Weeks 5-8)

### Week 5: Test Runner

- [ ] Implement test case discovery
- [ ] Parse `*_test.rego` files
- [ ] Execute OPA eval for tests
- [ ] Collect pass/fail results
- [ ] Generate test report (console output)
- [ ] Add `eunomia test` CLI command

### Week 6: Test Fixtures

- [ ] Implement fixture loading from JSON/YAML
- [ ] Support data files for policies
- [ ] Add mock identity helpers
- [ ] Create test utilities library
- [ ] Document testing patterns
- [ ] Add example test files

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

_Note: Weeks 13-16 are gap time while Archimedes completes its OPA integration_

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
