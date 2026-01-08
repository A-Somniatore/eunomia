# Week 21 Planning: Pre-Release Hardening

> **Date**: 2026-01-07  
> **Phase**: E4: Archimedes Integration  
> **Target**: v1.0.0 Release Candidate  
> **Total Effort**: ~36 hours of focused development

---

## Executive Summary

Week 21 completes all critical requirements before v1.0.0 release. This phase focuses on:

1. **Security Hardening** - Audit bundle signing, add rate limiting, verify mTLS
2. **Observability** - Add OpenTelemetry metrics across all crates
3. **Operational Excellence** - Kubernetes discovery, cross-platform CI, pre-release checks

All changes follow strict TDD, documentation, and commit practices defined in copilot-instructions.md.

---

## Development Practices (MANDATORY)

Every single change MUST follow these practices:

### 1. Test-Driven Development
- ✅ Write failing test FIRST
- ✅ Implement feature to make test pass
- ✅ Refactor while keeping tests green
- ✅ Run `cargo test` before every commit

### 2. Documentation
- ✅ Add rustdoc to all public types/functions
- ✅ Update design.md for architectural changes
- ✅ Update roadmap.md for status changes
- ✅ Add examples to key features

### 3. Code Quality
- ✅ Run `cargo fmt` before commit
- ✅ Run `cargo clippy -- -D warnings` before commit
- ✅ Run `cargo doc --no-deps` to ensure docs build
- ✅ No clippy warnings allowed

### 4. Git Discipline
- ✅ Small, focused commits (not all 36 hours in one commit!)
- ✅ Commit message format: `type(scope): description`
- ✅ Push at least at end of each work session
- ✅ Verify each commit builds: `cargo check`

---

## Critical Design Decisions

### Decision 1: Metrics Architecture

**Choice**: Create new `eunomia-metrics` crate

**Rationale**:
- Keeps concerns separated (audit ≠ metrics)
- Allows metrics to evolve independently
- Clear dependency: other crates depend on metrics, not vice versa
- Easier to add different exporters (Prometheus, OTLP, custom)

**Structure**:
```
eunomia-metrics/
├── src/
│   ├── lib.rs          # Public API
│   ├── registry.rs     # Metric registry
│   ├── metrics.rs      # Metric definitions
│   ├── exporter.rs     # Prometheus/OTLP exporter
│   └── middleware.rs   # gRPC middleware
```

### Decision 2: Rate Limiting Strategy

**Choice**: Use tower-governor middleware

**Rationale**:
- Tower ecosystem is Tokio-native
- Per-endpoint configuration
- Integrates cleanly with tonic gRPC
- Proven in production use

**Configuration**:
```yaml
rate_limits:
  push:
    requests_per_second: 100
  rollback:
    requests_per_second: 10
  status:
    requests_per_second: 1000  # Status updates are cheap
```

### Decision 3: Kubernetes Discovery Implementation

**Choice**: Use k8s-openapi + tokio watch API

**Rationale**:
- Native Kubernetes support
- Works with any K8s cluster (no external dependencies)
- Async-native with tokio
- Small binary footprint

**Sources**:
- Service annotations: `eunomia.themis.io/enable-push`
- Pod labels: Automatic discovery of Archimedes instances
- DNS fallback: If K8s discovery fails

---

## Task Breakdown & Effort

### Security Requirements (10 hrs)

| Task | Effort | Tests Required | Docs Required |
|------|--------|---|---|
| Bundle signing audit | 4 hrs | Ed25519 verification tests | design.md section |
| Rate limiting | 4 hrs | Rate limit enforcement tests | deployment-guide update |
| mTLS verification | 2 hrs | Client cert validation tests | troubleshooting-guide update |

### Observability Requirements (12 hrs)

| Task | Effort | Tests Required | Docs Required |
|------|--------|---|---|
| Metrics crate setup | 2 hrs | Registry tests | metrics/README |
| Compiler instrumentation | 2 hrs | Metric emission tests | design.md §13 |
| Registry instrumentation | 2 hrs | Metric emission tests | design.md §13 |
| Distributor instrumentation | 2 hrs | Metric emission tests | design.md §13 |
| Dashboard templates | 2 hrs | None (config files) | performance-guide update |
| OTLP exporter config | 2 hrs | Integration tests | deployment-guide update |

### Operational Requirements (10 hrs)

| Task | Effort | Tests Required | Docs Required |
|------|--------|---|---|
| K8s discovery | 8 hrs | Mock K8s API tests | design.md §7.2 |
| Cross-platform CI | 2 hrs | CI runs on all platforms | .github/workflows |

### Pre-Release (4 hrs)

| Task | Effort | Tests Required | Docs Required |
|------|--------|---|---|
| Final checklist | 2 hrs | All tests pass on CI | CHANGELOG.md |
| Release prep | 2 hrs | Tag v1.0.0-rc.1 | Release notes |

**Total: 36 hours**

---

## Metrics Definition (from spec.md §10.2)

All these metrics MUST be emitted:

```rust
// Counters (monotonically increasing)
eunomia_policy_deployments_total{service="X", status="success|failure"}
eunomia_policy_rollbacks_total{service="X", reason="..."}
eunomia_policy_tests_total{service="X", status="pass|fail"}
eunomia_authorization_decisions_total{service="X", operation_id="Y", decision="allow|deny"}

// Histograms (track distributions)
eunomia_bundle_compilation_duration_seconds{service="X"}
eunomia_authorization_evaluation_duration_seconds{service="X"}

// Gauges (point-in-time)
eunomia_policy_coverage_percentage{service="X"}
```

---

## Security Audit Checklist

### Bundle Signing (from design.md §14.1)

- [ ] Ed25519 key generation uses cryptographically secure RNG
- [ ] Signature verification fails on tampered bundles
- [ ] Key ID validation prevents key confusion attacks
- [ ] Signature algorithm is hardcoded (no algorithm substitution)
- [ ] Tests: Valid signature, invalid signature, missing key, wrong key

### Rate Limiting

- [ ] Per-endpoint limits are enforced
- [ ] Rate limit exceeded returns proper error
- [ ] Limit is per-caller (SPIFFE identity)
- [ ] Tests: Under limit, at limit, over limit, multiple clients

### mTLS Verification

- [ ] Valid client cert is accepted
- [ ] Invalid client cert is rejected
- [ ] Expired cert is rejected
- [ ] Self-signed cert is rejected (unless trusted)
- [ ] Tests: Valid, invalid, expired, self-signed certs

---

## Implementation Order

1. **Start with eunomia-metrics crate** (foundation for observability)
2. **Wire metrics to compiler** (early feedback on metrics design)
3. **Wire metrics to registry & distributor**
4. **Add rate limiting** (security)
5. **Add bundle signing audit** (security)
6. **Add mTLS tests** (security)
7. **Implement K8s discovery** (operational)
8. **Set up cross-platform CI** (operational)
9. **Pre-release checklist & tag v1.0.0-rc.1**

---

## Success Criteria

✅ All tests passing on all platforms (Linux, macOS, Windows)  
✅ All metrics emitted and scrape-able by Prometheus  
✅ Rate limiting enforced on all gRPC endpoints  
✅ Bundle signing verified in security audit  
✅ mTLS working with real certificates  
✅ K8s service discovery working  
✅ No clippy warnings  
✅ Documentation complete and builds  
✅ CHANGELOG.md prepared  
✅ v1.0.0-rc.1 tag created  

---

## Risk Mitigations

| Risk | Mitigation |
|------|-----------|
| Metrics causing performance regression | Benchmark metrics setup, add to perf-guide.md |
| Rate limiting breaks existing clients | Start with high limits, document in deployment-guide |
| K8s discovery not working in test env | Provide static endpoint fallback |
| Cross-platform CI takes too long | Use matrix build with fail-fast strategy |

---

## References

- [copilot-instructions.md](../.github/copilot-instructions.md) - Development practices
- [design.md](design.md) §13 - Observability architecture
- [design.md](design.md) §14 - Security model
- [spec.md](spec.md) §10-11 - Observable and security requirements
- [ARCHITECTURE.md](ARCHITECTURE.md) - Pre-v1.0.0 requirements

---

Generated: 2026-01-07  
Phase: E4 - Archimedes Integration  
Status: Ready to implement
