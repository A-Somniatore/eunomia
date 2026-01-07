# Eunomia

**Authorization Policy Platform for the Themis Ecosystem**

[![CI](https://github.com/A-Somniatore/eunomia/workflows/CI/badge.svg)](https://github.com/A-Somniatore/eunomia/actions)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)

Eunomia is a Git-backed OPA/Rego policy management system that provides:

- 📝 **Policy Authoring** – Write policies in Rego with IDE support
- ✅ **Policy Testing** – Comprehensive testing framework for policies
- 📦 **Bundle Compilation** – Compile and sign OPA bundles
- 🚀 **Policy Distribution** – Push policies to Archimedes services
- 📊 **Audit Logging** – Track all policy changes and deployments
- 🔄 **Rollback Support** – Automatic and manual rollback capabilities

## Installation

```bash
# Build from source
cargo build --release

# Install CLI
cargo install --path crates/eunomia-cli
```

## Quick Start

```bash
# Validate policies
eunomia validate policies/

# Run policy tests
eunomia test policies/

# Build and sign a bundle
eunomia build --service users-service --version 1.0.0 --output bundle.tar.gz
eunomia sign bundle.tar.gz --generate-key

# Push to Archimedes instances
eunomia push bundle.tar.gz --endpoints http://localhost:8080
```

## Documentation

| Document | Description |
|----------|-------------|
| [Design Document](docs/design.md) | Architecture and implementation details |
| [Specification](docs/spec.md) | Requirements and policy conventions |
| [Roadmap](docs/roadmap.md) | Development timeline and status |
| [Policy Authoring Guide](docs/policy-authoring-guide.md) | How to write Rego policies |
| [Testing Guide](docs/testing-guide.md) | Policy testing best practices |
| [Migration Guide](docs/migration-guide.md) | Migrating from other auth systems |
| [Architecture Review](docs/ARCHITECTURE.md) | Code quality and recommendations |
| [Contributing](CONTRIBUTING.md) | Development guidelines |

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                        Eunomia System                            │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐       │
│  │   Git Repo   │───▶│   Compiler   │───▶│   Registry   │       │
│  │  (Policies)  │    │   (Bundles)  │    │  (Storage)   │       │
│  └──────────────┘    └──────────────┘    └──────────────┘       │
│                                                 │                │
│                                                 ▼                │
│                      ┌──────────────────────────────────┐       │
│                      │         Control Plane            │       │
│                      │   (Coordination & Distribution)  │       │
│                      └──────────────────────────────────┘       │
│                                    │                             │
│                    ┌───────────────┼───────────────┐            │
│                    ▼               ▼               ▼            │
│              ┌──────────┐   ┌──────────┐   ┌──────────┐        │
│              │Archimedes│   │Archimedes│   │Archimedes│        │
│              │Service A │   │Service B │   │Service C │        │
│              └──────────┘   └──────────┘   └──────────┘        │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

## CLI Commands

| Command | Description |
|---------|-------------|
| `eunomia validate <path>` | Validate Rego policy syntax and lint rules |
| `eunomia test <path>` | Run policy tests (`*_test.rego` files) |
| `eunomia build` | Compile policies into OPA bundle |
| `eunomia sign <bundle>` | Sign bundle with Ed25519 key |
| `eunomia publish <bundle>` | Publish bundle to OCI registry |
| `eunomia fetch <service>` | Fetch bundle from registry |
| `eunomia push <bundle>` | Push bundle to Archimedes instances |
| `eunomia status` | Check deployment status |
| `eunomia rollback` | Rollback to previous policy version |

## Key Features

### Policy Management

- Git as source of truth for all policies
- Branch-based policy environments (dev/staging/prod)
- Policy versioning and rollback support
- Semantic versioning for bundles

### Testing Framework

- Native Rego tests (`*_test.rego`)
- JSON/YAML fixture-based tests
- Mock identity builders for common scenarios
- Coverage analysis and reporting

### Bundle Distribution

- Hybrid push/pull policy distribution via gRPC
- Ed25519 signed bundles for integrity verification
- Atomic updates with automatic rollback
- Local caching for resilient operation

### Observability

- Audit logging for policy changes and deployments
- Policy evaluation metrics
- Deployment state tracking
- Health monitoring for instances

## Project Structure

```
eunomia/
├── crates/
│   ├── eunomia-core/             # Core types, bundle model, signing
│   ├── eunomia-compiler/         # Rego parsing, validation, bundling
│   ├── eunomia-test/             # Policy testing framework
│   ├── eunomia-registry/         # OCI registry client, caching
│   ├── eunomia-distributor/      # gRPC distribution, rollback
│   ├── eunomia-audit/            # Audit logging
│   └── eunomia-cli/              # CLI application
├── docs/
│   ├── design.md                 # Implementation design
│   ├── spec.md                   # Specification
│   ├── roadmap.md                # Development timeline
│   ├── policy-authoring-guide.md # Writing policies
│   ├── testing-guide.md          # Testing best practices
│   └── migration-guide.md        # Migration from other systems
├── examples/
│   └── policies/                 # Example policy patterns
├── proto/
│   └── control_plane.proto       # gRPC API definitions
└── tests/                        # Integration tests
```

## Crate Overview

| Crate | Description |
|-------|-------------|
| `eunomia-core` | Core types (`Bundle`, `Policy`), signing (`BundleSigner`/`BundleVerifier`), shared types from `themis-platform-types` |
| `eunomia-compiler` | Rego parsing via `regorus`, policy validation, linting, semantic analysis, bundle compilation |
| `eunomia-test` | Test discovery, runner, fixtures, mock identities, coverage analysis |
| `eunomia-registry` | OCI registry client, version resolution, LRU caching |
| `eunomia-distributor` | gRPC control plane, instance discovery, deployment strategies, rollback controller |
| `eunomia-audit` | Audit event types, logging backends, structured event emission |
| `eunomia-cli` | Command-line interface for all operations |

## Related Projects

- **[Themis](../themis/)** – Contract validation and code generation
- **[Archimedes](../archimedes/)** – HTTP/gRPC server framework with embedded OPA
- **[themis-platform-types](https://github.com/A-Somniatore/themis-platform-types)** – Shared types for policy evaluation

## Development

```bash
# Run all tests
cargo test

# Run with clippy
cargo clippy -- -D warnings

# Format code
cargo fmt

# Build documentation
cargo doc --no-deps --open
```

## License

Apache License 2.0 - See [LICENSE](LICENSE) for details.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development guidelines.
