# Eunomia – Implementation Design Document

> **Version**: 1.2.0-draft  
> **Status**: Design Phase  
> **Repository**: `github.com/A-Somniatore/eunomia` (to be created)  
> **Last Updated**: 2026-01-05
>
> **NOTE**: Archimedes is now a full framework replacement for Axum/FastAPI/Boost.
> See [ADR-008](../../docs/decisions/008-archimedes-full-framework.md).

---

## Table of Contents

1. [Executive Summary](#1-executive-summary)
2. [Goals & Non-Goals](#2-goals--non-goals)
3. [Architecture Overview](#3-architecture-overview)
4. [Repository Structure](#4-repository-structure)
5. [Policy Model](#5-policy-model)
6. [Policy Authoring](#6-policy-authoring)
7. [Policy Lifecycle](#7-policy-lifecycle)
8. [Policy Versioning](#7b-policy-versioning)
9. [Bundle Compilation](#8-bundle-compilation)
10. [Policy Distribution (Hybrid Push/Pull)](#9-policy-distribution)
11. [Control Plane](#10-control-plane)
12. [Resilience & Caching](#10b-resilience--caching)
13. [Testing Framework](#11-testing-framework)
14. [CI Pipeline](#12-ci-pipeline)
15. [Observability](#13-observability)
16. [Security Model](#14-security-model)
17. [CLI Design](#15-cli-design)
18. [Integration Points](#16-integration-points)
19. [Open Questions](#17-open-questions)
20. [Implementation Phases](#18-implementation-phases)

---

## 1. Executive Summary

Eunomia is the **authorization policy platform** for the Themis ecosystem. It provides:

- **Policy authoring** in OPA/Rego with Git as source of truth
- **Policy testing** framework with comprehensive test coverage
- **Policy compilation** into distributable bundles
- **Policy distribution** via **hybrid push/pull** deployment to Archimedes
- **Semantic versioning** for all policy bundles
- **Audit logging** for all policy changes and authorization decisions
- **Resilient operation** with local caching for high availability

Eunomia ensures that authorization logic is:

- **Declarative** – policies describe intent, not implementation
- **Testable** – every policy has explicit test cases
- **Auditable** – all changes tracked, all decisions logged
- **Atomic** – policy updates are all-or-nothing with rollback
- **Resilient** – services continue operating even if control plane is down

---

## 2. Goals & Non-Goals

### Goals

- ✅ Git-backed policy management (code review, history)
- ✅ OPA/Rego as the policy language
- ✅ Per-`operationId` authorization decisions
- ✅ **Hybrid push/pull** policy distribution to Archimedes
- ✅ Comprehensive policy testing framework
- ✅ Atomic policy updates with rollback
- ✅ Audit logging for policy changes
- ✅ Integration with SPIFFE identities
- ✅ **Semantic versioning** for all policy bundles
- ✅ **Local caching** for resilient operation

### Non-Goals (V1)

- ❌ UI-based policy editing
- ❌ Dynamic/ad-hoc policy changes
- ❌ Per-request remote policy evaluation (embedded in Archimedes)
- ❌ Non-Rego policy languages
- ❌ Fine-grained data-level authorization (row-level security)

---

## 3. Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              EUNOMIA ECOSYSTEM                               │
│                                                                              │
│  ┌─────────────────────────────────────────────────────────────────────────┐│
│  │                       Policy Repositories (Git)                          ││
│  │                                                                          ││
│  │   ┌──────────────────────────────────────────────────────────────────┐  ││
│  │   │  policies/                                                        │  ││
│  │   │  ├── common/           # Shared policy modules                    │  ││
│  │   │  │   ├── roles.rego                                               │  ││
│  │   │  │   └── identity.rego                                            │  ││
│  │   │  ├── users-service/    # Service-specific policies                │  ││
│  │   │  │   ├── authz.rego                                               │  ││
│  │   │  │   └── authz_test.rego                                          │  ││
│  │   │  └── orders-service/                                              │  ││
│  │   │      ├── authz.rego                                               │  ││
│  │   │      └── authz_test.rego                                          │  ││
│  │   └──────────────────────────────────────────────────────────────────┘  ││
│  └─────────────────────────────────────────────────────────────────────────┘│
│                                    │                                         │
│                                    │ git push                                │
│                                    ▼                                         │
│  ┌─────────────────────────────────────────────────────────────────────────┐│
│  │                           CI Pipeline                                    ││
│  │                                                                          ││
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌───────────┐ ││
│  │  │  Parse   │→ │   Test   │→ │  Compile │→ │   Sign   │→ │  Publish  │ ││
│  │  │  Rego    │  │  Policies│  │  Bundle  │  │  Bundle  │  │  Bundle   │ ││
│  │  └──────────┘  └──────────┘  └──────────┘  └──────────┘  └───────────┘ ││
│  └─────────────────────────────────────────────────────────────────────────┘│
│                                    │                                         │
│                                    │ publish                                 │
│                                    ▼                                         │
│  ┌─────────────────────────────────────────────────────────────────────────┐│
│  │                        Bundle Registry                                   ││
│  │                                                                          ││
│  │   users-service-v1.2.0.bundle.tar.gz                                    ││
│  │   orders-service-v1.0.0.bundle.tar.gz                                   ││
│  │   ...                                                                    ││
│  └─────────────────────────────────────────────────────────────────────────┘│
│                                    │                                         │
│                                    │ push                                    │
│                                    ▼                                         │
│  ┌─────────────────────────────────────────────────────────────────────────┐│
│  │                     Eunomia Control Plane                                ││
│  │                                                                          ││
│  │  ┌──────────────────┐  ┌──────────────────┐  ┌────────────────────────┐││
│  │  │  Bundle Manager  │  │  Push Scheduler  │  │  Rollback Controller   │││
│  │  │                  │  │                  │  │                        │││
│  │  │  - Fetch bundles │  │  - Distribute    │  │  - Track deployments   │││
│  │  │  - Verify sigs   │  │  - Health checks │  │  - Trigger rollback    │││
│  │  └──────────────────┘  └──────────────────┘  └────────────────────────┘││
│  └─────────────────────────────────────────────────────────────────────────┘│
│                                    │                                         │
│              ┌─────────────────────┼─────────────────────┐                  │
│              │                     │                     │                   │
│              ▼                     ▼                     ▼                   │
│  ┌───────────────────┐ ┌───────────────────┐ ┌───────────────────┐         │
│  │   Archimedes      │ │   Archimedes      │ │   Archimedes      │         │
│  │   Instance 1      │ │   Instance 2      │ │   Instance N      │         │
│  │                   │ │                   │ │                   │         │
│  │  ┌─────────────┐  │ │  ┌─────────────┐  │ │  ┌─────────────┐  │         │
│  │  │OPA Evaluator│  │ │  │OPA Evaluator│  │ │  │OPA Evaluator│  │         │
│  │  │  (Local)    │  │ │  │  (Local)    │  │ │  │  (Local)    │  │         │
│  │  └─────────────┘  │ │  └─────────────┘  │ │  └─────────────┘  │         │
│  └───────────────────┘ └───────────────────┘ └───────────────────┘         │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 4. Repository Structure

### 4.1 Eunomia Toolchain Repository

```
eunomia/
├── Cargo.toml                    # Workspace root
├── README.md
├── LICENSE
│
├── crates/
│   ├── eunomia-core/             # Core types and traits
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── policy.rs         # Policy model
│   │       ├── bundle.rs         # Bundle model
│   │       ├── decision.rs       # Authorization decision
│   │       └── identity.rs       # Identity types
│   │
│   ├── eunomia-compiler/         # Policy compilation
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── parser.rs         # Rego parsing
│   │       ├── analyzer.rs       # Static analysis
│   │       ├── optimizer.rs      # Policy optimization
│   │       ├── bundler.rs        # Bundle creation
│   │       ├── validator.rs      # Policy validation
│   │       ├── lint.rs           # Policy linting rules
│   │       ├── engine.rs         # Rego engine wrapper
│   │       ├── semantic.rs       # Semantic validation
│   │       └── error.rs          # Error types
│   │
│   ├── eunomia-test/             # Testing framework
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── runner.rs         # Test execution
│   │       ├── fixtures.rs       # Test fixtures
│   │       ├── coverage.rs       # Coverage analysis
│   │       └── reporter.rs       # Test reporting
│   │
│   ├── eunomia-registry/         # Bundle registry client
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── client.rs         # Registry API
│   │       ├── cache.rs          # Local bundle caching
│   │       ├── config.rs         # Registry configuration
│   │       ├── oci.rs            # OCI manifest types
│   │       └── version.rs        # Version resolution
│   │
│   ├── eunomia-distributor/      # Policy distribution
│   │   ├── Cargo.toml
│   │   ├── build.rs              # Protobuf compilation (optional)
│   │   └── src/
│   │       ├── lib.rs            # Main Distributor struct
│   │       ├── config.rs         # Distributor configuration
│   │       ├── discovery.rs      # Instance discovery (Static, K8s, DNS)
│   │       ├── error.rs          # Error types
│   │       ├── health.rs         # Health monitoring
│   │       ├── instance.rs       # Instance representation
│   │       ├── pusher.rs         # Bundle pushing
│   │       ├── scheduler.rs      # Deployment scheduling
│   │       ├── state.rs          # Deployment state tracking
│   │       └── strategy.rs       # Deployment strategies
│   │
│   └── eunomia-audit/            # Audit logging (future)
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── logger.rs         # Audit log emission
│           └── schema.rs         # Audit event schema
│
├── crates/eunomia-cli/           # CLI application
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       └── commands/
│           ├── test.rs
│           ├── build.rs
│           ├── sign.rs
│           ├── publish.rs
│           ├── fetch.rs
│           ├── push.rs           # Policy deployment CLI
│           └── validate.rs
│
├── eunomia-control-plane/        # Control plane service (future)
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       ├── api.rs                # Control plane API
│       ├── state.rs              # Deployment state
│       └── reconciler.rs         # Desired state reconciliation
│
├── eunomia-action/               # GitHub Action (future)
│   ├── action.yml
│   ├── Dockerfile
│   └── entrypoint.sh
│
├── proto/                        # Protobuf schemas
│   └── control_plane.proto       # gRPC API definitions
│
├── schemas/                      # JSON schemas (future)
│   ├── bundle.schema.json
│   └── audit-event.schema.json
│
└── examples/
    ├── simple-rbac/              # Basic RBAC example
    ├── attribute-based/          # ABAC example
    └── multi-tenant/             # Multi-tenancy example
```

### 4.2 Policy Repository Structure

```
policies/
├── .github/
│   └── workflows/
│       └── policy-ci.yml
│
├── common/                       # Shared modules
│   ├── roles.rego               # Role definitions
│   ├── roles_test.rego          # Role tests
│   ├── identity.rego            # Identity helpers
│   └── identity_test.rego
│
├── services/                     # Per-service policies
│   ├── users-service/
│   │   ├── authz.rego           # Authorization policy
│   │   ├── authz_test.rego      # Policy tests
│   │   └── data.json            # Static data (optional)
│   ├── orders-service/
│   │   ├── authz.rego
│   │   └── authz_test.rego
│   └── payments-service/
│       ├── authz.rego
│       └── authz_test.rego
│
├── bundles/                      # Bundle configurations
│   ├── users-service.yaml
│   ├── orders-service.yaml
│   └── payments-service.yaml
│
├── eunomia.yaml                  # Global configuration
└── README.md
```

---

## 5. Policy Model

### 5.1 Authorization Decision

Every authorization request produces a decision:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorizationDecision {
    /// Whether the request is allowed
    pub allowed: bool,

    /// Reason for the decision (for auditing)
    pub reason: String,

    /// Policy that made the decision
    pub policy_id: String,

    /// Policy version
    pub policy_version: String,

    /// Evaluation duration
    pub evaluation_time_ns: u64,
}
```

### 5.2 Policy Input Schema

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyInput {
    /// Caller identity
    pub caller: CallerIdentity,

    /// Target service
    pub service: String,

    /// Target operation
    pub operation_id: String,

    /// HTTP method
    pub method: String,

    /// Request path
    pub path: String,

    /// Request headers (filtered, authorization headers stripped)
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub headers: HashMap<String, String>,

    /// Request timestamp (ISO 8601)
    pub timestamp: String,

    /// Environment (production, staging, development)
    pub environment: String,

    /// Additional context (extensible)
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub context: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum CallerIdentity {
    /// Internal service (SPIFFE)
    #[serde(rename = "spiffe")]
    Spiffe {
        /// Full SPIFFE ID
        spiffe_id: String,
        /// Parsed service name
        service_name: String,
        /// Trust domain
        trust_domain: String,
    },

    /// External user
    #[serde(rename = "user")]
    User {
        /// User ID
        user_id: String,
        /// User roles
        roles: Vec<String>,
        /// Tenant ID (for multi-tenant systems)
        #[serde(skip_serializing_if = "Option::is_none")]
        tenant_id: Option<String>,
    },

    /// API key
    #[serde(rename = "api_key")]
    ApiKey {
        /// Key identifier
        key_id: String,
        /// Scopes
        scopes: Vec<String>,
    },

    /// Anonymous
    #[serde(rename = "anonymous")]
    Anonymous,
}
```

### 5.3 Rego Policy Structure

```rego
# policies/services/users-service/authz.rego
package users_service.authz

import future.keywords.if
import future.keywords.in

import data.common.roles
import data.common.identity

# Default deny
default allow := false

# Rule: Internal services with correct SPIFFE ID can call any operation
allow if {
    input.caller.type == "spiffe"
    allowed_service(input.caller.service_name)
}

# Rule: Admin users can access all operations
allow if {
    input.caller.type == "user"
    roles.is_admin(input.caller.roles)
}

# Rule: Regular users can read their own data
allow if {
    input.caller.type == "user"
    input.operation_id == "getUser"
    own_resource(input)
}

# Rule: Regular users can update their own profile
allow if {
    input.caller.type == "user"
    input.operation_id == "updateUser"
    own_resource(input)
}

# Helper: Check if accessing own resource
own_resource(input) if {
    # Extract user_id from path (e.g., /users/{userId})
    path_parts := split(input.path, "/")
    user_id := path_parts[2]
    user_id == input.caller.user_id
}

# Helper: Allowed internal services
allowed_service(service_name) if {
    service_name in {
        "orders-service",
        "notifications-service",
        "analytics-service",
    }
}
```

---

## 6. Policy Authoring

### 6.1 Policy File Conventions

```rego
# Package naming: <service>.<module>
package users_service.authz

# Import future keywords for clarity
import future.keywords.if
import future.keywords.in
import future.keywords.contains
import future.keywords.every

# Import shared modules
import data.common.roles
import data.common.identity

# Document the policy
# METADATA
# title: Users Service Authorization
# description: Controls access to user management operations
# authors:
#   - platform-team@somniatore.com
# related_resources:
#   - https://docs.somniatore.com/policies/users

# Default decision (explicit deny)
default allow := false

# Always include a reason for debugging
reason := msg if {
    allow
    msg := "access granted"
} else := msg if {
    msg := "access denied: no matching rule"
}
```

### 6.2 Common Patterns

**Role-Based Access Control (RBAC):**

```rego
# common/roles.rego
package common.roles

# Role hierarchy
role_hierarchy := {
    "admin": {"admin", "editor", "viewer"},
    "editor": {"editor", "viewer"},
    "viewer": {"viewer"},
}

# Check if user has required role (including hierarchy)
has_role(user_roles, required_role) if {
    some user_role in user_roles
    required_role in role_hierarchy[user_role]
}

is_admin(roles) if {
    has_role(roles, "admin")
}

is_editor(roles) if {
    has_role(roles, "editor")
}
```

**Service-to-Service Authorization:**

```rego
# Allow specific services to call specific operations
allow if {
    input.caller.type == "spiffe"
    allowed_service_operation[input.caller.service_name][input.operation_id]
}

allowed_service_operation := {
    "orders-service": {"getUser", "getUsersByIds"},
    "notifications-service": {"getUser", "getUserEmail"},
    "analytics-service": {"listUsers"},
}
```

**Time-Based Access:**

```rego
# Only allow during business hours
allow if {
    input.caller.type == "user"
    business_hours
    # ... other conditions
}

business_hours if {
    hour := time.clock([input.timestamp, "UTC"])[0]
    hour >= 9
    hour < 17
}
```

### 6.3 Anti-Patterns to Avoid

```rego
# ❌ BAD: Hardcoded secrets
allow if {
    input.headers["x-api-key"] == "secret123"
}

# ✅ GOOD: Reference external data
allow if {
    valid_api_key(input.headers["x-api-key"])
}

# ❌ BAD: Overly permissive
allow if {
    true
}

# ✅ GOOD: Explicit conditions
allow if {
    input.caller.type == "spiffe"
    input.caller.trust_domain == "somniatore.com"
}

# ❌ BAD: Negation without default
not_blocked if {
    not input.caller.user_id in blocked_users
}

# ✅ GOOD: Clear default and explicit rule
default blocked := false
blocked if {
    input.caller.user_id in blocked_users
}
```

### 6.4 Semantic Validation

Beyond syntax checking, Eunomia provides semantic validation to catch logical issues in policies.

#### SemanticValidator

The `SemanticValidator` performs deep policy analysis:

```rust
use eunomia_compiler::{SemanticValidator, MockServiceContract};

// Create validator with mock service contracts
let mut validator = SemanticValidator::new();

// Register service contracts for validation
let contract = MockServiceContract::new("users-service")
    .add_operation("getUser")
    .add_operation("listUsers")
    .add_operation("createUser")
    .add_operation("updateUser")
    .add_operation("deleteUser");

validator.add_contract(contract);

// Or use predefined contracts
validator.add_contract(MockServiceContract::users_service_contract());
validator.add_contract(MockServiceContract::orders_service_contract());

// Validate a policy
let issues = validator.validate(&parsed_policy)?;

for issue in &issues {
    println!("[{}] {}: {}", issue.severity, issue.category, issue.description);
    if let Some(suggestion) = &issue.suggestion {
        println!("  Suggestion: {}", suggestion);
    }
}
```

#### Validation Categories

| Category      | Description                                |
| ------------- | ------------------------------------------ |
| `OperationId` | Unknown operation IDs referenced in policy |
| `Unused`      | Rules defined but never used               |
| `Deprecated`  | Usage of deprecated input fields           |
| `Schema`      | Input structure violations                 |
| `Reference`   | Missing rule or data references            |

#### InputSchema Validation

Policies should use the standard Themis input schema:

```rust
use eunomia_compiler::InputSchema;

// Standard Themis authorization input
let schema = InputSchema::themis_standard();

// Includes: caller, service, operation_id, method, path,
//           headers, timestamp, environment, context

// Check if a policy uses deprecated fields
let deprecated = schema.get_deprecated_fields();
// Returns: ["action", "resource"]
```

#### Mock Service Contracts

For testing policies without Themis, define mock contracts:

```rust
use eunomia_compiler::MockServiceContract;

// Define operations a service supports
let contract = MockServiceContract::new("orders-service")
    .add_operation("createOrder")
    .add_operation("getOrder")
    .add_operation("cancelOrder")
    .add_operation("fulfillOrder");

// Validator will warn if policy references
// operations not in any registered contract
```

---

## 7. Policy Lifecycle

### 7.1 Workflow States

```
┌─────────────┐    PR Created    ┌─────────────┐
│   Draft     │ ───────────────► │  Proposed   │
│             │                  │             │
└─────────────┘                  └──────┬──────┘
                                        │
                                        │ Tests Pass
                                        ▼
                                 ┌─────────────┐
                                 │   Tested    │
                                 │             │
                                 └──────┬──────┘
                                        │
                                        │ Review Approved
                                        ▼
                                 ┌─────────────┐
                                 │  Approved   │
                                 │             │
                                 └──────┬──────┘
                                        │
                                        │ PR Merged
                                        ▼
                                 ┌─────────────┐
                                 │  Compiled   │
                                 │             │
                                 └──────┬──────┘
                                        │
                                        │ Bundle Published
                                        ▼
                                 ┌─────────────┐
                                 │  Published  │
                                 │             │
                                 └──────┬──────┘
                                        │
                                        │ Push to Services
                                        ▼
                                 ┌─────────────┐
                                 │  Deployed   │
                                 │             │
                                 └─────────────┘
                                        │
                                        │ (if errors)
                                        ▼
                                 ┌─────────────┐
                                 │  Rolled     │
                                 │   Back      │
                                 └─────────────┘
```

### 7.2 Change Workflow

1. **Author policy change** in feature branch
2. **Write tests** for new/modified rules
3. **Open PR** triggers CI:
   - Rego syntax validation
   - Policy tests execution
   - Coverage analysis
   - Static analysis (complexity, conflicts)
4. **Review** by policy owner and security team
5. **Merge** triggers compilation and publishing
6. **Deploy** via control plane push to Archimedes instances
7. **Monitor** for authorization failures/anomalies
8. **Rollback** if issues detected

---

## 7b. Policy Versioning

### 7b.1 Semantic Versioning Standard

All policy bundles use **semantic versioning** (SemVer 2.0.0):

```
{MAJOR}.{MINOR}.{PATCH}

Examples:
  1.0.0  - Initial release
  1.1.0  - Added new rule (backward compatible)
  1.1.1  - Bug fix in existing rule
  2.0.0  - Breaking change (removed permission)
```

### 7b.2 Version Bump Rules

| Change Type                          | Version Bump | Example                            |
| ------------------------------------ | ------------ | ---------------------------------- |
| Remove permission (more restrictive) | MAJOR        | User can no longer access endpoint |
| Change authorization logic semantics | MAJOR        | Different decision for same input  |
| Add new permission (more permissive) | MINOR        | New role can access endpoint       |
| Add new policy for new operation     | MINOR        | Policy for new `operationId`       |
| Fix bug without changing semantics   | PATCH        | Typo in role name                  |
| Performance optimization             | PATCH        | Rule reordering                    |
| Documentation updates                | PATCH        | Comments, metadata                 |

### 7b.3 Version Configuration

```yaml
# bundles/users-service.yaml
service: users-service
version:
  current: "1.2.3"
  # Auto-bump based on changes
  auto_bump: true
  # Require explicit major bumps
  require_explicit_major: true

policies:
  - services/users-service/authz.rego

dependencies:
  - common/roles.rego
  - common/identity.rego
```

### 7b.4 Version Constraints

Archimedes can specify version constraints for policy loading:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyVersionConstraint {
    /// Minimum required version (inclusive)
    pub min_version: Option<Version>,

    /// Maximum allowed version (exclusive for major)
    pub max_version: Option<Version>,

    /// Exact version pin (overrides min/max)
    pub exact_version: Option<Version>,
}

impl PolicyVersionConstraint {
    /// Parse from string constraint
    pub fn parse(s: &str) -> Result<Self, ParseError> {
        // Supports: "1.2.3", ">=1.2.0", "^1.2.0", "~1.2.0", ">=1.0.0,<2.0.0"
        // ...
    }

    /// Check if version satisfies constraint
    pub fn satisfies(&self, version: &Version) -> bool {
        if let Some(exact) = &self.exact_version {
            return version == exact;
        }

        if let Some(min) = &self.min_version {
            if version < min {
                return false;
            }
        }

        if let Some(max) = &self.max_version {
            if version >= max {
                return false;
            }
        }

        true
    }
}
```

### 7b.5 Version in Audit Log

All authorization decisions include the policy version:

```json
{
  "timestamp": "2026-01-04T12:00:00Z",
  "service": "users-service",
  "operation_id": "getUser",
  "decision": "allowed",
  "policy_version": "1.2.3",
  "policy_bundle_checksum": "sha256:abc123..."
}
```

---

## 8. Bundle Compilation

### 8.0 Design Decisions

#### 8.0.1 OPA Bundle Format Compatibility

**Decision**: Use OPA-native bundle format (tar.gz) for maximum compatibility.

**Rationale**:

- OPA and Regorus can load bundles directly without transformation
- Standard format enables tooling interoperability
- Well-documented format with ecosystem support

**Implementation**:

```rust
// Bundle is exported as tar.gz with OPA-compatible structure
bundle.write_to_tarball(path)?;

// Bundle can be loaded directly by OPA/Regorus
// opa run --bundle users-service-v1.0.0.bundle.tar.gz
```

#### 8.0.2 Bundle Manifest Format

**Decision**: Use `.manifest` JSON file at bundle root (OPA standard).

**Required Fields**:

- `revision`: Monotonically increasing revision number
- `roots`: Array of root documents (e.g., `["users_service"]`)

**Extended Fields** (Eunomia-specific):

- `version`: Semantic version (e.g., `"1.2.0"`)
- `git_commit`: Source commit SHA
- `created_at`: RFC 3339 timestamp
- `checksum`: SHA-256 of bundle contents
- `service`: Service name this bundle is for

**Example**:

```json
{
  "revision": "2026010512000000",
  "roots": ["users_service", "common"],
  "version": "1.2.0",
  "service": "users-service",
  "git_commit": "abc123def456",
  "created_at": "2026-01-05T12:00:00Z",
  "checksum": {
    "algorithm": "sha256",
    "value": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
  }
}
```

#### 8.0.3 Policy File Organization in Bundle

**Decision**: Organize policies by package namespace under root directories.

**Mapping Rules**:

- Package `users_service.authz` → `users_service/authz.rego`
- Package `common.roles` → `common/roles.rego`
- Data file `data.json` in `users-service/` → `users_service/data.json`

**Implementation**:

```rust
fn package_to_path(package: &str) -> PathBuf {
    // "users_service.authz" -> "users_service/authz.rego"
    let parts: Vec<&str> = package.split('.').collect();
    let mut path = PathBuf::new();
    for part in &parts[..parts.len() - 1] {
        path.push(part);
    }
    path.push(format!("{}.rego", parts.last().unwrap()));
    path
}
```

#### 8.0.4 Bundle Checksum Calculation

**Decision**: Use SHA-256 over canonical bundle content for integrity verification.

**Process**:

1. Sort all files alphabetically by path
2. For each file: append `path + "\n" + content + "\n"`
3. Compute SHA-256 of concatenated result
4. Store as lowercase hex string

**Rationale**: Deterministic checksums regardless of tar ordering.

#### 8.0.5 Bundle Signing

**Decision**: Use Ed25519 signatures for bundle authenticity verification.

**Algorithm**: Ed25519 (RFC 8032)

- Fast and secure elliptic curve signing
- 32-byte public keys, 64-byte signatures
- Deterministic signatures (no random nonce needed)

**Signed Content**:

- Sign the canonical bundle checksum (SHA-256 hex string)
- This ensures both integrity (via checksum) and authenticity (via signature)

**Signature Format** (`.signatures/.manifest.sig`):

```json
{
  "signatures": [
    {
      "keyid": "eunomia-prod-2026",
      "algorithm": "ed25519",
      "value": "<base64-encoded-signature>"
    }
  ]
}
```

**Key Management**:

- Key IDs are human-readable identifiers (e.g., "eunomia-prod-2026")
- Private keys stored securely (environment variable or file)
- Public keys distributed to Archimedes instances for verification
- Support for key rotation with multiple valid keys

**CLI Integration**:

```bash
# Sign a bundle
eunomia sign --bundle ./bundle.tar.gz --key-file ./private.key --key-id prod-2026

# Verify a bundle
eunomia verify --bundle ./bundle.tar.gz --public-key ./public.key
```

**Rationale**: Ed25519 is fast, secure, and widely supported. Signing the checksum rather than the full bundle content is efficient and provides the same security guarantees.

#### 8.0.6 Registry API Design

**Decision**: Use OCI Distribution Specification-compatible API for bundle registry.

**Rationale**:

- OCI registries (Docker Registry, Harbor, ECR, GCR) are battle-tested infrastructure
- Enables use of existing registry infrastructure and tooling
- Built-in content-addressable storage and garbage collection
- Supports geo-replication out of the box

**OCI Artifact Mapping**:

```
Repository: eunomia/policies/<service-name>
Tag:        v<semver>  (e.g., v1.2.0)
Digest:     sha256:<content-hash>

Media Types:
  - application/vnd.eunomia.policy.bundle.v1+tar.gz  (bundle)
  - application/vnd.eunomia.policy.manifest.v1+json  (manifest)
  - application/vnd.eunomia.policy.signature.v1+json (signature)
```

**Registry URL Format**:

```
<registry-host>/<namespace>/<service-name>:<version>

Examples:
  registry.themis.io/policies/users-service:v1.2.0
  localhost:5000/eunomia/users-service:v1.0.0
```

**API Operations**:

| Operation    | HTTP Method | Endpoint                           |
| ------------ | ----------- | ---------------------------------- |
| Check exists | HEAD        | `/v2/<name>/manifests/<reference>` |
| Get manifest | GET         | `/v2/<name>/manifests/<reference>` |
| Push blob    | POST/PUT    | `/v2/<name>/blobs/uploads/`        |
| Pull blob    | GET         | `/v2/<name>/blobs/<digest>`        |
| List tags    | GET         | `/v2/<name>/tags/list`             |
| Delete tag   | DELETE      | `/v2/<name>/manifests/<reference>` |

**Client Configuration**:

```rust
pub struct RegistryConfig {
    /// Registry URL (e.g., "https://registry.themis.io")
    pub url: String,

    /// Namespace prefix (e.g., "policies")
    pub namespace: String,

    /// Authentication method
    pub auth: RegistryAuth,

    /// Request timeout
    pub timeout: Duration,

    /// TLS configuration for mTLS
    pub tls: Option<TlsConfig>,
}

pub enum RegistryAuth {
    /// No authentication (for local development)
    None,

    /// Basic authentication (username/password or username/token)
    Basic { username: String, password: String },

    /// Bearer token (OAuth2 / service account)
    Bearer { token: String },

    /// AWS ECR (uses IAM credentials)
    AwsEcr { region: String },

    /// GCP Artifact Registry (uses ADC)
    GcpArtifact { project: String, location: String },
}
```

**Version Resolution**:

- `latest` → Most recent semantic version
- `v1.2.3` → Exact version match
- `v1.2` → Latest patch in minor version
- `v1` → Latest minor/patch in major version
- `sha256:abc...` → Exact digest match

#### 8.0.7 Bundle Caching Strategy

**Decision**: Use local file-based cache with LRU eviction.

**Cache Structure**:

```
$EUNOMIA_CACHE_DIR/
├── bundles/
│   ├── users-service/
│   │   ├── v1.2.0.bundle.tar.gz
│   │   ├── v1.2.0.manifest.json
│   │   └── v1.1.0.bundle.tar.gz
│   └── orders-service/
│       └── v1.0.0.bundle.tar.gz
├── signatures/
│   └── users-service/
│       └── v1.2.0.sig
└── cache.db  # SQLite index (optional)
```

**Cache Configuration**:

```rust
pub struct CacheConfig {
    /// Cache directory (default: ~/.eunomia/cache)
    pub dir: PathBuf,

    /// Maximum cache size in bytes (default: 1GB)
    pub max_size: u64,

    /// Time-to-live for cache entries (default: 7 days)
    pub ttl: Duration,

    /// Enable cache integrity verification
    pub verify_checksums: bool,
}
```

**Cache Operations**:

- `get(service, version)` → Returns cached bundle if present and valid
- `put(service, version, bundle)` → Stores bundle, evicts LRU if needed
- `invalidate(service, version)` → Removes specific entry
- `clear()` → Removes all cached entries
- `prune()` → Removes expired entries, enforces size limit

### 8.1 Bundle Structure

```
users-service-v1.2.0.bundle.tar.gz
├── .manifest                   # OPA bundle manifest (JSON)
├── users_service/              # Root namespace directory
│   ├── authz.rego             # Main authorization policy
│   └── data.json              # Optional static data
├── common/                     # Shared modules
│   ├── roles.rego
│   └── identity.rego
└── .signatures/                # Optional signatures
    └── .manifest.sig          # Ed25519 signature of manifest
```

### 8.2 Manifest Format

The `.manifest` file follows OPA's bundle specification with Eunomia extensions:

```json
{
  "revision": "2026010512000000",
  "roots": ["users_service", "common"],
  "metadata": {
    "eunomia": {
      "version": "1.2.0",
      "service": "users-service",
      "git_commit": "abc123def456",
      "git_repository": "github.com/somniatore/policies",
      "created_at": "2026-01-05T12:00:00Z",
      "author": "platform-team",
      "change_summary": "Added support for API key authentication"
    },
    "checksum": {
      "algorithm": "sha256",
      "value": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    }
  }
}
```

**OPA Standard Fields**:

| Field      | Type     | Description                        |
| ---------- | -------- | ---------------------------------- |
| `revision` | string   | Unique bundle revision (timestamp) |
| `roots`    | string[] | Root documents exposed by bundle   |
| `metadata` | object   | Custom metadata (extensible)       |

**Eunomia Extension Fields** (under `metadata.eunomia`):

| Field            | Type   | Description                          |
| ---------------- | ------ | ------------------------------------ |
| `version`        | string | Semantic version (SemVer 2.0)        |
| `service`        | string | Target service name                  |
| `git_commit`     | string | Source commit SHA                    |
| `git_repository` | string | Source repository URL                |
| `created_at`     | string | Bundle creation timestamp (RFC 3339) |
| `author`         | string | Bundle author/team                   |
| `change_summary` | string | Description of changes               |

### 8.3 Compilation Process

```rust
pub struct BundleCompiler {
    config: CompilerConfig,
}

impl BundleCompiler {
    pub async fn compile(&self, policy_dir: &Path, service: &str) -> Result<Bundle, CompileError> {
        // 1. Parse all Rego files
        let modules = self.parse_rego_files(policy_dir).await?;

        // 2. Resolve dependencies
        let resolved = self.resolve_dependencies(&modules, service)?;

        // 3. Validate policy structure
        self.validate_policy_structure(&resolved)?;

        // 4. Optimize (optional)
        let optimized = if self.config.optimize {
            self.optimize_policy(&resolved)?
        } else {
            resolved
        };

        // 5. Create bundle
        let bundle = Bundle::new(service, optimized);

        // 6. Sign bundle
        let signed = self.sign_bundle(bundle)?;

        Ok(signed)
    }

    fn validate_policy_structure(&self, modules: &ResolvedModules) -> Result<(), CompileError> {
        // Verify entrypoint exists
        if !modules.has_rule(&self.config.entrypoint) {
            return Err(CompileError::MissingEntrypoint(self.config.entrypoint.clone()));
        }

        // Verify default deny
        if !modules.has_default_deny() {
            return Err(CompileError::MissingDefaultDeny);
        }

        // Check for policy conflicts
        let conflicts = modules.find_conflicts();
        if !conflicts.is_empty() {
            return Err(CompileError::PolicyConflicts(conflicts));
        }

        Ok(())
    }
}
```

---

## 9. Policy Distribution (Hybrid Push/Pull)

### 9.1 Hybrid Distribution Model

Eunomia uses a **hybrid push/pull model** for maximum reliability:

- **Primary: Push** – Control plane actively pushes policy updates to Archimedes instances
- **Fallback: Pull** – Archimedes instances can pull from registry if push fails or on startup

```
┌─────────────────┐                    ┌─────────────────┐
│    Eunomia      │                    │   Archimedes    │
│  Control Plane  │                    │    Instance     │
│                 │                    │                 │
│  ┌───────────┐  │   PUSH (Primary)   │  ┌───────────┐  │
│  │  Pusher   │──┼────────────────────►│  Control  │  │
│  │           │  │   POST /control/   │  │  Endpoint │  │
│  └───────────┘  │     policy-update  │  └─────┬─────┘  │
│                 │                    │        │        │
│                 │                    │        ▼        │
│                 │                    │  ┌───────────┐  │
│                 │                    │  │  Policy   │  │
│                 │                    │  │  Loader   │  │
│                 │                    │  └─────┬─────┘  │
│                 │                    │        │        │
│                 │                    │        ▼        │
│                 │                    │  ┌───────────┐  │
│                 │                    │  │   Local   │  │
│                 │                    │  │   Cache   │  │
│                 │                    │  └───────────┘  │
└─────────────────┘                    └────────┬────────┘
                                                │
         ┌──────────────────────────────────────┘
         │ PULL (Fallback/Startup)
         ▼
┌─────────────────────────────────────────────────────────┐
│                    Bundle Registry                       │
│                   (OCI Registry)                         │
│                                                          │
│   users-service/v1.2.0.bundle.tar.gz                    │
│   orders-service/v1.0.0.bundle.tar.gz                   │
└─────────────────────────────────────────────────────────┘
```

### 9.2 Push Distribution (Primary)

```rust
pub struct PolicyDistributor {
    registry: BundleRegistry,
    service_discovery: ServiceDiscovery,
    config: DistributorConfig,
}

impl PolicyDistributor {
    pub async fn distribute(&self, bundle: &Bundle) -> Result<DistributionReport, DistributeError> {
        let service = &bundle.manifest.service;

        // 1. Discover all instances of the service
        let instances = self.service_discovery.find_instances(service).await?;

        // 2. Group by deployment strategy
        let groups = self.group_by_strategy(&instances);

        // 3. Push to each group (canary, then rolling)
        let mut report = DistributionReport::new();

        // Canary deployment (if configured)
        if let Some(canary) = groups.canary {
            let canary_result = self.push_to_instances(&canary, bundle).await?;
            report.add_phase("canary", canary_result);

            // Wait and verify canary health
            self.wait_and_verify(&canary, self.config.canary_duration).await?;
        }

        // Rolling deployment to remaining instances
        for batch in groups.rolling.chunks(self.config.batch_size) {
            let batch_result = self.push_to_instances(batch, bundle).await?;
            report.add_phase("rolling", batch_result);

            // Brief pause between batches
            tokio::time::sleep(self.config.batch_delay).await;
        }

        Ok(report)
    }

    async fn push_to_instances(&self, instances: &[Instance], bundle: &Bundle) -> Result<PushResult, PushError> {
        let mut results = Vec::new();

        for instance in instances {
            let result = self.push_to_instance(instance, bundle).await;
            results.push((instance.id.clone(), result));
        }

        PushResult::from_results(results)
    }

    async fn push_to_instance(&self, instance: &Instance, bundle: &Bundle) -> Result<(), PushError> {
        let client = self.create_mtls_client(&instance.control_endpoint)?;

        let response = client
            .post(&format!("{}/control/policy-update", instance.control_endpoint))
            .json(&PolicyUpdateRequest {
                bundle_url: bundle.registry_url.clone(),
                manifest: bundle.manifest.clone(),
            })
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(PushError::InstanceRejected {
                instance: instance.id.clone(),
                status: response.status(),
                body: response.text().await?,
            });
        }

        Ok(())
    }
}
```

### 9.3 Pull Distribution (Fallback)

Archimedes instances can pull policies in these scenarios:

- **Startup**: Load policy from registry before control plane connection established
- **Push failure**: If control plane is unreachable
- **Recovery**: After outage, sync to latest version
- **Periodic refresh**: Optional background sync as health check

```rust
/// Archimedes-side policy puller
pub struct PolicyPuller {
    registry: RegistryClient,
    config: PullerConfig,
    cache: PolicyCache,
}

impl PolicyPuller {
    /// Pull latest policy for a service
    pub async fn pull_latest(&self, service: &str) -> Result<Bundle, PullError> {
        // 1. Query registry for latest version
        let latest_version = self.registry.get_latest_version(service).await?;

        // 2. Check if we already have this version cached
        if let Some(cached) = self.cache.get(service, &latest_version).await? {
            tracing::debug!(service, version = %latest_version, "using cached policy");
            return Ok(cached);
        }

        // 3. Download bundle
        let bundle = self.registry.fetch_bundle(service, &latest_version).await?;

        // 4. Verify signature
        bundle.verify_signature(&self.config.signing_keys)?;

        // 5. Cache for future use
        self.cache.store(service, &bundle).await?;

        tracing::info!(service, version = %latest_version, "pulled and cached policy");
        Ok(bundle)
    }

    /// Background sync task
    pub async fn background_sync(&self, service: &str, interval: Duration) {
        let mut ticker = tokio::time::interval(interval);

        loop {
            ticker.tick().await;

            match self.pull_latest(service).await {
                Ok(bundle) => {
                    // Notify policy loader of potential update
                    self.notify_update(&bundle).await;
                }
                Err(e) => {
                    tracing::warn!(error = %e, "background policy sync failed");
                    // Continue with cached policy
                }
            }
        }
    }
}
```

### 9.4 Graceful Degradation

When neither push nor pull is available:

```rust
/// Policy loading with graceful degradation
pub async fn load_policy(service: &str, config: &PolicyConfig) -> Result<Policy, PolicyError> {
    // 1. Try to get pushed policy (already in memory)
    if let Some(policy) = PUSHED_POLICIES.get(service) {
        return Ok(policy.clone());
    }

    // 2. Try to pull from registry
    match puller.pull_latest(service).await {
        Ok(bundle) => return Ok(bundle.into_policy()),
        Err(e) => tracing::warn!(error = %e, "pull failed, trying cache"),
    }

    // 3. Fall back to local cache (SQLite)
    if let Some(cached) = cache.get_latest_cached(service).await? {
        tracing::warn!(service, "using stale cached policy");
        return Ok(cached);
    }

    // 4. Fall back to embedded default (if configured)
    if let Some(default) = config.default_policy.as_ref() {
        tracing::error!(service, "using default deny-all policy");
        return Ok(default.clone());
    }

    Err(PolicyError::NoPolicyAvailable(service.to_string()))
}
```

### 9.5 Rollback Mechanism

The `RollbackController` manages rollback operations, including version history tracking, auto-rollback on health failures, and cooldown management.

```rust
/// Configuration for the rollback controller.
pub struct RollbackConfig {
    /// Enable automatic rollback on health failures.
    pub auto_rollback_enabled: bool,
    /// Number of consecutive health check failures before triggering auto-rollback.
    pub failure_threshold: u32,
    /// Time window for counting failures.
    pub failure_window: Duration,
    /// Cooldown period between auto-rollbacks.
    pub cooldown_period: Duration,
    /// Maximum rollback history entries to keep per service.
    pub max_history_entries: usize,
}

/// Controller for managing rollbacks.
pub struct RollbackController {
    config: RollbackConfig,
    state: Arc<RwLock<RollbackState>>,
    audit_logger: Option<Arc<AuditLogger>>,
}

impl RollbackController {
    /// Creates a new rollback controller with optional audit logging.
    pub fn with_audit_logger(config: RollbackConfig, logger: Arc<AuditLogger>) -> Self;

    /// Records a successful deployment for version tracking.
    pub fn record_deployment(&self, service: &str, version: &str, deployment_id: &str);

    /// Records a health check failure for auto-rollback evaluation.
    pub fn record_health_failure(&self, service: &str);

    /// Checks if an auto-rollback should be triggered.
    /// Returns Some(target_version) if conditions are met.
    pub fn should_auto_rollback(&self, service: &str) -> Option<String>;

    /// Validates and logs the start of a rollback.
    pub fn prepare_rollback(&self, trigger: &RollbackTrigger) -> Result<String>;

    /// Records a completed rollback with audit logging.
    pub fn record_rollback(&self, result: RollbackResult);

    /// Gets the previous version for rollback targeting.
    pub fn get_previous_version(&self, service: &str) -> Option<String>;

    /// Validates that a rollback can be performed.
    pub fn validate_rollback(&self, trigger: &RollbackTrigger) -> Result<()>;
}
```

#### Rollback Triggers

```rust
/// Trigger types for initiating a rollback.
pub struct RollbackTrigger {
    pub service: String,
    pub target_version: String,
    pub reason: String,
    pub is_automatic: bool,
    pub target_instances: Option<Vec<String>>,
    pub force: bool,  // Bypass validation checks
}

impl RollbackTrigger {
    /// Manual rollback with validation.
    pub fn manual(service: &str, target_version: &str, reason: &str) -> Self;

    /// Forced rollback that bypasses validation.
    pub fn forced(service: &str, target_version: &str, reason: &str) -> Self;

    /// Automatic rollback triggered by health monitor.
    pub fn automatic(service: &str, target_version: &str, reason: &str) -> Self;
}
```

#### Auto-Rollback Flow

```
┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
│  Health Monitor │────▶│ RollbackController│────▶│   Distributor   │
│                 │     │                  │     │                 │
│ record_failure()│     │ should_auto_     │     │  rollback()     │
│                 │     │   rollback()     │     │                 │
└─────────────────┘     └──────────────────┘     └─────────────────┘
                               │
                               ▼
                        ┌──────────────────┐
                        │   AuditLogger    │
                        │                  │
                        │ rollback_started │
                        │ rollback_complete│
                        └──────────────────┘
```

---

## 10. Control Plane

### 10.1 Control Plane API

````protobuf
// proto/control_plane.proto
syntax = "proto3";

package eunomia.control.v1;

service ControlPlane {
  // Policy management
  rpc DeployPolicy(DeployPolicyRequest) returns (DeployPolicyResponse);
  rpc RollbackPolicy(RollbackPolicyRequest) returns (RollbackPolicyResponse);
  rpc GetPolicyStatus(GetPolicyStatusRequest) returns (PolicyStatus);

  // Instance management
  rpc ListInstances(ListInstancesRequest) returns (ListInstancesResponse);
  rpc GetInstanceHealth(GetInstanceHealthRequest) returns (InstanceHealth);

  // Event streaming
  rpc WatchDeployment(WatchDeploymentRequest) returns (stream DeploymentEvent);

  // Audit
  rpc GetAuditLog(GetAuditLogRequest) returns (stream AuditEvent);
}
}

### 10.1.1 Deployment Event Streaming

The `EventBus` provides real-time streaming of deployment events to connected clients.

```rust
/// Event types for deployment lifecycle events.
pub enum EventType {
    DeploymentStarted,
    InstanceUpdateStarted,
    InstanceUpdateCompleted,
    InstanceUpdateFailed,
    InstanceSkipped,
    BatchCompleted,
    CanaryValidationStarted,
    CanaryValidationPassed,
    CanaryValidationFailed,
    DeploymentCompleted,
    DeploymentFailed,
    RollbackStarted,
    RollbackCompleted,
}

/// Deployment event data.
pub struct DeploymentEventData {
    pub deployment_id: String,
    pub event_type: EventType,
    pub timestamp: DateTime<Utc>,
    pub service: String,
    pub version: String,
    pub instance_id: Option<String>,
    pub message: String,
    pub progress: Option<u8>,
    pub metadata: Option<serde_json::Value>,
}

/// Event bus for publishing and subscribing to deployment events.
pub struct EventBus {
    sender: broadcast::Sender<DeploymentEventData>,
}

impl EventBus {
    /// Creates a new event bus with the given capacity.
    pub fn new(capacity: usize) -> Self;

    /// Publishes an event to all subscribers.
    pub fn publish(&self, event: DeploymentEventData) -> usize;

    /// Subscribes to receive events.
    pub fn subscribe(&self) -> EventSubscriber;
}

/// Filtered event subscriber.
impl EventSubscriber {
    /// Receives the next event.
    pub async fn recv(&mut self) -> Option<DeploymentEventData>;

    /// Filters events for a specific deployment.
    pub fn filter_deployment(self, deployment_id: String) -> FilteredSubscriber;

    /// Filters events for a specific service.
    pub fn filter_service(self, service: String) -> FilteredSubscriber;
}
````

#### Event Flow

```
┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
│   Distributor   │────▶│    EventBus      │────▶│  gRPC Stream    │
│                 │     │                  │     │                 │
│  push_to_inst() │     │   broadcast      │     │ WatchDeployment │
│                 │     │   channel        │     │                 │
└─────────────────┘     └──────────────────┘     └─────────────────┘
                               │
                    ┌──────────┴──────────┐
                    ▼                     ▼
            ┌──────────────┐      ┌──────────────┐
            │ Subscriber 1 │      │ Subscriber 2 │
            │  (filtered)  │      │  (all events)│
            └──────────────┘      └──────────────┘
```

message DeployPolicyRequest {
string service = 1;
string version = 2;
DeploymentStrategy strategy = 3;
}

message DeploymentStrategy {
enum Type {
IMMEDIATE = 0;
CANARY = 1;
ROLLING = 2;
}
Type type = 1;
int32 canary_percentage = 2;
Duration canary_duration = 3;
int32 batch_size = 4;
}

message PolicyStatus {
string service = 1;
string current_version = 2;
string previous_version = 3;
DeploymentState state = 4;
repeated InstancePolicyStatus instances = 5;
}

enum DeploymentState {
UNKNOWN = 0;
DEPLOYED = 1;
DEPLOYING = 2;
ROLLING_BACK = 3;
FAILED = 4;
}

````

### 10.2 State Management

```rust
pub struct DeploymentState {
    db: Database,
}

impl DeploymentState {
    pub async fn record_deployment(&self, deployment: &Deployment) -> Result<(), StateError> {
        sqlx::query!(
            r#"
            INSERT INTO policy_deployments (
                id, service, version, git_commit,
                started_at, completed_at, status, instances
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
            deployment.id,
            deployment.service,
            deployment.version,
            deployment.git_commit,
            deployment.started_at,
            deployment.completed_at,
            deployment.status.to_string(),
            serde_json::to_value(&deployment.instances)?,
        )
        .execute(&self.db.pool)
        .await?;

        Ok(())
    }

    pub async fn get_deployment_history(&self, service: &str, limit: i32) -> Result<Vec<Deployment>, StateError> {
        let deployments = sqlx::query_as!(
            DeploymentRow,
            r#"
            SELECT * FROM policy_deployments
            WHERE service = $1
            ORDER BY started_at DESC
            LIMIT $2
            "#,
            service,
            limit,
        )
        .fetch_all(&self.db.pool)
        .await?;

        Ok(deployments.into_iter().map(Deployment::from).collect())
    }
}
````

---

## 10b. Resilience & Caching

### 10b.1 Local Policy Cache

Each Archimedes instance maintains a local SQLite cache for resilience:

```rust
/// Local policy cache using SQLite
pub struct PolicyCache {
    db: SqlitePool,
    encryption_key: [u8; 32],
}

impl PolicyCache {
    pub async fn new(cache_path: &Path) -> Result<Self, CacheError> {
        let db = SqlitePool::connect(&format!("sqlite:{}", cache_path.display())).await?;

        // Run migrations
        sqlx::migrate!("./migrations/policy_cache").run(&db).await?;

        Ok(Self {
            db,
            encryption_key: Self::derive_key()?,
        })
    }

    /// Store a policy bundle in cache
    pub async fn store(&self, service: &str, bundle: &Bundle) -> Result<(), CacheError> {
        let encrypted = self.encrypt_bundle(bundle)?;

        sqlx::query!(
            r#"
            INSERT OR REPLACE INTO policy_cache (
                service, version, bundle_data, checksum, cached_at, expires_at
            ) VALUES ($1, $2, $3, $4, $5, $6)
            "#,
            service,
            bundle.manifest.version,
            encrypted,
            bundle.manifest.checksum.value,
            Utc::now(),
            Utc::now() + Duration::days(7), // Cache for 7 days
        )
        .execute(&self.db)
        .await?;

        Ok(())
    }

    /// Get cached policy, checking expiration
    pub async fn get(&self, service: &str, version: &str) -> Result<Option<Bundle>, CacheError> {
        let row = sqlx::query!(
            r#"
            SELECT bundle_data, expires_at FROM policy_cache
            WHERE service = $1 AND version = $2 AND expires_at > $3
            "#,
            service,
            version,
            Utc::now(),
        )
        .fetch_optional(&self.db)
        .await?;

        match row {
            Some(r) => {
                let bundle = self.decrypt_bundle(&r.bundle_data)?;
                Ok(Some(bundle))
            }
            None => Ok(None),
        }
    }

    /// Get latest cached version (fallback when registry unavailable)
    pub async fn get_latest_cached(&self, service: &str) -> Result<Option<Bundle>, CacheError> {
        let row = sqlx::query!(
            r#"
            SELECT bundle_data FROM policy_cache
            WHERE service = $1
            ORDER BY cached_at DESC
            LIMIT 1
            "#,
            service,
        )
        .fetch_optional(&self.db)
        .await?;

        match row {
            Some(r) => {
                let bundle = self.decrypt_bundle(&r.bundle_data)?;
                tracing::warn!(service, "using stale cached policy (registry unavailable)");
                Ok(Some(bundle))
            }
            None => Ok(None),
        }
    }
}
```

### 10b.2 Cache Database Schema

```sql
-- migrations/policy_cache/001_initial.sql
CREATE TABLE IF NOT EXISTS policy_cache (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    service TEXT NOT NULL,
    version TEXT NOT NULL,
    bundle_data BLOB NOT NULL,
    checksum TEXT NOT NULL,
    cached_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    expires_at TIMESTAMP NOT NULL,

    UNIQUE(service, version)
);

CREATE INDEX idx_policy_cache_service ON policy_cache(service);
CREATE INDEX idx_policy_cache_expires ON policy_cache(expires_at);

-- Track cache hits for metrics
CREATE TABLE IF NOT EXISTS cache_metrics (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    service TEXT NOT NULL,
    hit_type TEXT NOT NULL, -- 'hit', 'miss', 'stale'
    timestamp TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
```

### 10b.3 Health Check Integration

```rust
/// Policy health check for Archimedes instances
pub struct PolicyHealthCheck {
    cache: PolicyCache,
    registry: RegistryClient,
    config: HealthCheckConfig,
}

impl PolicyHealthCheck {
    /// Check policy health status
    pub async fn check(&self, service: &str) -> PolicyHealth {
        let mut health = PolicyHealth::default();

        // 1. Check if we have a loaded policy
        if let Some(loaded) = self.get_loaded_policy(service) {
            health.loaded = true;
            health.loaded_version = Some(loaded.version.clone());
        }

        // 2. Check registry connectivity
        match self.registry.get_latest_version(service).await {
            Ok(latest) => {
                health.registry_available = true;
                health.latest_version = Some(latest.clone());

                // Check if we're behind
                if let Some(ref loaded) = health.loaded_version {
                    health.up_to_date = loaded == &latest;
                }
            }
            Err(e) => {
                health.registry_available = false;
                health.registry_error = Some(e.to_string());
            }
        }

        // 3. Check cache status
        if let Ok(Some(cached)) = self.cache.get_latest_cached(service).await {
            health.cached = true;
            health.cached_version = Some(cached.manifest.version.clone());
        }

        // 4. Determine overall status
        health.status = if health.loaded && health.up_to_date {
            HealthStatus::Healthy
        } else if health.loaded {
            HealthStatus::Degraded
        } else {
            HealthStatus::Unhealthy
        };

        health
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct PolicyHealth {
    pub status: HealthStatus,
    pub loaded: bool,
    pub loaded_version: Option<String>,
    pub cached: bool,
    pub cached_version: Option<String>,
    pub registry_available: bool,
    pub latest_version: Option<String>,
    pub up_to_date: bool,
    pub registry_error: Option<String>,
}
```

### 10b.4 Graceful Degradation Strategy

```rust
/// Degradation levels for policy loading
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DegradationLevel {
    /// Normal operation - using pushed/pulled latest policy
    Normal,

    /// Using cached policy (registry temporarily unavailable)
    CachedFallback,

    /// Using stale cached policy (expired but still valid)
    StaleFallback,

    /// Using embedded default policy (deny-all)
    DefaultFallback,
}

impl DegradationLevel {
    pub fn should_alert(&self) -> bool {
        matches!(self, Self::StaleFallback | Self::DefaultFallback)
    }

    pub fn metric_label(&self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::CachedFallback => "cached",
            Self::StaleFallback => "stale",
            Self::DefaultFallback => "default",
        }
    }
}
```

---

## 11. Testing Framework

### 11.0 Design Decisions

#### 11.0.1 Import Resolution Strategy

**Decision**: Load all `.rego` files from the policy directory into a single `RegoEngine` instance before running tests.

**Rationale**:

- Tests commonly use `import data.<package>` to reference the policy under test
- Loading all policies together allows OPA's module system to resolve references
- This matches how OPA's native `opa test` command works

**Implementation**:

```rust
// TestRunner.run_suite() loads all policy files first
let mut engine = RegoEngine::new();

// Load ALL .rego files (both policies and tests)
for (path, source) in suite.policy_files() {
    engine.add_policy(&name, source)?;
}

// Then execute each test rule
for test in suite.tests() {
    engine.eval_bool(&test.qualified_name)?;
}
```

#### 11.0.2 Fixture File Conventions

**Decision**: Use `*_fixtures.json` or `*_fixtures.yaml` naming pattern.

**Structure**:

```json
{
  "name": "users_service_authz",
  "package": "users_service.authz",
  "fixtures": [
    {
      "name": "admin_can_delete_user",
      "description": "Admin role grants full access",
      "input": { "caller": { "type": "user", "roles": ["admin"] } },
      "expected_allowed": true
    }
  ]
}
```

**Discovery**: Fixtures are discovered alongside test files and loaded when present.

#### 11.0.3 Mock Identity Builders

**Decision**: Provide separate builder types for each identity type with factory methods for common scenarios.

**Implementation** (in `eunomia_test::mock_identity`):

```rust
use eunomia_test::{MockUser, MockSpiffe, MockApiKey};

// User identity with factory methods
let admin = MockUser::admin();  // Pre-configured admin user
let viewer = MockUser::viewer();
let custom = MockUser::new("user-123")
    .with_roles(vec!["custom-role"])
    .with_tenant("tenant-1")
    .build();

// SPIFFE service identity with factory methods
let orders = MockSpiffe::orders_service();  // Pre-configured
let gateway = MockSpiffe::gateway();
let custom = MockSpiffe::new("custom-service")
    .with_trust_domain("example.com")
    .with_namespace("production")
    .build();

// API key identity with factory methods
let read_only = MockApiKey::read_only();
let full = MockApiKey::full_access();
let custom = MockApiKey::new("key-123")
    .with_scopes(vec!["users:read", "users:write"])
    .build();
```

**Factory Methods Available**:

| Type         | Factory Methods                                                     |
| ------------ | ------------------------------------------------------------------- |
| `MockUser`   | `admin()`, `viewer()`, `editor()`, `guest()`, `super_admin()`       |
| `MockSpiffe` | `users_service()`, `orders_service()`, `gateway()`                  |
| `MockApiKey` | `read_only()`, `full_access()`, `read_service()`, `write_service()` |

#### 11.0.4 Test Utilities

**Decision**: Provide a utilities module with common test helpers.

**Available in `eunomia_test::test_utils`**:

```rust
use eunomia_test::test_utils::{InputBuilder, assert_allowed, assert_denied};

// Fluent input builder
let input = InputBuilder::new()
    .user("user-123", vec!["admin"])
    .operation("getUser")
    .method("GET")
    .path("/users/123")
    .build();

// Assertion helpers
assert_allowed(&engine, &input)?;
assert_denied(&engine, &input)?;
```

**Policy Generators** (for testing the test framework):

```rust
use eunomia_test::test_utils::{simple_allow_policy, role_based_policy};

// Generate simple policies for testing
let policy = simple_allow_policy("test.authz");
let rbac_policy = role_based_policy("test.authz", &["admin", "editor"]);
```

### 11.1 Test File Structure

```rego
# policies/services/users-service/authz_test.rego
package users_service.authz_test

import future.keywords.if
import data.users_service.authz

# Test: Admin can access any operation
test_admin_can_access_any_operation if {
    authz.allow with input as {
        "caller": {
            "type": "user",
            "user_id": "admin-123",
            "roles": ["admin"],
        },
        "service": "users-service",
        "operation_id": "deleteUser",
        "method": "DELETE",
        "path": "/users/456",
    }
}

# Test: Regular user cannot delete other users
test_user_cannot_delete_others if {
    not authz.allow with input as {
        "caller": {
            "type": "user",
            "user_id": "user-123",
            "roles": ["viewer"],
        },
        "service": "users-service",
        "operation_id": "deleteUser",
        "method": "DELETE",
        "path": "/users/456",
    }
}

# Test: User can read own profile
test_user_can_read_own_profile if {
    authz.allow with input as {
        "caller": {
            "type": "user",
            "user_id": "user-123",
            "roles": ["viewer"],
        },
        "service": "users-service",
        "operation_id": "getUser",
        "method": "GET",
        "path": "/users/user-123",
    }
}

# Test: Internal service with valid SPIFFE can access
test_internal_service_allowed if {
    authz.allow with input as {
        "caller": {
            "type": "spiffe",
            "spiffe_id": "spiffe://somniatore.com/orders-service",
            "service_name": "orders-service",
            "trust_domain": "somniatore.com",
        },
        "service": "users-service",
        "operation_id": "getUser",
        "method": "GET",
        "path": "/users/123",
    }
}

# Test: Unknown service denied
test_unknown_service_denied if {
    not authz.allow with input as {
        "caller": {
            "type": "spiffe",
            "spiffe_id": "spiffe://somniatore.com/malicious-service",
            "service_name": "malicious-service",
            "trust_domain": "somniatore.com",
        },
        "service": "users-service",
        "operation_id": "getUser",
        "method": "GET",
        "path": "/users/123",
    }
}

# Test: Anonymous access denied
test_anonymous_denied if {
    not authz.allow with input as {
        "caller": {
            "type": "anonymous",
        },
        "service": "users-service",
        "operation_id": "getUser",
        "method": "GET",
        "path": "/users/123",
    }
}
```

### 11.2 Test Runner

```rust
pub struct TestRunner {
    opa: OpaEngine,
    config: TestConfig,
}

impl TestRunner {
    pub async fn run_tests(&self, policy_dir: &Path) -> TestReport {
        let mut report = TestReport::new();

        // Find all test files
        let test_files = self.find_test_files(policy_dir).await;

        for test_file in test_files {
            let module = self.opa.parse_module(&test_file)?;

            // Find all test rules (prefixed with "test_")
            for rule in module.rules() {
                if rule.name().starts_with("test_") {
                    let result = self.run_test(&rule).await;
                    report.add_result(test_file.clone(), rule.name(), result);
                }
            }
        }

        report
    }

    async fn run_test(&self, rule: &Rule) -> TestResult {
        let start = Instant::now();

        match self.opa.eval_bool(rule.qualified_name()) {
            Ok(true) => TestResult::Passed {
                duration: start.elapsed(),
            },
            Ok(false) => TestResult::Failed {
                duration: start.elapsed(),
                reason: "test returned false".to_string(),
            },
            Err(e) => TestResult::Error {
                duration: start.elapsed(),
                error: e.to_string(),
            },
        }
    }
}
```

### 11.3 Coverage Analysis

```rust
pub struct CoverageAnalyzer {
    opa: OpaEngine,
}

impl CoverageAnalyzer {
    pub async fn analyze(&self, policy_dir: &Path, test_dir: &Path) -> CoverageReport {
        // 1. Parse all policy rules
        let policy_rules = self.extract_rules(policy_dir).await;

        // 2. Run tests with coverage tracking
        let coverage_data = self.run_with_coverage(test_dir).await;

        // 3. Calculate coverage
        let mut report = CoverageReport::new();

        for rule in policy_rules {
            let hits = coverage_data.hits_for_rule(&rule);
            report.add_rule(rule, hits > 0, hits);
        }

        report
    }
}

pub struct CoverageReport {
    pub total_rules: usize,
    pub covered_rules: usize,
    pub coverage_percentage: f64,
    pub uncovered_rules: Vec<String>,
    pub rule_details: Vec<RuleCoverage>,
}
```

---

## 12. CI Pipeline

### 12.1 GitHub Action

```yaml
# .github/workflows/policy-ci.yml
name: Policy CI

on:
  push:
    paths:
      - "policies/**"
  pull_request:
    paths:
      - "policies/**"

jobs:
  validate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install OPA
        uses: open-policy-agent/setup-opa@v2
        with:
          version: latest

      - name: Validate Rego Syntax
        run: |
          opa check policies/

  test:
    runs-on: ubuntu-latest
    needs: validate
    steps:
      - uses: actions/checkout@v4

      - name: Run Policy Tests
        uses: somniatore/eunomia-action@v1
        with:
          command: test
          policy-path: ./policies
          coverage-threshold: 80

      - name: Upload Coverage Report
        uses: actions/upload-artifact@v4
        with:
          name: coverage-report
          path: coverage.html

  analyze:
    runs-on: ubuntu-latest
    needs: validate
    steps:
      - uses: actions/checkout@v4

      - name: Static Analysis
        uses: somniatore/eunomia-action@v1
        with:
          command: analyze
          policy-path: ./policies
          fail-on-warnings: true

  compile:
    runs-on: ubuntu-latest
    needs: [validate, test, analyze]
    if: github.ref == 'refs/heads/main'
    steps:
      - uses: actions/checkout@v4

      - name: Compile Bundles
        uses: somniatore/eunomia-action@v1
        with:
          command: compile
          policy-path: ./policies
          output-path: ./bundles

      - name: Sign Bundles
        uses: somniatore/eunomia-action@v1
        with:
          command: sign
          bundles-path: ./bundles
          signing-key: ${{ secrets.POLICY_SIGNING_KEY }}

      - name: Publish to Registry
        uses: somniatore/eunomia-action@v1
        with:
          command: publish
          bundles-path: ./bundles
          registry-url: ${{ secrets.EUNOMIA_REGISTRY_URL }}
          registry-token: ${{ secrets.EUNOMIA_REGISTRY_TOKEN }}
```

### 12.2 Static Analysis Rules

```yaml
# eunomia-analysis.yaml
rules:
  # Require default deny
  security/default-deny:
    enabled: true
    severity: error

  # No hardcoded secrets
  security/no-hardcoded-secrets:
    enabled: true
    severity: error
    patterns:
      - "password"
      - "secret"
      - "api_key"
      - "token"

  # Complexity limits
  complexity/max-rule-depth:
    enabled: true
    severity: warning
    max-depth: 5

  complexity/max-rules-per-package:
    enabled: true
    severity: warning
    max-rules: 20

  # Test coverage
  coverage/minimum:
    enabled: true
    severity: error
    threshold: 80

  # Documentation
  docs/package-metadata:
    enabled: true
    severity: warning
    require:
      - title
      - description
```

---

## 13. Observability

### 13.1 Audit Events

```rust
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "event_type")]
pub enum AuditEvent {
    #[serde(rename = "policy_created")]
    PolicyCreated {
        service: String,
        version: String,
        author: String,
        git_commit: String,
        timestamp: DateTime<Utc>,
    },

    #[serde(rename = "policy_deployed")]
    PolicyDeployed {
        service: String,
        version: String,
        instances: Vec<String>,
        strategy: String,
        timestamp: DateTime<Utc>,
    },

    #[serde(rename = "policy_rollback")]
    PolicyRollback {
        service: String,
        from_version: String,
        to_version: String,
        reason: String,
        timestamp: DateTime<Utc>,
    },

    #[serde(rename = "authorization_decision")]
    AuthorizationDecision {
        service: String,
        operation_id: String,
        caller_type: String,
        caller_id: String,
        allowed: bool,
        policy_version: String,
        evaluation_time_ns: u64,
        request_id: String,
        timestamp: DateTime<Utc>,
    },
}
```

### 13.2 Metrics

| Metric                                              | Type      | Labels                                |
| --------------------------------------------------- | --------- | ------------------------------------- |
| `eunomia_policy_deployments_total`                  | Counter   | `service`, `status`                   |
| `eunomia_policy_rollbacks_total`                    | Counter   | `service`, `reason`                   |
| `eunomia_bundle_compilation_duration_seconds`       | Histogram | `service`                             |
| `eunomia_policy_tests_total`                        | Counter   | `service`, `status`                   |
| `eunomia_policy_coverage_percentage`                | Gauge     | `service`                             |
| `eunomia_authorization_decisions_total`             | Counter   | `service`, `operation_id`, `decision` |
| `eunomia_authorization_evaluation_duration_seconds` | Histogram | `service`                             |

### 13.3 Structured Logging

```json
{
  "timestamp": "2026-01-04T12:00:00.000Z",
  "level": "INFO",
  "message": "policy deployed successfully",
  "service": "users-service",
  "version": "1.2.0",
  "instances_updated": 5,
  "duration_ms": 1234,
  "strategy": "canary",
  "trace_id": "abc123..."
}
```

---

## 14. Security Model

> **Security Audit**: Completed Week 21 (2026-01-08)
> 
> The bundle signing implementation has been audited for security.
> See [signing.rs](../crates/eunomia-core/src/signing.rs) for implementation.

### 14.1 Bundle Signing Overview

Eunomia uses **Ed25519** digital signatures to ensure bundle authenticity and integrity:

| Property | Guarantee |
|----------|-----------|
| **Authenticity** | Bundles can only be signed by holders of the private key |
| **Integrity** | Any modification to bundle content invalidates the signature |
| **Non-repudiation** | Signatures are tied to specific key IDs for audit trails |
| **Key Rotation** | Multiple key IDs supported for smooth key transitions |

### 14.2 Cryptographic Design

**Algorithm Selection**:
- **Signature**: Ed25519 (EdDSA over Curve25519)
- **Hash**: SHA-256 for bundle checksums
- **Encoding**: Base64 for serialized signatures

**Why Ed25519**:
- 128-bit security level
- Small keys (32 bytes public, 32 bytes private)
- Fast signing and verification
- Deterministic signatures (no nonce required)
- Well-audited implementation (`ed25519-dalek` crate)

### 14.3 Checksum Computation

The bundle checksum covers **all security-relevant content**:

```rust
// Checksum includes:
// 1. Bundle format version (prevents cross-version attacks)
// 2. Bundle name (service identity)
// 3. Bundle version (prevents version confusion)
// 4. All policy files (sorted by package name)
// 5. All data files (sorted by path)

fn compute_checksum(&self) -> String {
    let mut hasher = Sha256::new();
    
    // Include bundle identity to prevent signature reuse
    hasher.update(b"eunomia-bundle:1\n");
    hasher.update(self.name.as_bytes());
    hasher.update(b"\n");
    hasher.update(self.version.as_bytes());
    hasher.update(b"\n");
    
    // Add sorted policies and data files
    // ...
    
    hex::encode(hasher.finalize())
}
```

**Security Properties**:
- Deterministic: Same content always produces same checksum
- Order-independent: Sorted iteration ensures reproducibility
- Identity-bound: Bundle name and version included in hash

### 14.4 Signature Structure

```json
{
  "signatures": [
    {
      "keyid": "prod-2026-01",
      "algorithm": "ed25519",
      "value": "base64-encoded-signature"
    }
  ]
}
```

**Fields**:
- `keyid`: Identifies which key signed (for key rotation)
- `algorithm`: Always "ed25519" (informational only)
- `value`: 64-byte Ed25519 signature, base64-encoded

### 14.5 Key Management Best Practices

> **Important**: Key management is the operator's responsibility.
> Eunomia provides cryptographic primitives but not key storage.

**Recommended Practices**:

| Practice | Recommendation |
|----------|---------------|
| **Key Generation** | Use `SigningKeyPair::generate()` with `OsRng` |
| **Key Storage** | HSM, Vault, or K8s Secrets with encryption at rest |
| **Key Distribution** | Out-of-band verification of public keys |
| **Key Rotation** | Plan regular rotation, use key IDs for transition |
| **Key Backup** | Securely backup private keys for disaster recovery |

**Environment Variables for CI/CD**:
```bash
# Private key for signing (base64-encoded)
export EUNOMIA_SIGNING_KEY="base64-private-key"
export EUNOMIA_SIGNING_KEY_ID="prod-2026-01"

# Public keys for verification (comma-separated)
export EUNOMIA_VERIFY_KEYS="prod-2026-01:base64-public-key,prod-2025-12:base64-public-key"
```

### 14.6 Threat Model & Mitigations

| Threat | Mitigation |
|--------|-----------|
| **Signature forgery** | Ed25519's 128-bit security makes forgery computationally infeasible |
| **Bundle tampering** | SHA-256 checksum detects any content modification |
| **Signature reuse** | Bundle name/version in checksum prevents cross-bundle attacks |
| **Key compromise** | Key rotation via key IDs; revoke compromised keys |
| **Man-in-the-middle** | mTLS for all control plane communication |
| **Replay attacks** | Version numbers prevent replay of old bundles |

### 14.7 Security Audit Test Coverage

The following security scenarios are tested in `signing.rs`:

```
✅ test_security_key_generation_uniqueness
✅ test_security_deterministic_key_from_seed
✅ test_security_invalid_seed_lengths
✅ test_security_malformed_base64_key
✅ test_security_malformed_signature_rejected
✅ test_security_truncated_signature_rejected
✅ test_security_empty_signature_value_rejected
✅ test_security_signature_from_different_bundle_rejected
✅ test_security_version_change_invalidates_signature
✅ test_security_policy_content_change_invalidates_signature
✅ test_security_key_id_mismatch_handles_gracefully
✅ test_security_verify_all_returns_only_valid_keys
✅ test_security_algorithm_field_ignored
✅ test_security_public_key_invalid_format
✅ test_security_checksum_order_independence
```

### 14.8 mTLS for Control Plane

All communication between Eunomia control plane and Archimedes uses mutual TLS:

```rust
pub struct TlsConfig {
    /// Server certificate path
    pub cert_path: PathBuf,
    /// Server private key path
    pub key_path: PathBuf,
    /// CA certificate for client verification (mTLS)
    pub ca_path: Option<PathBuf>,
    /// Require client certificates
    pub require_client_cert: bool,
}
```

**mTLS Verification**:
1. Server presents certificate signed by trusted CA
2. Client presents certificate signed by trusted CA
3. Server verifies client's SPIFFE ID against allowlist
4. Encrypted channel established

### 14.9 SPIFFE Identity Allowlist

```yaml
# eunomia.yaml
control_plane:
  listen_addr: "0.0.0.0:9443"
  tls:
    cert_path: "/etc/certs/server.crt"
    key_path: "/etc/certs/server.key"
    ca_path: "/etc/certs/ca.crt"

  # Only these SPIFFE IDs can push policies
  allowed_pushers:
    - "spiffe://somniatore.com/eunomia/control-plane"
    - "spiffe://somniatore.com/ci/policy-deployer"
```

---

## 15. CLI Design

### 15.1 Command Structure

```bash
eunomia <command> [options]

Commands:
  test        Run policy tests
  compile     Compile policies into bundles
  sign        Sign a policy bundle
  publish     Publish bundle to registry
  push        Push bundle to Archimedes instances
  rollback    Rollback to previous policy version
  status      Get deployment status
  audit       View audit logs

Options:
  --config    Path to eunomia config file
  --verbose   Enable verbose output
  --json      Output in JSON format
```

### 15.2 Command Examples

```bash
# Run tests
eunomia test ./policies
eunomia test ./policies --coverage --coverage-threshold 80

# Compile bundle
eunomia compile \
  --policy-path ./policies/services/users-service \
  --output ./bundles/users-service.bundle.tar.gz \
  --version 1.2.0

# Sign bundle
eunomia sign \
  --bundle ./bundles/users-service.bundle.tar.gz \
  --key-file ./keys/policy-signing.key

# Publish to registry
eunomia publish \
  --bundle ./bundles/users-service.bundle.tar.gz \
  --registry https://registry.somniatore.com

# Push to services
eunomia push \
  --service users-service \
  --version 1.2.0 \
  --strategy canary \
  --canary-percentage 10 \
  --canary-duration 5m

# Check status
eunomia status --service users-service

# Rollback
eunomia rollback \
  --service users-service \
  --to-version 1.1.0 \
  --reason "increased authorization failures"

# View audit log
eunomia audit --service users-service --since 24h
```

---

## 16. Integration Points

### 16.1 Archimedes Integration

```
┌─────────────────┐         ┌─────────────────┐
│    Eunomia      │         │   Archimedes    │
│  Control Plane  │         │    Instance     │
│                 │         │                 │
│  ┌───────────┐  │ mTLS    │  ┌───────────┐  │
│  │  Pusher   │──┼─────────┼─►│  Control  │  │
│  │           │  │         │  │  Endpoint │  │
│  └───────────┘  │         │  └─────┬─────┘  │
│                 │         │        │        │
│                 │         │        ▼        │
│                 │         │  ┌───────────┐  │
│                 │         │  │    OPA    │  │
│                 │         │  │ Evaluator │  │
│                 │         │  └───────────┘  │
└─────────────────┘         └─────────────────┘
```

**Control Endpoint Contract:**

```yaml
# Archimedes control endpoint
POST /control/policy-update
Authorization: mTLS (SPIFFE)

Request:
  bundle_url: string
  manifest: PolicyManifest

Response:
  status: "accepted" | "rejected"
  current_version: string
  previous_version: string
  error?: string
```

### 16.2 Themis Integration

Eunomia uses Themis contract metadata for policy context:

```rego
# Access contract metadata in policies
allow if {
    # Get operation metadata from Themis contract
    operation := data.themis.operations[input.operation_id]

    # Only allow if operation is marked as idempotent
    operation.metadata.idempotent == true

    # And caller has appropriate role
    has_required_role(input.caller, operation.metadata.required_role)
}
```

### 16.3 Stoa Integration

Stoa displays:

- Policy status per service
- Deployment history
- Authorization decision metrics
- Policy diffs between versions
- Audit log viewer

---

## 17. Open Questions

### Resolved

- ✅ **Policy language**: OPA/Rego - ADR-002
- ✅ **Rego parsing**: Use `regorus` crate (pure Rust) - ADR-004
- ✅ **Distribution model**: Hybrid push/pull (push primary, pull fallback) - ADR-003
- ✅ **State storage**: PostgreSQL for deployment state
- ✅ **Policy versioning**: Semantic versioning (SemVer 2.0.0)
- ✅ **Canary duration**: Default 5 minutes, configurable per deployment
- ✅ **Automatic rollback triggers**: Error rate > 5%, latency p99 > 500ms, 3+ consecutive health check failures

### Under Discussion

- 🟡 **Multi-cluster support**: Design in gap weeks (13-16), implement post-MVP

### Resolved (Gap Week Research)

- ✅ **Cache encryption key management**: Use Kubernetes Secrets with external-secrets-operator for rotation. Bundle encryption uses per-environment keys stored in Vault/K8s Secrets. Key rotation triggers bundle re-encryption and re-push (automated via control plane).
- ✅ **Cross-region replication**: Use OCI registry geo-replication for bundle distribution. Each region pulls from nearest registry replica. Control plane is single-region with read replicas. Multi-region active-active is post-MVP.
- ✅ **Policy inheritance**: Research complete. Use import-based inheritance for MVP, bundle composition for post-MVP. See [Policy Inheritance Design](designs/policy-inheritance.md).
- ✅ **External data integration**: Research complete. Push-based data sync from IdP (Okta, LDAP) via control plane. See [External Data Integration Design](designs/external-data-integration.md).

> 📝 **Note**: Gap weeks (13-16) used for prototyping yellow items.

---

## 18. Implementation Phases

### Phase 1: Core Framework (Weeks 1-4)

- [ ] Set up repository structure
- [ ] Implement `eunomia-core` (types, models)
- [ ] Implement `eunomia-compiler` (Rego parsing, bundling)
- [ ] Basic CLI scaffold

**Deliverable**: Compile Rego policies into bundles

### Phase 2: Testing Framework (Weeks 5-8)

- [ ] Implement `eunomia-test` (test runner)
- [ ] Coverage analysis
- [ ] Static analysis rules
- [ ] CLI command: `test`

**Deliverable**: Run and validate policy tests

### Phase 3: Registry & Publishing (Weeks 9-12)

- [ ] Implement `eunomia-registry` (registry client)
- [ ] Bundle signing
- [ ] Set up registry infrastructure
- [ ] CLI commands: `sign`, `publish`

**Deliverable**: Publish signed bundles to registry

### Phase 4: Distribution (Weeks 13-16)

- [ ] Implement `eunomia-distributor` (push mechanism)
- [ ] Canary deployment support
- [ ] Health checking
- [ ] CLI commands: `push`, `status`

**Deliverable**: Push policies to Archimedes instances

### Phase 5: Control Plane (Weeks 17-20)

- [ ] Implement `eunomia-control-plane` (service)
- [ ] State management
- [ ] Rollback controller
- [ ] gRPC API

**Deliverable**: Centralized policy management service

### Phase 6: Observability & Audit (Weeks 21-24)

- [ ] Implement `eunomia-audit` (audit logging)
- [ ] Metrics integration
- [ ] Alert rules
- [ ] CI integration (GitHub Action)

**Deliverable**: Production-ready with full observability

---

## Appendix A: Example Policy Repository

```
policies/
├── .github/
│   └── workflows/
│       └── policy-ci.yml
├── common/
│   ├── roles.rego
│   ├── roles_test.rego
│   ├── identity.rego
│   └── identity_test.rego
├── services/
│   ├── users-service/
│   │   ├── authz.rego
│   │   └── authz_test.rego
│   └── orders-service/
│       ├── authz.rego
│       └── authz_test.rego
├── bundles/
│   ├── users-service.yaml
│   └── orders-service.yaml
├── eunomia.yaml
├── eunomia-analysis.yaml
└── README.md
```

---

_End of Eunomia Design Document_
