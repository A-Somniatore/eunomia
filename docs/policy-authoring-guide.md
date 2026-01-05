# Policy Authoring Guide

> **Version**: 1.0.0  
> **Last Updated**: 2026-01-05  
> **Audience**: Developers writing authorization policies for Themis services

This guide covers how to write, test, and deploy authorization policies using Eunomia and OPA/Rego.

---

## Table of Contents

1. [Introduction](#1-introduction)
2. [Getting Started](#2-getting-started)
3. [Policy Structure](#3-policy-structure)
4. [Writing Rules](#4-writing-rules)
5. [Common Patterns](#5-common-patterns)
6. [Testing Policies](#6-testing-policies)
7. [Building & Signing](#7-building--signing)
8. [Deployment](#8-deployment)
9. [Best Practices](#9-best-practices)
10. [Troubleshooting](#10-troubleshooting)

---

## 1. Introduction

### What is Eunomia?

Eunomia is the authorization policy platform for the Themis ecosystem. It provides:

- **OPA/Rego** as the policy language
- **Git-backed** policy management with code review
- **Comprehensive testing** framework
- **Signed bundles** for secure distribution
- **Push-based deployment** to Archimedes instances

### Key Concepts

| Concept | Description |
|---------|-------------|
| **Policy** | A Rego file defining authorization rules |
| **Bundle** | A compiled, signed package of policies |
| **Decision** | The result of evaluating a policy (allow/deny) |
| **CallerIdentity** | Who is making the request (user, service, API key) |
| **OperationId** | The specific operation being authorized |

### Authorization Flow

```
┌─────────────┐     ┌───────────────┐     ┌────────────────┐
│   Client    │────▶│  Archimedes   │────▶│  OPA Engine    │
│  (Request)  │     │   (Service)   │     │  (Local Eval)  │
└─────────────┘     └───────────────┘     └────────────────┘
                           │                      │
                           │                      ▼
                           │              ┌────────────────┐
                           │              │  Policy Bundle │
                           │◀─────────────│  (from Eunomia)│
                           │              └────────────────┘
```

---

## 2. Getting Started

### Prerequisites

- Rust toolchain (for building Eunomia CLI)
- Git (for policy version control)
- Basic understanding of Rego syntax

### Installing Eunomia CLI

```bash
# From source
git clone https://github.com/A-Somniatore/eunomia
cd eunomia
cargo install --path crates/eunomia-cli

# Verify installation
eunomia --version
```

### Creating Your First Policy

1. **Create the policy directory structure:**

```bash
mkdir -p policies/my-service
cd policies/my-service
```

2. **Create the authorization policy (`authz.rego`):**

```rego
# METADATA
# title: My Service Authorization
# description: Authorization rules for my-service
# scope: service
package my_service.authz

import future.keywords.if
import future.keywords.in

# Default deny - all requests denied unless explicitly allowed
default allow := false

# Allow admin users to do anything
allow if {
    input.caller.type == "user"
    "admin" in input.caller.roles
}

# Allow users to read their own data
allow if {
    input.caller.type == "user"
    input.operation_id == "getProfile"
    input.context.user_id == input.caller.user_id
}
```

3. **Create tests (`authz_test.rego`):**

```rego
package my_service.authz_test

import data.my_service.authz

# Test: Admin users should be allowed
test_admin_allowed if {
    authz.allow with input as {
        "caller": {"type": "user", "user_id": "admin-1", "roles": ["admin"]},
        "operation_id": "deleteUser",
        "service": "my-service"
    }
}

# Test: Regular users cannot delete others
test_user_cannot_delete_others if {
    not authz.allow with input as {
        "caller": {"type": "user", "user_id": "user-1", "roles": ["user"]},
        "operation_id": "deleteUser",
        "service": "my-service"
    }
}

# Test: Users can read their own profile
test_user_reads_own_profile if {
    authz.allow with input as {
        "caller": {"type": "user", "user_id": "user-1", "roles": ["user"]},
        "operation_id": "getProfile",
        "context": {"user_id": "user-1"},
        "service": "my-service"
    }
}
```

4. **Run the tests:**

```bash
eunomia test policies/my-service
```

---

## 3. Policy Structure

### Package Naming Convention

```rego
# Service policies: <service_name>.authz
package users_service.authz

# Shared libraries: common.<module_name>
package common.roles

# Test packages: <package>_test
package users_service.authz_test
```

### Required Elements

Every policy file MUST include:

```rego
# METADATA block (required)
# METADATA
# title: Service Name Authorization
# description: What this policy does
# scope: service

# Package declaration
package service_name.authz

# Future keywords for modern Rego
import future.keywords.if
import future.keywords.in
import future.keywords.contains
import future.keywords.every

# Default deny (MANDATORY)
default allow := false

# At least one allow rule
allow if {
    # conditions
}
```

### Input Schema

Authorization requests follow this structure:

```json
{
  "caller": {
    "type": "user|spiffe|api_key|anonymous",
    "user_id": "user-123",
    "email": "user@example.com",
    "roles": ["admin", "user"],
    "service_name": "orders-service",
    "trust_domain": "example.com",
    "key_id": "api-key-123",
    "key_name": "Production API Key",
    "scopes": ["read", "write"]
  },
  "service": "users-service",
  "operation_id": "getUser",
  "method": "GET",
  "path": "/users/123",
  "headers": {
    "content-type": "application/json"
  },
  "timestamp": "2026-01-05T10:30:00Z",
  "environment": "production",
  "context": {
    "user_id": "123",
    "resource_owner": "user-456"
  }
}
```

### Caller Types

| Type | Fields | Use Case |
|------|--------|----------|
| `user` | `user_id`, `email`, `roles` | Human users via web/mobile |
| `spiffe` | `service_name`, `trust_domain` | Service-to-service calls |
| `api_key` | `key_id`, `key_name`, `scopes` | Programmatic API access |
| `anonymous` | (none) | Unauthenticated requests |

---

## 4. Writing Rules

### Basic Rule Structure

```rego
# Simple condition
allow if {
    input.caller.type == "user"
    "admin" in input.caller.roles
}

# Multiple conditions (AND)
allow if {
    input.caller.type == "user"
    input.operation_id == "readData"
    input.caller.user_id == input.context.owner_id
}

# Alternative rules (OR) - separate rule blocks
allow if {
    is_admin
}

allow if {
    is_owner
}
```

### Using Helper Rules

```rego
# Define reusable helper rules
is_admin if {
    input.caller.type == "user"
    "admin" in input.caller.roles
}

is_owner if {
    input.caller.type == "user"
    input.caller.user_id == input.context.resource_owner
}

is_internal_service if {
    input.caller.type == "spiffe"
    input.caller.trust_domain == "somniatore.com"
}

# Use helpers in allow rules
allow if is_admin
allow if is_owner
allow if is_internal_service
```

### Working with Collections

```rego
# Check membership
allow if {
    input.operation_id in allowed_operations
}

allowed_operations := {"getUser", "listUsers", "getProfile"}

# Check any element matches
allow if {
    some role in input.caller.roles
    role in ["admin", "superuser"]
}

# Check all elements
allow if {
    every scope in required_scopes {
        scope in input.caller.scopes
    }
}

required_scopes := ["users:read", "users:write"]
```

### Extracting Values from Paths

```rego
# Extract user ID from path like "/users/123"
allow if {
    path_parts := split(input.path, "/")
    count(path_parts) >= 3
    path_parts[1] == "users"
    user_id := path_parts[2]
    user_id == input.caller.user_id
}
```

---

## 5. Common Patterns

### Role-Based Access Control (RBAC)

```rego
package my_service.authz

import future.keywords.if
import future.keywords.in

default allow := false

# Role hierarchy
role_grants := {
    "admin": {"admin", "editor", "viewer"},
    "editor": {"editor", "viewer"},
    "viewer": {"viewer"}
}

# Check if user has required role (including hierarchy)
has_role(required) if {
    some user_role in input.caller.roles
    required in role_grants[user_role]
}

# Operations mapped to required roles
operation_roles := {
    "deleteUser": "admin",
    "updateUser": "editor",
    "getUser": "viewer",
    "listUsers": "viewer"
}

allow if {
    input.caller.type == "user"
    required_role := operation_roles[input.operation_id]
    has_role(required_role)
}
```

### Service-to-Service Authorization

```rego
package my_service.authz

import future.keywords.if
import future.keywords.in

default allow := false

# Define which services can call which operations
service_permissions := {
    "orders-service": {"getUser", "getUsersByIds"},
    "payments-service": {"getUser", "validateUser"},
    "analytics-service": {"listUsers"}
}

allow if {
    input.caller.type == "spiffe"
    input.caller.trust_domain == "somniatore.com"
    allowed_ops := service_permissions[input.caller.service_name]
    input.operation_id in allowed_ops
}
```

### API Key Scope-Based Access

```rego
package my_service.authz

import future.keywords.if
import future.keywords.in
import future.keywords.every

default allow := false

# Map operations to required scopes
operation_scopes := {
    "getUser": ["users:read"],
    "updateUser": ["users:write"],
    "deleteUser": ["users:delete"],
    "listUsers": ["users:read"]
}

allow if {
    input.caller.type == "api_key"
    required := operation_scopes[input.operation_id]
    every scope in required {
        scope in input.caller.scopes
    }
}
```

### Resource Ownership

```rego
package my_service.authz

import future.keywords.if

default allow := false

# Users can access their own resources
allow if {
    input.caller.type == "user"
    input.context.resource_owner == input.caller.user_id
}

# Users can only modify resources in "draft" status
allow if {
    input.caller.type == "user"
    input.operation_id in {"updateResource", "deleteResource"}
    input.context.resource_owner == input.caller.user_id
    input.context.resource_status == "draft"
}
```

### Time-Based Access

```rego
package my_service.authz

import future.keywords.if

default allow := false

# Only allow during business hours (9 AM - 5 PM UTC)
allow if {
    input.caller.type == "user"
    business_hours
    # ... other conditions
}

business_hours if {
    ts := time.parse_rfc3339_ns(input.timestamp)
    [hour, _, _] := time.clock([ts, "UTC"])
    hour >= 9
    hour < 17
}
```

### Multi-Tenant Isolation

```rego
package my_service.authz

import future.keywords.if

default allow := false

# Users can only access resources in their tenant
allow if {
    input.caller.type == "user"
    input.caller.tenant_id == input.context.tenant_id
}

# Cross-tenant access requires explicit permission
allow if {
    input.caller.type == "user"
    "cross_tenant_access" in input.caller.permissions
}
```

---

## 6. Testing Policies

### Writing Tests

Test files must:
- Have `_test.rego` suffix
- Use `test_` prefix for test rules
- Return `true` on success

```rego
package my_service.authz_test

import data.my_service.authz

# Positive test - should allow
test_admin_allowed if {
    authz.allow with input as {
        "caller": {"type": "user", "roles": ["admin"]},
        "operation_id": "deleteUser"
    }
}

# Negative test - should deny
test_guest_denied if {
    not authz.allow with input as {
        "caller": {"type": "user", "roles": ["guest"]},
        "operation_id": "deleteUser"
    }
}

# Test with full input
test_user_reads_own_profile if {
    authz.allow with input as {
        "caller": {
            "type": "user",
            "user_id": "user-123",
            "roles": ["user"]
        },
        "service": "users-service",
        "operation_id": "getProfile",
        "method": "GET",
        "path": "/users/user-123/profile",
        "context": {"user_id": "user-123"}
    }
}
```

### Using Fixtures

Create `authz_fixtures.json` or `authz_fixtures.yaml`:

```json
{
  "fixtures": [
    {
      "name": "admin_can_delete_user",
      "description": "Admin users should be able to delete any user",
      "input": {
        "caller": {"type": "user", "user_id": "admin-1", "roles": ["admin"]},
        "operation_id": "deleteUser",
        "context": {"target_user_id": "user-456"}
      },
      "expected": {
        "allow": true
      }
    },
    {
      "name": "user_cannot_delete_others",
      "description": "Regular users cannot delete other users",
      "input": {
        "caller": {"type": "user", "user_id": "user-123", "roles": ["user"]},
        "operation_id": "deleteUser",
        "context": {"target_user_id": "user-456"}
      },
      "expected": {
        "allow": false
      }
    }
  ]
}
```

### Running Tests

```bash
# Run all tests in a directory
eunomia test policies/

# Run tests for a specific service
eunomia test policies/users-service/

# Run with verbose output
eunomia test -v policies/

# Run with filter
eunomia test --filter "admin" policies/

# Fail fast on first error
eunomia test -f policies/
```

### Test Coverage Best Practices

Ensure you test:

1. **Happy paths** - Valid access scenarios
2. **Deny cases** - Invalid access attempts
3. **Edge cases** - Boundary conditions
4. **All caller types** - User, SPIFFE, API key, anonymous
5. **All operations** - Each operationId in your service
6. **Role hierarchies** - If using RBAC

---

## 7. Building & Signing

### Building a Bundle

```bash
# Build a bundle for a service
eunomia build \
  --dir policies/users-service \
  --service users-service \
  --version 1.0.0 \
  --output bundles/

# Build with git commit metadata
eunomia build \
  --dir policies/users-service \
  --service users-service \
  --version 1.0.0 \
  --git-commit $(git rev-parse HEAD) \
  --output bundles/
```

### Signing Bundles

```bash
# Generate a signing key pair
eunomia sign --generate-key --key-file signing-key.pem

# Sign a bundle
eunomia sign \
  --bundle bundles/users-service-1.0.0.bundle.tar.gz \
  --key-file signing-key.pem \
  --key-id production-key-2026

# Sign using environment variable
export EUNOMIA_SIGNING_KEY=$(cat signing-key.pem)
eunomia sign \
  --bundle bundles/users-service-1.0.0.bundle.tar.gz \
  --key-id production-key-2026
```

### Publishing to Registry

```bash
# Publish to OCI registry
eunomia publish \
  --bundle bundles/users-service-1.0.0.bundle.tar.gz \
  --registry registry.example.com \
  --service users-service \
  --version 1.0.0 \
  --token $REGISTRY_TOKEN

# With basic auth
eunomia publish \
  --bundle bundles/users-service-1.0.0.bundle.tar.gz \
  --registry registry.example.com \
  --service users-service \
  --version 1.0.0 \
  --username admin \
  --password $REGISTRY_PASSWORD
```

---

## 8. Deployment

### Push to Instances

```bash
# Deploy to specific endpoints
eunomia push \
  --service users-service \
  --version 1.0.0 \
  --endpoints host1:8080,host2:8080,host3:8080 \
  --strategy immediate

# Canary deployment (10% first)
eunomia push \
  --service users-service \
  --version 1.0.0 \
  --endpoints host1:8080,host2:8080,host3:8080 \
  --strategy canary \
  --canary-percentage 10 \
  --canary-duration 300

# Rolling deployment
eunomia push \
  --service users-service \
  --version 1.0.0 \
  --endpoints host1:8080,host2:8080,host3:8080 \
  --strategy rolling \
  --batch-size 1 \
  --batch-delay 30

# Dry run
eunomia push \
  --service users-service \
  --version 1.0.0 \
  --endpoints host1:8080 \
  --strategy immediate \
  --dry-run
```

### Deployment Strategies

| Strategy | Description | Use Case |
|----------|-------------|----------|
| `immediate` | All instances at once | Development, small deployments |
| `canary` | Percentage-based rollout | Production, risk mitigation |
| `rolling` | Batch-based sequential | Large deployments |

---

## 9. Best Practices

### DO ✅

```rego
# DO: Use default deny
default allow := false

# DO: Use explicit imports
import future.keywords.if
import future.keywords.in

# DO: Document your policies
# METADATA
# title: Clear description
# description: What this does

# DO: Use helper rules for readability
is_admin if {
    "admin" in input.caller.roles
}

allow if is_admin

# DO: Test both allow and deny cases
test_admin_allowed if { ... }
test_guest_denied if { not ... }

# DO: Use descriptive rule names
allow_self_service_profile_read if { ... }
```

### DON'T ❌

```rego
# DON'T: Use default allow
default allow := true  # SECURITY RISK!

# DON'T: Hardcode secrets
allow if {
    input.headers["x-api-key"] == "secret123"  # NEVER DO THIS
}

# DON'T: Use overly permissive rules
allow if { true }  # ALLOWS EVERYTHING!

# DON'T: Ignore caller type
allow if {
    "admin" in input.caller.roles  # What if caller is API key?
}

# DON'T: Use deprecated input fields
allow if {
    input.action == "read"  # Use operation_id instead
    input.resource == "user"  # Use context instead
}
```

### Security Guidelines

1. **Always use default deny** - `default allow := false`
2. **Validate caller type** - Don't assume caller structure
3. **Check trust domain** - For SPIFFE identities
4. **Use scopes for API keys** - Fine-grained access control
5. **Log denials** - For security monitoring
6. **Review changes** - All policy changes through code review
7. **Test thoroughly** - Coverage for all code paths

---

## 10. Troubleshooting

### Common Errors

**"undefined rule: allow"**
```rego
# Problem: Missing default
# Solution: Add default rule
default allow := false
```

**"var is unsafe"**
```rego
# Problem: Using unbound variable
allow if {
    role in input.caller.roles  # 'role' is undefined
}

# Solution: Use 'some' to introduce variable
allow if {
    some role in input.caller.roles
    role == "admin"
}
```

**"rego_type_error: undefined ref"**
```rego
# Problem: Accessing non-existent field
allow if {
    input.user.role == "admin"  # 'user' doesn't exist
}

# Solution: Use correct path
allow if {
    input.caller.roles[_] == "admin"
}
```

### Debugging Tips

1. **Use `print()` for debugging:**
```rego
allow if {
    print("Checking admin access for:", input.caller)
    "admin" in input.caller.roles
}
```

2. **Test with minimal input first:**
```rego
test_minimal if {
    authz.allow with input as {
        "caller": {"type": "user", "roles": ["admin"]}
    }
}
```

3. **Check policy loading:**
```bash
eunomia test -v policies/  # Verbose output shows loaded files
```

4. **Validate syntax:**
```bash
eunomia validate policies/my-service/authz.rego
```

### Getting Help

- **Documentation**: [docs/](.)
- **Examples**: [examples/policies/](../examples/policies/)
- **Issues**: [GitHub Issues](https://github.com/A-Somniatore/eunomia/issues)

---

## Appendix: Quick Reference

### CLI Commands

| Command | Description |
|---------|-------------|
| `eunomia test <dir>` | Run policy tests |
| `eunomia build --dir <dir> --service <name> --version <ver>` | Build bundle |
| `eunomia sign --bundle <file> --key-file <key>` | Sign bundle |
| `eunomia publish --bundle <file> --registry <url>` | Publish to registry |
| `eunomia fetch --registry <url> --service <name>` | Fetch from registry |
| `eunomia push --service <name> --endpoints <hosts>` | Deploy to instances |
| `eunomia validate <file>` | Validate policy syntax |

### Input Fields Reference

| Field | Type | Description |
|-------|------|-------------|
| `caller.type` | string | `user`, `spiffe`, `api_key`, `anonymous` |
| `caller.user_id` | string | User identifier |
| `caller.email` | string | User email |
| `caller.roles` | array | User roles |
| `caller.service_name` | string | SPIFFE service name |
| `caller.trust_domain` | string | SPIFFE trust domain |
| `caller.key_id` | string | API key identifier |
| `caller.scopes` | array | API key scopes |
| `service` | string | Target service name |
| `operation_id` | string | Target operation |
| `method` | string | HTTP method |
| `path` | string | Request path |
| `headers` | object | Request headers |
| `timestamp` | string | RFC3339 timestamp |
| `environment` | string | Environment name |
| `context` | object | Additional context |
