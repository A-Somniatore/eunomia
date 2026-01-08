# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.0.0] - 2026-01-08

### Added

#### Core Platform
- **Policy Compilation**: Full OPA/Rego policy compilation with `regorus` engine
- **Bundle Format**: OPA-compatible tar.gz bundles with `.manifest` JSON metadata
- **Bundle Signing**: Ed25519 digital signatures for policy bundles
- **Signature Verification**: Multi-signature support with key ID validation

#### CLI Commands
- `eunomia test` - Run policy tests with fixtures and assertions
- `eunomia build` - Compile Rego policies into OPA bundles
- `eunomia sign` - Sign bundles with Ed25519 keys
- `eunomia publish` - Publish bundles to OCI registry
- `eunomia fetch` - Fetch bundles from registry with semantic versioning
- `eunomia validate` - Validate policy syntax and structure
- `eunomia push` - Push bundles to Archimedes instances
- `eunomia status` - Check deployment status across instances
- `eunomia rollback` - Rollback to previous policy version

#### Distribution
- **Push Distribution**: gRPC-based push to Archimedes instances
- **Deployment Strategies**: Immediate, rolling, and canary deployments
- **Rollback Controller**: Automatic rollback on health check failures
- **Version History**: Track deployment history per service

#### Instance Discovery
- **Static Discovery**: Predefined endpoint list
- **DNS Discovery**: DNS-based service resolution with hickory-resolver
- **Kubernetes Discovery**: K8s Endpoints API with namespace/label filtering

#### Registry Integration
- **OCI Registry**: Full OCI distribution spec compliance
- **Semantic Versioning**: Version resolution with `~`, `^`, `>=` constraints
- **Local Caching**: LRU bundle cache with TTL eviction

#### Security
- **mTLS Support**: Mutual TLS for server and client authentication
- **Rate Limiting**: Token bucket rate limiter for gRPC endpoints
- **Bundle Integrity**: SHA-256 checksums and signature verification
- **Audit Logging**: Security events tracked with structured audit logs

#### Observability
- **OpenTelemetry Metrics**: Full instrumentation of compiler, registry, and distributor
- **Grafana Dashboards**: Pre-built dashboards for monitoring
- **Prometheus Export**: Metrics endpoint for scraping
- **Health Checks**: Instance health tracking with failure detection

#### Documentation
- Production deployment guide with Kubernetes manifests
- Troubleshooting runbook with error resolution steps
- Performance tuning guide with SLOs and recommendations
- Example policies (RBAC, multi-tenant, microservices)

### Changed
- Uses `themis-platform-types` for all shared types (`PolicyInput`, `CallerIdentity`, etc.)

### Security
- Security audit of bundle signing completed
- Rate limiting protects against resource exhaustion
- mTLS verification testing with 24 integration tests
- Cross-platform CI ensures consistent behavior

### Performance
- Bundle compilation: < 100ms for typical bundles
- Push latency: < 50ms P99 per instance
- Memory-efficient streaming for large bundles

### Crates
- `eunomia-core` - Core types and traits
- `eunomia-compiler` - Rego parsing and bundle compilation
- `eunomia-test` - Testing framework
- `eunomia-registry` - Bundle registry client
- `eunomia-distributor` - gRPC push distribution
- `eunomia-audit` - Audit logging
- `eunomia-metrics` - OpenTelemetry instrumentation
- `eunomia-cli` - Command-line interface

### Testing
- 560+ unit tests across all crates
- 116 integration tests
- 15 security tests for bundle signing
- 24 mTLS integration tests
- 20 rate limiting tests
- Cross-platform CI (Linux, macOS, Windows)
- MSRV compatibility check (Rust 1.75.0)

## [0.1.0] - 2026-01-04

### Added
- Initial project structure
- Integration with `themis-platform-types`
- Basic Rego parsing proof of concept

[Unreleased]: https://github.com/A-Somniatore/eunomia/compare/v1.0.0...HEAD
[1.0.0]: https://github.com/A-Somniatore/eunomia/compare/v0.1.0...v1.0.0
[0.1.0]: https://github.com/A-Somniatore/eunomia/releases/tag/v0.1.0
