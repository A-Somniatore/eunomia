# Eunomia

**Authorization Policy Platform for the Themis Ecosystem**

Eunomia is a Git-backed OPA/Rego policy management system that provides:

- 📝 **Policy Authoring** – Write policies in Rego with IDE support
- ✅ **Policy Testing** – Comprehensive testing framework for policies
- 📦 **Bundle Compilation** – Compile and sign OPA bundles
- 🚀 **Policy Distribution** – Push policies to Archimedes services
- 📊 **Audit Logging** – Track all authorization decisions
- 🔐 **Contract Integration** – Derive policies from Themis contracts

## Quick Links

- [Design Document](docs/design.md)
- [Specification](docs/spec.md)
- [Roadmap](docs/roadmap.md)
- [Contributing](CONTRIBUTING.md)
- [Integration Specification](../docs/integration/integration-spec.md) – Shared schemas with Archimedes/Themis
- [Themis Platform](../)

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

## Key Features

### Policy Management

- Git as source of truth for all policies
- Branch-based policy environments (dev/staging/prod)
- Policy versioning and rollback support

### Testing Framework

- Unit tests for individual policy rules
- Integration tests with mock data
- Coverage reporting for policies
- Decision replay testing

### Bundle Distribution

- Hybrid push/pull policy distribution via gRPC
- Signed bundles for integrity verification
- Atomic updates across service clusters
- Local caching for resilient operation

### Observability

- Audit logging for all authorization decisions
- Policy evaluation metrics
- Decision latency tracking
- Compliance reporting

## Project Structure (Planned)

```
eunomia/
├── .github/
│   └── copilot-instructions.md
├── docs/
│   ├── design.md                 # Implementation design
│   ├── spec.md                   # Specification
│   └── roadmap.md                # Development roadmap
├── crates/                       # (when code is added)
│   ├── eunomia-core/             # Core types and traits
│   ├── eunomia-compiler/         # Rego parsing and bundle compilation
│   ├── eunomia-test/             # Policy testing framework
│   ├── eunomia-registry/         # Bundle registry client
│   ├── eunomia-distributor/      # gRPC push distribution
│   └── eunomia-audit/            # Audit logging
├── eunomia-cli/                  # CLI tool
├── eunomia-control-plane/        # Control plane service
├── tests/                        # Integration tests
├── examples/                     # Example policies
├── README.md
└── CONTRIBUTING.md
```

## Related Projects

- **[Themis](../themis/)** – Contract validation and code generation
- **[Archimedes](../docs/components/archimedes-design.md)** – HTTP/gRPC server framework
- **[Stoa](../docs/components/stoa-design.md)** – Web UI for service governance

## License

Apache License 2.0 - See [ADR-007](../docs/decisions/007-apache-2-license.md) for rationale.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development guidelines.
