# Eunomia – Implementation Design Document

> **Version**: 1.1.0-draft  
> **Status**: Design Phase  
> **Repository**: `github.com/A-Somniatore/eunomia` (to be created)  
> **Last Updated**: 2026-01-04

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
│   │       └── bundler.rs        # Bundle creation
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
│   │       ├── publish.rs        # Publishing
│   │       └── fetch.rs          # Fetching
│   │
│   ├── eunomia-distributor/      # Policy distribution
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── scheduler.rs      # Push scheduling
│   │       ├── pusher.rs         # Bundle pushing
│   │       └── health.rs         # Health checking
│   │
│   └── eunomia-audit/            # Audit logging
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── logger.rs         # Audit log emission
│           └── schema.rs         # Audit event schema
│
├── eunomia-cli/                  # CLI application
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       └── commands/
│           ├── test.rs
│           ├── compile.rs
│           ├── publish.rs
│           ├── push.rs
│           └── rollback.rs
│
├── eunomia-control-plane/        # Control plane service
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       ├── api.rs                # Control plane API
│       ├── state.rs              # Deployment state
│       └── reconciler.rs         # Desired state reconciliation
│
├── eunomia-action/               # GitHub Action
│   ├── action.yml
│   ├── Dockerfile
│   └── entrypoint.sh
│
├── schemas/                      # JSON/Protobuf schemas
│   ├── bundle.schema.json
│   ├── audit-event.schema.json
│   └── control-plane.proto
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

| Change Type | Version Bump | Example |
|-------------|--------------|---------|
| Remove permission (more restrictive) | MAJOR | User can no longer access endpoint |
| Change authorization logic semantics | MAJOR | Different decision for same input |
| Add new permission (more permissive) | MINOR | New role can access endpoint |
| Add new policy for new operation | MINOR | Policy for new `operationId` |
| Fix bug without changing semantics | PATCH | Typo in role name |
| Performance optimization | PATCH | Rule reordering |
| Documentation updates | PATCH | Comments, metadata |

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

### 8.1 Bundle Structure

```
users-service-v1.2.0.bundle.tar.gz
├── manifest.json
├── policy/
│   ├── users_service/
│   │   └── authz.rego
│   └── common/
│       ├── roles.rego
│       └── identity.rego
├── data/
│   └── static_data.json        # Optional static data
└── signatures/
    └── manifest.sig            # Bundle signature
```

### 8.2 Manifest Format

```json
{
  "version": "1.2.0",
  "service": "users-service",
  "created_at": "2026-01-04T12:00:00Z",
  "git_commit": "abc123def456",
  "git_repository": "github.com/somniatore/policies",

  "entrypoint": "users_service.authz.allow",

  "dependencies": ["common.roles", "common.identity"],

  "checksum": {
    "algorithm": "sha256",
    "value": "e3b0c44298fc..."
  },

  "signature": {
    "algorithm": "ed25519",
    "key_id": "policy-signing-key-2026",
    "value": "base64-encoded-signature"
  },

  "metadata": {
    "author": "platform-team",
    "change_summary": "Added support for API key authentication"
  }
}
```

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

```rust
pub struct RollbackController {
    state: DeploymentState,
    distributor: PolicyDistributor,
}

impl RollbackController {
    pub async fn rollback(&self, service: &str, target_version: &str) -> Result<RollbackReport, RollbackError> {
        // 1. Fetch previous bundle from registry
        let bundle = self.registry.fetch_bundle(service, target_version).await?;

        // 2. Verify bundle integrity
        bundle.verify()?;

        // 3. Update deployment state
        self.state.mark_rollback_in_progress(service).await?;

        // 4. Push old version to all instances (immediately, no canary)
        let report = self.distributor.push_immediate(&bundle).await?;

        // 5. Update state
        self.state.mark_rollback_complete(service, &report).await?;

        // 6. Emit audit event
        self.audit.log(AuditEvent::PolicyRollback {
            service: service.to_string(),
            from_version: self.state.current_version(service).await?,
            to_version: target_version.to_string(),
            reason: "manual rollback",
        }).await?;

        Ok(report)
    }

    pub async fn auto_rollback_on_failure(&self, service: &str, health_check: &HealthCheck) -> Result<(), RollbackError> {
        let previous = self.state.previous_version(service).await?;

        if let Some(prev_version) = previous {
            tracing::warn!(
                service = service,
                current = %self.state.current_version(service).await?,
                rolling_back_to = %prev_version,
                "auto-rollback triggered due to health check failures"
            );

            self.rollback(service, &prev_version).await?;
        }

        Ok(())
    }
}
```

---

## 10. Control Plane

### 10.1 Control Plane API

```protobuf
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

  // Audit
  rpc GetAuditLog(GetAuditLogRequest) returns (stream AuditEvent);
}

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
```

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
```

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

### 14.1 Bundle Signing

```rust
pub struct BundleSigner {
    private_key: Ed25519PrivateKey,
    key_id: String,
}

impl BundleSigner {
    pub fn sign(&self, bundle: &Bundle) -> SignedBundle {
        // Create canonical representation
        let canonical = bundle.canonical_bytes();

        // Sign with Ed25519
        let signature = self.private_key.sign(&canonical);

        SignedBundle {
            bundle: bundle.clone(),
            signature: Signature {
                algorithm: "ed25519".to_string(),
                key_id: self.key_id.clone(),
                value: base64::encode(&signature),
            },
        }
    }
}

pub struct BundleVerifier {
    public_keys: HashMap<String, Ed25519PublicKey>,
}

impl BundleVerifier {
    pub fn verify(&self, signed: &SignedBundle) -> Result<(), VerifyError> {
        let public_key = self.public_keys
            .get(&signed.signature.key_id)
            .ok_or(VerifyError::UnknownKey)?;

        let canonical = signed.bundle.canonical_bytes();
        let signature = base64::decode(&signed.signature.value)?;

        public_key.verify(&canonical, &signature)
            .map_err(|_| VerifyError::InvalidSignature)
    }
}
```

### 14.2 mTLS for Control Plane

```rust
pub struct ControlPlaneClient {
    client: reqwest::Client,
    allowed_spiffe_ids: HashSet<SpiffeId>,
}

impl ControlPlaneClient {
    pub fn new(config: &TlsConfig) -> Result<Self, TlsError> {
        // Load client certificate and key
        let identity = Identity::from_pem(
            &fs::read(&config.cert_path)?,
            &fs::read(&config.key_path)?,
        )?;

        // Load CA certificate
        let ca = Certificate::from_pem(&fs::read(&config.ca_path)?)?;

        let client = reqwest::Client::builder()
            .identity(identity)
            .add_root_certificate(ca)
            .https_only(true)
            .build()?;

        Ok(Self {
            client,
            allowed_spiffe_ids: config.allowed_spiffe_ids.clone(),
        })
    }

    pub fn verify_peer_identity(&self, peer_cert: &Certificate) -> Result<(), AuthError> {
        let spiffe_id = extract_spiffe_id(peer_cert)?;

        if !self.allowed_spiffe_ids.contains(&spiffe_id) {
            return Err(AuthError::UnauthorizedCaller(spiffe_id));
        }

        Ok(())
    }
}
```

### 14.3 SPIFFE Identity Allowlist

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
- 🟡 **Policy inheritance**: Prototype in gap weeks, defer full implementation
- 🟡 **External data**: Research IdP integration in gap weeks

### Resolved (Gap Week Research)

- ✅ **Cache encryption key management**: Use Kubernetes Secrets with external-secrets-operator for rotation. Bundle encryption uses per-environment keys stored in Vault/K8s Secrets. Key rotation triggers bundle re-encryption and re-push (automated via control plane).
- ✅ **Cross-region replication**: Use OCI registry geo-replication for bundle distribution. Each region pulls from nearest registry replica. Control plane is single-region with read replicas. Multi-region active-active is post-MVP.

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
