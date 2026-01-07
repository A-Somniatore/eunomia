# Basic RBAC Example

A simple role-based access control policy for a REST API.

## Structure

```
basic-rbac/
├── eunomia.toml          # Bundle configuration
├── authz.rego            # Main authorization policy
├── data/
│   └── roles.json        # Role-to-permission mappings
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
eunomia compile . -o basic-rbac.bundle

# Publish (if using registry)
eunomia publish basic-rbac.bundle --registry https://registry.example.com
```

## Policy Overview

This example implements a three-tier role system:

| Role | Permissions |
|------|-------------|
| `admin` | Full access to all resources |
| `editor` | Read and write to content |
| `viewer` | Read-only access |

## Usage in Archimedes

When integrating with Archimedes, send authorization requests with:

```json
{
  "caller": {
    "type": "user",
    "id": "user-123",
    "roles": ["editor"]
  },
  "action": "write",
  "resource": "/api/posts/456"
}
```

Expected response:

```json
{
  "allowed": true,
  "reason": "editor can write to /api/posts/*"
}
```

## Customization

1. **Add new roles**: Edit `data/roles.json`
2. **Add new resources**: Edit permissions in `data/roles.json`
3. **Custom logic**: Modify `authz.rego`

## Testing

Add new test cases to `tests/authz_test.rego`:

```rego
test_custom_scenario {
    authz.allow with input as {
        "caller": {"type": "user", "roles": ["custom-role"]},
        "action": "custom-action",
        "resource": "/api/custom"
    }
}
```
