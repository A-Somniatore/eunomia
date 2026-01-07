# Policy Inheritance Design

> **Status**: Research Complete (Week 15-16)  
> **Author**: Platform Team  
> **Created**: 2026-01-05  
> **Target**: Post-MVP (Phase E5+)

---

## 1. Overview

This document outlines patterns for policy inheritance and composition, allowing
organizations to define base policies that service-specific policies can extend.

### Problem Statement

As the number of services grows, common patterns emerge:

- All services need admin access rules
- All services need audit logging rules
- All services need rate limiting rules
- Service-specific rules add to (not replace) common rules

Without inheritance, each service duplicates common rules, leading to:

- Inconsistent enforcement
- Maintenance burden
- Drift between services

---

## 2. Goals

1. **DRY Policies**: Define common rules once, reuse everywhere
2. **Flexibility**: Services can extend OR override base rules
3. **Clarity**: Clear precedence when rules conflict
4. **Testability**: Base and derived policies are independently testable
5. **Auditability**: Clear lineage of which rules apply

---

## 3. Inheritance Patterns in Rego

### 3.1 Pattern 1: Import and Extend (Current Best Practice ✅)

Services import common modules and build on them:

```rego
# common/authz.rego
package common.authz

import future.keywords.if
import future.keywords.in

# Base admin rule - any service can use
is_admin if {
    input.caller.type == "user"
    "admin" in input.caller.roles
}

# Base service-to-service rule
is_internal_service if {
    input.caller.type == "spiffe"
    endswith(input.caller.trust_domain, "somniatore.com")
}

# Base deny conditions
is_blocked_user if {
    input.caller.type == "user"
    input.caller.user_id in data.blocked_users
}
```

```rego
# services/users-service/authz.rego
package users_service.authz

import future.keywords.if
import data.common.authz

default allow := false

# Inherit: Admins can do anything
allow if {
    authz.is_admin
}

# Inherit: Internal services can call
allow if {
    authz.is_internal_service
}

# Extend: Service-specific rules
allow if {
    input.caller.type == "user"
    input.operation_id == "getOwnProfile"
    input.context.user_id == input.caller.user_id
}

# Override: Block takes precedence
deny if {
    authz.is_blocked_user
}

# Final decision
allow if {
    not deny
}
```

**Pros**:

- Works with current Rego/OPA
- Explicit about what's inherited
- Easy to test

**Cons**:

- Requires explicit imports in every service
- No automatic inheritance

### 3.2 Pattern 2: Hierarchical Data (For Dynamic Rules)

Use data hierarchy to define rules that apply at different levels:

```json
{
  "policy_rules": {
    "global": {
      "admin_access": true,
      "blocked_users": ["user-bad-1", "user-bad-2"],
      "rate_limits": {
        "default": 1000,
        "burst": 100
      }
    },
    "services": {
      "users-service": {
        "rate_limits": {
          "default": 2000
        },
        "allowed_operations": ["getUser", "listUsers"]
      }
    }
  }
}
```

```rego
# Policy uses hierarchical data
package authz

import future.keywords.if

# Get effective config (service overrides global)
effective_config := merged if {
    global := data.policy_rules.global
    service := object.get(data.policy_rules.services, input.service, {})
    merged := object.union(global, service)
}

allow if {
    effective_config.admin_access
    input.caller.roles[_] == "admin"
}

rate_limit := effective_config.rate_limits.default
```

**Pros**:

- Dynamic configuration without policy changes
- Clear override semantics

**Cons**:

- Logic in data, harder to review
- Less explicit than code

### 3.3 Pattern 3: Policy Composition (Eunomia Feature)

Eunomia bundles multiple policy modules with defined composition:

```yaml
# bundles/users-service.yaml
apiVersion: eunomia.io/v1
kind: PolicyBundle
metadata:
  name: users-service
  version: 1.2.0
spec:
  # Base policies applied first
  base:
    - common/authz.rego
    - common/audit.rego
    - common/rate-limit.rego

  # Service policies
  policies:
    - services/users-service/authz.rego

  # Composition rules
  composition:
    # How to combine allow decisions
    allow: any # any base OR service allow = allow
    # How to combine deny decisions
    deny: any # any base OR service deny = deny
    # Final: allow AND NOT deny
```

**Implementation in Bundler**:

```rust
// eunomia-compiler/src/bundler.rs (future enhancement)

pub struct BundleComposition {
    pub allow_strategy: CompositionStrategy,
    pub deny_strategy: CompositionStrategy,
}

pub enum CompositionStrategy {
    /// Any rule returning true = true
    Any,
    /// All rules must return true = true
    All,
    /// First matching rule wins
    First,
}

impl Bundler {
    pub fn with_composition(mut self, composition: BundleComposition) -> Self {
        self.composition = Some(composition);
        self
    }

    fn generate_entrypoint(&self) -> String {
        // Generate wrapper policy that combines base + service
        format!(r#"
package entrypoint

import data.common.authz as base
import data.{service}.authz as service

default allow := false

# Combined allow (any)
allow if {{ base.allow }}
allow if {{ service.allow }}

# Combined deny (any)
deny if {{ base.deny }}
deny if {{ service.deny }}

# Final decision
result := {{"allow": allow, "deny": deny}}
        "#, service = self.service_name)
    }
}
```

---

## 4. Recommended Approach

### 4.1 MVP: Import and Extend

For MVP, use Pattern 1 (import and extend):

```
policies/
├── common/
│   ├── authz.rego          # Base rules (is_admin, is_internal, etc.)
│   ├── authz_test.rego
│   ├── audit.rego          # Audit helpers
│   └── identity.rego       # Identity helpers
├── services/
│   ├── users-service/
│   │   ├── authz.rego      # Imports common.authz
│   │   └── authz_test.rego
```

### 4.2 Post-MVP: Bundle Composition

After MVP, add bundle composition in Eunomia:

```yaml
# eunomia.yaml (global config)
defaults:
  base_policies:
    - common/authz.rego
    - common/audit.rego
  composition:
    allow: any
    deny: any
```

---

## 5. Inheritance Rules

### 5.1 Precedence Order

When multiple rules could apply:

1. **Explicit deny** always wins
2. **Service-specific rules** override base for same operation
3. **Base rules** apply if no service override
4. **Default deny** if no rules match

### 5.2 Example Precedence

```rego
# common/authz.rego
package common.authz

# Base: admins can do anything
allow if { is_admin }

# Base: blocked users denied
deny if { is_blocked_user }
```

```rego
# users-service/authz.rego
package users_service.authz

import data.common.authz

# Service: admins can do anything (inherited)
allow if { authz.is_admin }

# Service: users can read own profile (extension)
allow if {
    input.operation_id == "getProfile"
    input.context.user_id == input.caller.user_id
}

# Service: maintenance mode (override)
deny if { data.maintenance_mode }

# Explicit combination
final_allow if {
    allow
    not deny
    not authz.deny  # Check base deny too
}
```

---

## 6. Testing Inherited Policies

### 6.1 Test Base Policies Independently

```rego
# common/authz_test.rego
package common.authz_test

import data.common.authz

test_is_admin_with_admin_role if {
    authz.is_admin with input as {
        "caller": {"type": "user", "roles": ["admin"]}
    }
}

test_is_admin_without_role if {
    not authz.is_admin with input as {
        "caller": {"type": "user", "roles": ["viewer"]}
    }
}
```

### 6.2 Test Service Policies with Mocked Base

```rego
# users-service/authz_test.rego
package users_service.authz_test

import data.users_service.authz

# Test service-specific rule
test_user_can_read_own_profile if {
    authz.allow with input as {
        "caller": {"type": "user", "user_id": "user-123", "roles": []},
        "operation_id": "getProfile",
        "context": {"user_id": "user-123"}
    }
}

# Test inherited rule still works
test_admin_can_access_anything if {
    authz.allow with input as {
        "caller": {"type": "user", "roles": ["admin"]},
        "operation_id": "deleteUser"
    }
}
```

### 6.3 Integration Tests for Full Inheritance

```rust
// eunomia-test (future)
#[test]
fn test_policy_inheritance_chain() {
    let suite = TestSuite::builder()
        .add_policy("common/authz.rego")
        .add_policy("services/users-service/authz.rego")
        .build();

    // Test that base rules apply
    let result = suite.evaluate(json!({
        "caller": {"type": "user", "roles": ["admin"]},
        "operation_id": "anything"
    }));
    assert!(result.allow);

    // Test that service rules extend
    let result = suite.evaluate(json!({
        "caller": {"type": "user", "user_id": "u1", "roles": []},
        "operation_id": "getProfile",
        "context": {"user_id": "u1"}
    }));
    assert!(result.allow);
}
```

---

## 7. CLI Support

### 7.1 Validate Inheritance

```bash
# Validate that service policy correctly imports base
eunomia validate --check-inheritance services/users-service/

# Output:
# ✓ Imports common.authz
# ✓ Imports common.audit
# ⚠ Does not import common.rate_limit (optional)
# ✓ Defines default allow := false
# ✓ No conflicting rule names with base
```

### 7.2 Visualize Policy Inheritance

```bash
# Show inheritance tree
eunomia inspect --inheritance services/users-service/authz.rego

# Output:
# users_service.authz
# ├── common.authz (imported)
# │   ├── is_admin
# │   ├── is_internal_service
# │   └── is_blocked_user
# ├── allow (4 rules)
# │   ├── [inherited] is_admin
# │   ├── [inherited] is_internal_service
# │   ├── [local] user can read own profile
# │   └── [local] user can update own profile
# └── deny (2 rules)
#     ├── [inherited] is_blocked_user
#     └── [local] maintenance_mode
```

---

## 8. Bundle Composition Configuration

### 8.1 Bundle Configuration Schema

```yaml
# bundles/users-service.yaml
apiVersion: eunomia.io/v1
kind: PolicyBundle
metadata:
  name: users-service
  version: 1.2.0
  labels:
    team: platform
    tier: critical

spec:
  # Policies to include (order matters for composition)
  policies:
    # Base policies (applied first)
    - path: common/authz.rego
      role: base
    - path: common/audit.rego
      role: base

    # Service policies (extend base)
    - path: services/users-service/authz.rego
      role: service

  # Data files
  data:
    - path: services/users-service/data.json

  # How to compose decisions
  composition:
    entrypoint: users_service.authz.final_allow

    # Or auto-generate entrypoint with strategy
    # strategy:
    #   allow: any  # OR across all policies
    #   deny: any   # OR across all policies
```

---

## 9. Implementation Phases

### Phase 1: MVP (Current)

- [x] Import-based inheritance (manual)
- [x] Common policy modules in examples
- [x] Documentation of patterns

### Phase 2: Post-MVP (E5)

- [ ] `--check-inheritance` CLI flag
- [ ] Inheritance visualization
- [ ] Bundle composition config

### Phase 3: Advanced (E6+)

- [ ] Auto-generated entrypoints
- [ ] Inheritance conflict detection
- [ ] Policy lineage in audit logs

---

## 10. Alternatives Considered

### 10.1 OPA Plugins for Inheritance

Custom OPA plugin that implements inheritance.

**Rejected**: Adds complexity, harder to debug, non-standard.

### 10.2 Pre-Processing/Macro System

Template system that expands inheritance at compile time.

**Rejected**: Obscures final policy, harder to debug.

### 10.3 Multiple Bundles with Layering

Deploy multiple bundles that layer on each other.

**Deferred**: Could work but adds deployment complexity.

---

## 11. References

- [OPA Policy Composition](https://www.openpolicyagent.org/docs/latest/policy-language/#modules)
- [Rego Import Statement](https://www.openpolicyagent.org/docs/latest/policy-reference/#import)
- [Styra DAS Policy Libraries](https://docs.styra.com/das/policies/libraries)
