# Testing Guide for Eunomia

This guide covers how to write, run, and maintain policy tests using the Eunomia testing framework.

## Table of Contents

1. [Overview](#overview)
2. [Test Discovery](#test-discovery)
3. [Native Rego Tests](#native-rego-tests)
4. [Fixture-Based Tests](#fixture-based-tests)
5. [Mock Identities](#mock-identities)
6. [Test Utilities](#test-utilities)
7. [Running Tests](#running-tests)
8. [Best Practices](#best-practices)
9. [Troubleshooting](#troubleshooting)

---

## Overview

Eunomia supports two types of policy tests:

| Type                    | Description                           | Best For                  |
| ----------------------- | ------------------------------------- | ------------------------- |
| **Native Rego Tests**   | `test_*` rules in `*_test.rego` files | Complex logic, edge cases |
| **Fixture-Based Tests** | JSON/YAML input + expected output     | Scenarios, documentation  |

Both types can be mixed in the same project and are discovered automatically.

---

## Test Discovery

The `TestDiscovery` component finds tests in your policy directories:

```rust
use eunomia_test::{TestDiscovery, DiscoveryConfig};

// Default discovery
let discovery = TestDiscovery::new();
let suite = discovery.discover("policies/")?;

// Custom configuration
let config = DiscoveryConfig::new()
    .with_recursive(true)          // Scan subdirectories
    .with_fixtures(true)           // Include fixture files
    .exclude_dir("vendor");        // Skip vendor directory

let discovery = TestDiscovery::with_config(config);
let suite = discovery.discover("policies/")?;

println!("Found {} tests", suite.test_count());
```

### What Gets Discovered

| Pattern                   | Description                         |
| ------------------------- | ----------------------------------- |
| `*_test.rego`             | Rego test files with `test_*` rules |
| `*.rego`                  | Policy files (loaded for imports)   |
| `*_fixtures.json`         | JSON fixture files                  |
| `*_fixtures.yaml`         | YAML fixture files                  |
| `data.json` / `data.yaml` | Data files for policy context       |

---

## Native Rego Tests

Native tests are Rego rules prefixed with `test_` in files ending with `_test.rego`.

### Basic Test Structure

```rego
# authz_test.rego
package authz_test

import data.authz        # Import the policy under test
import future.keywords

# Test admin access
test_admin_allowed if {
    authz.allow with input as {
        "caller": {"type": "user", "roles": ["admin"]},
        "operation_id": "deleteUser",
        "method": "DELETE"
    }
}

# Test guest denial
test_guest_denied if {
    not authz.allow with input as {
        "caller": {"type": "user", "roles": []},
        "operation_id": "deleteUser",
        "method": "DELETE"
    }
}
```

### Using Helper Functions

```rego
package authz_test

import data.authz
import data.common.authz as base
import future.keywords

# Create reusable input
admin_input := {
    "caller": {"type": "user", "user_id": "admin-1", "roles": ["admin"]},
    "operation_id": "createUser",
    "method": "POST"
}

test_admin_create if {
    authz.allow with input as admin_input
}

# Test with modified input
test_editor_create_denied if {
    modified := object.union(admin_input, {"caller": {"type": "user", "roles": ["editor"]}})
    not authz.allow with input as modified
}
```

### Testing with External Data

```rego
package authz_test

import data.authz
import data.roles    # Loaded from data.json

test_role_permissions if {
    # data.json: {"roles": {"admin": ["read", "write", "delete"]}}
    "write" in roles.admin
}
```

---

## Fixture-Based Tests

Fixtures define test cases in JSON or YAML format.

### JSON Fixture Format

```json
{
  "name": "Authorization Test Suite",
  "package": "my_service.authz",
  "fixtures": [
    {
      "name": "admin_can_delete",
      "description": "Admin users should be able to delete resources",
      "input": {
        "caller": {
          "type": "user",
          "user_id": "admin-123",
          "roles": ["admin"]
        },
        "operation_id": "deleteResource",
        "method": "DELETE",
        "path": "/resources/123"
      },
      "expected_allowed": true
    },
    {
      "name": "guest_cannot_delete",
      "description": "Guest users cannot delete resources",
      "input": {
        "caller": {
          "type": "user",
          "roles": []
        },
        "operation_id": "deleteResource",
        "method": "DELETE"
      },
      "expected_allowed": false
    }
  ]
}
```

### YAML Fixture Format

```yaml
name: Authorization Test Suite
package: my_service.authz
fixtures:
  - name: service_can_read
    description: Orders service can read user data
    input:
      caller:
        type: spiffe
        spiffe_id: spiffe://example.com/ns/prod/sa/orders
        service_name: orders-service
        trust_domain: example.com
      operation_id: getUser
      method: GET
    expected_allowed: true

  - name: unknown_service_denied
    description: Unknown services are denied
    input:
      caller:
        type: spiffe
        spiffe_id: spiffe://evil.com/ns/prod/sa/hacker
        service_name: hacker-service
        trust_domain: evil.com
      operation_id: getUser
      method: GET
    expected_allowed: false
```

### Fixture with Additional Data

```json
{
  "fixtures": [
    {
      "name": "test_with_data",
      "input": { "caller": { "type": "user", "roles": ["viewer"] } },
      "expected_allowed": true,
      "data": {
        "permissions": {
          "viewer": ["read"]
        }
      }
    }
  ]
}
```

---

## Mock Identities

The `MockUser`, `MockSpiffe`, and `MockApiKey` builders simplify identity creation.

### MockUser

```rust
use eunomia_test::MockUser;

// Factory methods
let admin = MockUser::admin();           // admin role
let viewer = MockUser::viewer();         // viewer role
let editor = MockUser::editor();         // editor role
let guest = MockUser::guest();           // no roles
let super_admin = MockUser::super_admin(); // super_admin + admin roles

// Custom builder
let user = MockUser::new("user-123")
    .with_roles(["custom_role", "another_role"])
    .with_tenant("tenant-abc")
    .build();
```

### MockSpiffe

```rust
use eunomia_test::MockSpiffe;

// Factory methods
let users = MockSpiffe::users_service();
let orders = MockSpiffe::orders_service();
let gateway = MockSpiffe::gateway();

// Custom builder
let service = MockSpiffe::new("my-service")
    .with_trust_domain("prod.example.com")
    .with_namespace("production")
    .build();
```

### MockApiKey

```rust
use eunomia_test::MockApiKey;

// Factory methods
let read_key = MockApiKey::read_only();      // read:* scope
let admin_key = MockApiKey::full_access();   // admin:* scope
let user_read = MockApiKey::read_service("users");  // read:users
let user_write = MockApiKey::write_service("users"); // read:users + write:users

// Custom builder
let key = MockApiKey::new("api-key-123")
    .with_scopes(["custom:scope", "another:scope"])
    .build();
```

---

## Test Utilities

### InputBuilder

Build policy input with a fluent API:

```rust
use eunomia_test::{InputBuilder, MockUser};

let input = InputBuilder::new()
    .caller(MockUser::admin())
    .operation("deleteUser")
    .method("DELETE")
    .path("/users/user-456")
    .service("users-service")
    .header("Authorization", "Bearer token")
    .context_string("resourceId", "user-456")
    .environment("production")
    .build();
```

### Assertions

```rust
use eunomia_test::{assert_allowed, assert_denied, assert_all_passed};

// Single result assertions
let result = runner.run_fixture(&fixture, policy);
assert_allowed(&result);

let deny_result = runner.run_fixture(&deny_fixture, policy);
assert_denied(&deny_result);

// All results assertion
let results = runner.run_suite(&suite)?;
assert_all_passed(&results);
```

### Policy Generators

```rust
use eunomia_test::{simple_allow_policy, role_based_policy, scope_based_policy};

// Allow based on caller type
let policy = simple_allow_policy("admin");

// Allow based on role
let policy = role_based_policy("super_admin");

// Allow based on API key scope
let policy = scope_based_policy("read:users");
```

---

## Running Tests

### With CLI

```bash
# Run all tests
eunomia test policies/

# Run tests in specific directory
eunomia test policies/users-service/

# Verbose output
eunomia test policies/ -v

# Fail fast on first error
eunomia test policies/ -f

# Filter tests by pattern
eunomia test policies/ --filter "admin"
```

### With Rust API

```rust
use eunomia_test::{TestRunner, TestConfig, TestDiscovery};

let discovery = TestDiscovery::new();
let suite = discovery.discover("policies/")?;

let runner = TestRunner::new(
    TestConfig::default()
        .with_fail_fast(true)
        .with_timeout(Duration::from_secs(30))
);

// Run only Rego tests
let results = runner.run_suite(&suite)?;

// Run only fixtures
let results = runner.run_discovered_fixtures(&suite)?;

// Run both
let results = runner.run_all(&suite)?;

println!("Passed: {}, Failed: {}", results.passed(), results.failed());

for failure in results.failures() {
    println!("FAILED: {} - {}", failure.name, failure.error.as_deref().unwrap_or(""));
}
```

---

## Best Practices

### 1. Test Default Deny

Always verify that unauthorized access is denied:

```rego
test_anonymous_denied if {
    not authz.allow with input as {"caller": {"type": "anonymous"}}
}

test_no_roles_denied if {
    not authz.allow with input as {
        "caller": {"type": "user", "roles": []}
    }
}
```

### 2. Test All Caller Types

Cover all identity types your policy handles:

```rego
# User access
test_user_access if { ... }

# Service-to-service
test_service_access if { ... }

# API keys
test_api_key_access if { ... }

# Anonymous (usually denied)
test_anonymous_denied if { ... }
```

### 3. Test Edge Cases

```rego
# Empty inputs
test_empty_roles if {
    not authz.allow with input as {"caller": {"type": "user", "roles": []}}
}

# Missing fields
test_missing_operation if {
    not authz.allow with input as {"caller": {"type": "admin"}}
}

# Boundary conditions
test_exactly_required_role if { ... }
```

### 4. Use Descriptive Names

```rego
# Good
test_admin_can_delete_any_user if { ... }
test_editor_cannot_delete_other_users if { ... }
test_viewer_can_only_read_own_profile if { ... }

# Bad
test_1 if { ... }
test_delete if { ... }
```

### 5. Organize by Feature

```
policies/
├── users-service/
│   ├── authz.rego
│   ├── authz_test.rego        # Rego tests
│   └── authz_fixtures.json    # Fixture tests
└── orders-service/
    ├── authz.rego
    └── authz_test.rego
```

### 6. Test Role Hierarchies

If using hierarchical roles, test that inheritance works correctly:

```rego
# Test higher roles can do what lower roles can
test_admin_inherits_viewer_permissions if {
    authz.allow with input as {
        "caller": {"type": "user", "roles": ["admin"]},
        "operation_id": "viewContent"  # A viewer-level operation
    }
}

# Test lower roles cannot escalate
test_viewer_cannot_access_admin_operations if {
    not authz.allow with input as {
        "caller": {"type": "user", "roles": ["viewer"]},
        "operation_id": "deleteUser"  # An admin-level operation
    }
}
```

### 7. Test Tenant Isolation (Multi-Tenant)

For multi-tenant applications, always test cross-tenant access:

```rego
# Users can access their own tenant
test_user_accesses_own_tenant if {
    authz.allow with input as {
        "caller": {"type": "user", "tenant_id": "tenant-a"},
        "context": {"tenant_id": "tenant-a"}
    }
}

# Users cannot access other tenants
test_user_cannot_access_other_tenant if {
    not authz.allow with input as {
        "caller": {"type": "user", "tenant_id": "tenant-a"},
        "context": {"tenant_id": "tenant-b"}
    }
}
```

### 8. Test Scope Requirements

For API key/scope-based authorization, test all scope combinations:

```rego
# Single scope sufficient
test_read_scope_allows_read if {
    authz.allow with input as {
        "caller": {"type": "api_key", "scopes": ["users:read"]},
        "operation_id": "getUser"
    }
}

# Multiple scopes required
test_multiple_scopes_required if {
    authz.allow with input as {
        "caller": {"type": "api_key", "scopes": ["analytics:read", "analytics:export"]},
        "operation_id": "generateReport"
    }
}

# Missing one scope fails
test_missing_scope_denied if {
    not authz.allow with input as {
        "caller": {"type": "api_key", "scopes": ["analytics:read"]},
        "operation_id": "generateReport"  # Requires both read and export
    }
}
```

### 9. Test Time-Based Constraints

If policies have time-based rules, test expiration and validity windows:

```rego
test_expired_key_denied if {
    not authz.allow with input as {
        "caller": {
            "type": "api_key",
            "expires_at": "2020-01-01T00:00:00Z"
        },
        "timestamp": "2026-01-05T10:00:00Z"
    }
}

test_valid_key_allowed if {
    authz.allow with input as {
        "caller": {
            "type": "api_key",
            "scopes": ["users:read"],
            "expires_at": "2027-01-01T00:00:00Z"
        },
        "operation_id": "getUser",
        "timestamp": "2026-01-05T10:00:00Z"
    }
}
```

### 10. Coverage Checklist

Before submitting policy changes, ensure you have tests for:

- [ ] All `allow` rules (positive tests)
- [ ] Default deny behavior (negative tests)
- [ ] Each operation defined in the policy
- [ ] Each caller type (user, spiffe, api_key, anonymous)
- [ ] Role/scope boundaries and inheritance
- [ ] Resource ownership checks
- [ ] Tenant isolation (if applicable)
- [ ] Time-based constraints (if applicable)

---

## Troubleshooting

### Import Errors

**Problem**: `undefined ref: data.authz`

**Solution**: Ensure all policy files are in the same directory tree. The test runner loads all `.rego` files.

### Fixture Not Found

**Problem**: `Could not find policy file for fixture`

**Solution**: Name your fixture file to match the policy: `authz_fixtures.json` for `authz.rego`.

### Test Timeout

**Problem**: Tests hang or timeout

**Solution**: Check for infinite loops in Rego rules. Use `TestConfig::with_timeout()` to adjust.

### Data File Not Loaded

**Problem**: `undefined ref: data.roles`

**Solution**: Name your data file `data.json` or `data.yaml`. Discovery finds files starting with "data".

---

## See Also

- [Example Policies](../examples/policies/README.md)
- [Design Document - Testing](design.md#8-testing-framework)
- [Rego Language Reference](https://www.openpolicyagent.org/docs/latest/policy-language/)
