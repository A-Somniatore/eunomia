# Multi-Tenant Authorization Example

Authorization policies for multi-tenant SaaS applications.

## Structure

```
multi-tenant/
├── eunomia.toml          # Bundle configuration
├── authz.rego            # Main authorization policy
├── tenant.rego           # Tenant isolation rules
├── data/
│   └── permissions.json  # Permission definitions
└── tests/
    └── authz_test.rego   # Policy tests
```

## Quick Start

```bash
# Validate policies
eunomia validate .

# Run tests
eunomia test .

# Compile bundle
eunomia compile . -o multi-tenant.bundle
```

## Policy Features

### Tenant Isolation

Each request must include a tenant context:

```json
{
  "caller": {
    "type": "user",
    "id": "user-123",
    "tenant_id": "tenant-abc",
    "roles": ["editor"]
  },
  "resource": {
    "path": "/api/documents/456",
    "tenant_id": "tenant-abc"
  },
  "action": "read"
}
```

### Key Rules

1. **Tenant Boundary**: Users can only access resources in their tenant
2. **Super Admin**: Platform admins can cross tenant boundaries
3. **Tenant Admin**: Tenant-level admins have full access within their tenant

## Customization

### Adding Custom Tenant Roles

Edit `data/permissions.json`:

```json
{
  "tenant_roles": {
    "custom-role": {
      "permissions": [{ "action": "read", "resource": "/api/custom/**" }]
    }
  }
}
```
