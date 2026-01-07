# Microservices Authorization Example

Service-to-service authorization policies for microservices architectures.

## Structure

```
microservices/
├── eunomia.toml              # Bundle configuration
├── authz.rego                # Main authorization policy
├── service_mesh.rego         # Service mesh rules
├── data/
│   └── service_registry.json # Service permissions
└── tests/
    └── authz_test.rego       # Policy tests
```

## Quick Start

```bash
# Validate policies
eunomia validate .

# Run tests
eunomia test .

# Compile bundle
eunomia compile . -o microservices.bundle
```

## Policy Features

### Service Identity

Each service has an identity with allowed dependencies:

```json
{
  "caller": {
    "type": "service",
    "id": "orders-service",
    "namespace": "production"
  },
  "target": {
    "service": "users-service",
    "method": "GetUser"
  }
}
```

### Key Rules

1. **Service Allowlist**: Services can only call explicitly allowed dependencies
2. **Namespace Isolation**: Production services cannot call staging services
3. **Method-Level Control**: Fine-grained control over allowed methods
4. **mTLS Verification**: Optional certificate-based identity verification

## Integration with Service Mesh

Use with Istio, Linkerd, or other service meshes by integrating with Archimedes as an external authorization service.
