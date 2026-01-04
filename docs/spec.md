# Eunomia – Authorization Platform Specification (V1)

## Purpose

Eunomia is the authorization platform for the Themis ecosystem. It defines, validates, compiles, and distributes authorization policies that govern _who is allowed to call which service operations_. Eunomia is the source of truth for authorization logic and ensures policies are auditable, testable, and safely enforced at runtime.

This document is developer-ready and intended to be used directly to implement Eunomia.

---

## 1. Responsibilities

Eunomia is responsible for:

- Acting as the source of truth for authorization policies
- Managing policy definitions written in OPA/Rego
- Validating and testing policies
- Compiling policies into distributable bundles
- Securely pushing policy updates to Archimedes runtimes
- Auditing and versioning policy changes

Eunomia is explicitly **not** responsible for:

- Contract/schema definition (Themis)
- Request routing or traffic exposure (Kratos / ingress)
- Runtime request handling or middleware execution (Archimedes)

---

## 2. Policy Model

### 2.1 Policy Engine

- Authorization engine: **OPA (Open Policy Agent)**
- Policy language: **Rego**

OPA/Rego is used to ensure expressive, composable, and formally evaluable authorization rules.

---

## 3. Policy Scope & Granularity

- Authorization decisions are evaluated at the **operationId** level
- Policies determine whether a given caller identity may invoke a specific operation

### 3.1 Inputs to Policy Evaluation

Each authorization decision evaluates the following inputs:

- Caller identity (SPIFFE ID)
- Target service name
- Target operationId
- Request metadata (method, path, headers if needed)
- Environment/context metadata

---

## 4. Policy Source of Truth

### 4.1 Git-Backed Policies

- Policies are stored in Git repositories
- Git is the authoritative source of truth
- All changes go through:
  - Code review
  - CI validation

Directory structure (example):

```
policies/
  users-service/
    allow.rego
    deny.rego
```

---

## 5. Policy Validation & Testing

### 5.1 CI Enforcement

CI must validate:

- Rego syntax correctness
- Policy compilation success
- Unit tests for allow/deny scenarios
- No ambiguous or conflicting rules

Policy changes that fail validation must not be merged or published.

---

## 6. Policy Compilation

- Eunomia compiles Rego policies into OPA bundles
- Bundles are versioned and content-addressed
- Each bundle corresponds to a specific Git revision

Compiled bundles must be immutable.

---

## 7. Policy Distribution Model

### 7.1 Hybrid Push/Pull Distribution

- Eunomia uses a **hybrid push/pull** model for maximum reliability
- **Primary (Push)**: Control plane pushes bundles to Archimedes instances
- **Fallback (Pull)**: Services can pull from OCI registry on startup or if push fails
- **Local Cache**: SQLite cache for resilient offline operation

### 7.2 Control Plane Interface

- Archimedes exposes a private control-plane endpoint
- Eunomia calls this endpoint to push updates

Security:

- mTLS authentication
- SPIFFE identity allowlist
- Only Eunomia is authorized to push policies

---

## 8. Runtime Integration (Archimedes)

- Archimedes embeds an OPA evaluator
- Policies are evaluated locally per request
- No runtime dependency on Eunomia availability

### 8.1 Policy Update Semantics

- Policy updates are applied atomically
- Previous policy bundle retained for rollback
- Failed updates do not affect live traffic

---

## 9. Failure Behavior

### 9.1 Authorization Denials

When a request is denied:

- Archimedes returns HTTP 403 or equivalent gRPC status
- Themis standard error envelope is used
- Structured audit log is emitted
- Authorization-denied metrics are incremented

### 9.2 Eunomia Failures

- If Eunomia is unavailable:
  - Existing policy bundles remain active
  - No authorization decisions are degraded

---

## 10. Audit & Observability

### 10.1 Audit Logging

Eunomia must emit audit logs for:

- Policy creation
- Policy modification
- Policy deletion
- Policy bundle publication

Audit logs must include:

- Policy version
- Git commit reference
- Author
- Timestamp

### 10.2 Metrics

Eunomia emits metrics for:

- Policy compilation success/failure
- Bundle publication success/failure
- Policy push latency

---

## 11. Security Model

- All communication with Archimedes uses mTLS
- SPIFFE identities are mandatory
- No shared secrets or API keys
- Policy bundles may be optionally signed for defense-in-depth

---

## 12. Testing Strategy

### 12.1 Policy Unit Tests

- Explicit allow scenarios
- Explicit deny scenarios
- Edge cases and defaults

### 12.2 Integration Tests

- Policy bundle compilation
- Hybrid push/pull distribution to Archimedes
- Runtime authorization decisions

### 12.3 Failure Mode Tests

- Invalid policy rejection
- Partial update rollback

---

## 13. Policy File Conventions

### 13.1 Directory Structure

Policies are organized per service with a standard layout:

```
policies/
├── common/                     # Shared utilities and base rules
│   ├── authz.rego             # Reusable authorization helpers
│   └── authz_test.rego        # Tests for common utilities
├── users-service/             # Service-specific policies
│   ├── authz.rego             # Authorization rules
│   └── authz_test.rego        # Policy tests
└── orders-service/
    ├── authz.rego
    └── authz_test.rego
```

### 13.2 Package Naming

Packages follow the convention `<service_name>.authz`:

```rego
package users_service.authz
```

Common/shared libraries use `common.<library_name>`:

```rego
package common.authz
```

### 13.3 Policy Metadata

Every policy file MUST include a METADATA comment block:

```rego
# METADATA
# title: Users Service Authorization Policy
# description: Authorization rules for the users-service
# authors:
#   - Team Name
# scope: service|library|test
# related_resources:
#   - https://docs.example.com/policies
package users_service.authz
```

Required fields:

- `title`: Human-readable policy name
- `description`: What the policy does
- `scope`: One of `service`, `library`, or `test`

Optional fields:

- `authors`: List of authors or teams
- `related_resources`: URLs to documentation

### 13.4 Import Conventions

Use explicit imports with `future.keywords`:

```rego
import future.keywords.if
import future.keywords.in
import future.keywords.contains

# For importing shared libraries
import data.common.authz as base
```

### 13.5 Default Deny Pattern

All authorization policies MUST use the default deny pattern:

```rego
# Default deny - requests denied unless explicitly allowed
default allow := false

# Allow rules define exceptions
allow if {
    # conditions
}
```

Never use `default allow := true`.

### 13.6 Rule Organization

Organize rules in consistent sections:

```rego
# =============================================================================
# Admin Access
# =============================================================================

allow if {
    is_admin
}

# =============================================================================
# User Self-Service
# =============================================================================

allow if {
    # user-specific rules
}

# =============================================================================
# Service-to-Service Access
# =============================================================================

allow if {
    # SPIFFE-based rules
}
```

### 13.7 Test Conventions

Test files must:

- Have `_test.rego` suffix
- Use package name ending in `_test`
- Follow naming pattern `test_<scenario>`:

```rego
package users_service.authz_test

test_admin_can_access_anything if {
    authz.allow with input as {
        "caller": {"type": "user", "roles": ["admin"]}
    }
}

test_user_cannot_access_admin_endpoint if {
    not authz.allow with input as {
        "caller": {"type": "user", "roles": ["user"]},
        "operation_id": "deleteUser"
    }
}
```

### 13.8 Input Schema

Authorization input follows this structure:

```json
{
  "caller": {
    "type": "user|spiffe|api_key",
    "user_id": "user-123",
    "roles": ["admin", "user"],
    "service_name": "orders-service",
    "trust_domain": "example.com",
    "scopes": ["users:read"]
  },
  "operation_id": "getUser",
  "method": "GET",
  "path": "/users/123",
  "resource": {
    "owner_id": "user-123",
    "status": "pending"
  },
  "time": {
    "timestamp": "2024-01-01T00:00:00Z",
    "hour": 14,
    "day_of_week": "monday"
  }
}
```

Caller types:

- `user`: Human user with `user_id` and `roles`
- `spiffe`: Service identity with `service_name` and `trust_domain`
- `api_key`: Programmatic access with `scopes`

### 13.9 Linting Rules

All policies are validated against these rules:

| Rule ID                         | Severity | Description                                 |
| ------------------------------- | -------- | ------------------------------------------- |
| `security/default-deny`         | Error    | Policies must have `default allow := false` |
| `security/no-hardcoded-secrets` | Error    | No hardcoded passwords, tokens, or keys     |
| `security/no-wildcard-allow`    | Warning  | Avoid unconditional allow rules             |
| `style/explicit-imports`        | Hint     | Prefer explicit imports                     |

Use the Eunomia CLI or API to validate policies:

```bash
eunomia validate policies/users-service/authz.rego
```
