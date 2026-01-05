# Sample Rego Policies for Eunomia

This directory contains example authorization policies demonstrating common patterns for the Themis Platform.

## Directory Structure

```
examples/policies/
├── common/                     # Shared utilities and base rules
│   ├── authz.rego             # Reusable authorization helpers
│   └── authz_test.rego        # Tests for common utilities
├── users-service/             # Users service policies
│   ├── authz.rego             # Authorization rules
│   └── authz_test.rego        # Policy tests
└── orders-service/            # Orders service policies
    ├── authz.rego             # Authorization rules
    └── authz_test.rego        # Policy tests
```

## Policy Patterns Demonstrated

### 1. Default Deny

All policies follow the "default deny" pattern - requests are denied unless explicitly allowed:

```rego
default allow := false
```

### 2. Caller Types

The policies handle three types of callers:

- **User**: Human users with roles (admin, user, support)
- **SPIFFE**: Service-to-service calls with trust domains
- **API Key**: Programmatic access with scopes

### 3. Common Input Structure

```json
{
  "caller": {
    "type": "user|spiffe|api_key",
    "user_id": "user-123", // for users
    "roles": ["admin", "user"], // for users
    "service_name": "orders-service", // for SPIFFE
    "trust_domain": "somniatore.com", // for SPIFFE
    "scopes": ["users:read"] // for API keys
  },
  "operation_id": "getUser",
  "method": "GET",
  "path": "/users/123",
  "resource": {
    // optional, for ownership checks
    "owner_id": "user-123",
    "status": "pending"
  }
}
```

### 4. Reusable Helpers

The `common/authz.rego` package provides reusable functions:

```rego
import data.common.authz as base

# Caller type checks
base.is_user
base.is_service
base.is_api_key

# Role helpers
base.has_role("admin")
base.has_any_role(["admin", "support"])
base.is_admin

# SPIFFE helpers
base.is_trusted_service("orders-service", "somniatore.com")
base.from_trust_domain("somniatore.com")

# Scope helpers (API keys)
base.has_scope("users:read")
base.has_any_scope(["users:read", "users:write"])

# HTTP method helpers
base.is_read_operation   // GET, HEAD, OPTIONS
base.is_write_operation  // POST, PUT, PATCH, DELETE
```

## Testing Policies

### With OPA CLI

```bash
# Run all tests
opa test examples/policies/ -v

# Run specific service tests
opa test examples/policies/users-service/ -v

# With coverage
opa test examples/policies/ -v --coverage
```

### With Eunomia

```rust
use eunomia_compiler::{RegoEngine, validate_file};

// Validate policy
let report = validate_file("examples/policies/users-service/authz.rego")?;
assert!(report.is_valid());

// Evaluate policy
let mut engine = RegoEngine::new();
engine.add_policy_from_file("examples/policies/users-service/authz.rego")?;
engine.set_input_json(json!({
    "caller": { "type": "user", "roles": ["admin"] },
    "operation_id": "getUser",
    "method": "GET"
}))?;

let allowed = engine.eval_bool("data.users_service.authz.allow")?;
```

## Policy Metadata

Each policy file includes METADATA comments:

```rego
# METADATA
# title: Service Name Authorization Policy
# description: What this policy does
# authors:
#   - Team Name
# scope: service|library|test
# related_resources:
#   - https://docs.example.com/policies
```

## Adding New Policies

1. Create a new directory under `examples/policies/` for your service
2. Add `authz.rego` with your authorization rules
3. Add `authz_test.rego` with comprehensive tests
4. Import `common/authz.rego` for reusable helpers
5. Follow the default deny pattern
6. Include METADATA comments

## Security Best Practices

1. **Always use default deny** - Start with `default allow := false`
2. **Validate trust domains** - Don't trust any SPIFFE identity
3. **Check ownership** - Users should only access their own resources
4. **Limit service permissions** - Services get only what they need
5. **Use explicit scopes** - API keys should have minimum required scopes
6. **Test denial cases** - Ensure unauthorized access is blocked
