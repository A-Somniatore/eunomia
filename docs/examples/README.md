# Eunomia Policy Examples

This directory contains example policy structures for common authorization patterns in the Themis ecosystem.

## Directory Structure

```
examples/
├── basic-rbac/           # Simple role-based access control
├── multi-tenant/         # Multi-tenant SaaS authorization
├── api-gateway/          # API gateway policy patterns
└── microservices/        # Service-to-service authorization
```

## Getting Started

### 1. Choose a Template

Select an example that matches your use case:

| Example          | Description               | Use Case                    |
| ---------------- | ------------------------- | --------------------------- |
| `basic-rbac/`    | Role-based access control | Simple permission systems   |
| `multi-tenant/`  | Tenant-isolated policies  | SaaS applications           |
| `api-gateway/`   | Gateway authorization     | API rate limiting, routing  |
| `microservices/` | Service mesh policies     | Inter-service communication |

### 2. Copy and Customize

```bash
# Copy an example to your project
cp -r examples/basic-rbac my-policies/

# Customize the policies
cd my-policies/
vim authz.rego
```

### 3. Validate and Test

```bash
# Validate policy syntax
eunomia validate my-policies/

# Run policy tests
eunomia test my-policies/

# Compile bundle
eunomia compile my-policies/ -o my-service.bundle
```

### 4. Deploy

```bash
# Publish to registry
eunomia publish my-service.bundle --registry https://registry.example.com

# Push to instances
eunomia push --service my-service --version v1.0.0 --endpoints host1:50051,host2:50051
```

## Policy Structure

Every policy repository should follow this structure:

```
my-policies/
├── eunomia.toml          # Project configuration
├── authz.rego            # Main authorization policy
├── data/                 # Static data files
│   └── roles.json        # Role definitions
├── lib/                  # Reusable policy modules
│   └── helpers.rego      # Helper functions
└── tests/                # Policy tests
    ├── authz_test.rego   # Rego test cases
    └── fixtures/         # Test data
        └── users.json    # Sample user data
```

## Configuration File

The `eunomia.toml` file configures the policy bundle:

```toml
[bundle]
name = "my-service"
version = "1.0.0"
description = "Authorization policies for my service"

[bundle.roots]
authz = "authz.rego"

[bundle.data]
roles = "data/roles.json"

[build]
target = "rego"
optimize = true

[distribution]
strategy = "rolling"
```

## Writing Tests

Policy tests use Rego's built-in testing framework:

```rego
# tests/authz_test.rego
package authz_test

import data.authz

# Test admin can access all resources
test_admin_allowed {
    authz.allow with input as {
        "caller": {"type": "user", "roles": ["admin"]},
        "action": "read",
        "resource": "/api/users"
    }
}

# Test unauthorized user is denied
test_no_role_denied {
    not authz.allow with input as {
        "caller": {"type": "user", "roles": []},
        "action": "write",
        "resource": "/api/admin"
    }
}
```

Run tests with:

```bash
eunomia test my-policies/
```

## Common Patterns

### Pattern 1: Default Deny

Always start with a default deny rule:

```rego
package authz

default allow = false

allow {
    # Explicit allow rules
}
```

### Pattern 2: Role Hierarchy

Implement role inheritance:

```rego
package authz

# Admin inherits all manager permissions
has_role[role] {
    input.caller.roles[_] == role
}

has_role["manager"] {
    has_role["admin"]
}

has_role["viewer"] {
    has_role["manager"]
}
```

### Pattern 3: Resource Matching

Use glob patterns for resource matching:

```rego
package authz

allow {
    some permission
    permissions[permission]
    glob.match(permission.resource, [], input.resource)
    permission.action == input.action
}
```

## Need Help?

- [Troubleshooting Guide](../troubleshooting-guide.md)
- [Performance Guide](../performance-guide.md)
- [Deployment Guide](../deployment-guide.md)
- [Full Documentation](../README.md)
