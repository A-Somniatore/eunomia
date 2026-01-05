# Policy Migration Guide

> **Version**: 1.0.0  
> **Last Updated**: 2026-01-05  
> **Audience**: Teams migrating existing authorization to Eunomia

This guide helps teams migrate from existing authorization systems to Eunomia's OPA/Rego-based policies.

---

## Table of Contents

1. [Migration Overview](#1-migration-overview)
2. [Assessment Phase](#2-assessment-phase)
3. [Design Phase](#3-design-phase)
4. [Implementation Phase](#4-implementation-phase)
5. [Testing Phase](#5-testing-phase)
6. [Deployment Phase](#6-deployment-phase)
7. [Common Migration Scenarios](#7-common-migration-scenarios)
8. [Troubleshooting](#8-troubleshooting)

---

## 1. Migration Overview

### Why Migrate to Eunomia?

| Feature | Traditional | Eunomia |
|---------|-------------|---------|
| Policy Location | Hardcoded in application | External, Git-managed |
| Policy Updates | Redeploy application | Hot-reload without downtime |
| Auditability | Manual review | Git history + signed bundles |
| Testing | Unit tests in app code | Dedicated policy test framework |
| Consistency | Per-service implementation | Centralized platform |

### Migration Stages

```
┌─────────────────────────────────────────────────────────────────┐
│  1. ASSESS    │  2. DESIGN    │  3. IMPLEMENT │  4. TEST      │
│               │               │               │               │
│  - Inventory  │  - Input      │  - Write      │  - Unit tests │
│  - Document   │    schema     │    policies   │  - Integration│
│  - Prioritize │  - Package    │  - Build      │  - Shadow mode│
│               │    structure  │    bundles    │               │
└─────────────────────────────────────────────────────────────────┘
                                                        │
                                    ┌───────────────────▼─────────┐
                                    │  5. DEPLOY                  │
                                    │                             │
                                    │  - Gradual rollout          │
                                    │  - Monitor & adjust         │
                                    │  - Decommission old system  │
                                    └─────────────────────────────┘
```

---

## 2. Assessment Phase

### 2.1 Inventory Current Authorization

Document all authorization decisions in your codebase:

```
Authorization Inventory Template:

Service: _______________
Total Authorization Points: ___

1. Authorization Point: _______________
   - Location (file:line): _______________
   - Caller Types Supported: user / service / api_key / anonymous
   - Decision Factors:
     [ ] Roles
     [ ] Permissions
     [ ] Resource ownership
     [ ] Tenant membership
     [ ] Time-based
     [ ] Other: _______________
   - Current Implementation:
     [ ] Hardcoded
     [ ] Database lookup
     [ ] External service
     [ ] Configuration file
   - Business Rules: _______________
```

### 2.2 Document Business Rules

For each authorization point, document the actual business rules:

```markdown
## Operation: deleteUser

### Who Can Delete Users?
- Admins can delete any user
- Users can delete their own account (self-service)
- Support staff can delete users in their assigned region

### Restrictions
- Cannot delete users with active subscriptions
- Cannot delete the last admin
- Soft-delete only (mark as inactive)

### Exceptions
- Platform super-admins can force-delete
- Scheduled cleanup jobs can delete inactive accounts > 2 years
```

### 2.3 Prioritize Migration Order

Recommended priority:

1. **Simple policies first** - Read-only operations, basic role checks
2. **Critical paths second** - Core business operations
3. **Complex rules last** - Multi-factor decisions, external data

```
Priority Matrix:

High Value, Low Complexity  ←── Start here
        │
        ▼
High Value, High Complexity ←── Then here
        │
        ▼
Low Value, Low Complexity   ←── If time permits
        │
        ▼
Low Value, High Complexity  ←── Consider keeping in app
```

---

## 3. Design Phase

### 3.1 Define Input Schema

Map your application context to the Eunomia input schema:

```json
{
  "caller": {
    "type": "user|spiffe|api_key|anonymous",
    
    // User fields (map from your auth token/session)
    "user_id": "from: jwt.sub",
    "email": "from: jwt.email",
    "roles": "from: jwt.roles or database lookup",
    "tenant_id": "from: jwt.tenant_id",
    
    // Service fields (map from SPIFFE/mTLS)
    "service_name": "from: SPIFFE ID",
    "trust_domain": "from: SPIFFE ID",
    
    // API key fields (map from key validation)
    "key_id": "from: database lookup",
    "scopes": "from: database lookup"
  },
  
  "service": "your-service-name",
  "operation_id": "from: OpenAPI operationId or method name",
  "method": "from: HTTP method",
  "path": "from: HTTP path",
  
  "context": {
    // Application-specific data
    "resource_id": "from: path params",
    "resource_owner": "from: database lookup",
    "resource_status": "from: database lookup"
  }
}
```

### 3.2 Design Package Structure

```
policies/
├── common/                    # Shared helpers
│   └── authz.rego
├── users-service/
│   ├── authz.rego            # Main policy
│   ├── authz_test.rego       # Tests
│   └── data.json             # Static data (roles, permissions)
├── orders-service/
│   └── ...
└── billing-service/
    └── ...
```

### 3.3 Plan Phased Rollout

```yaml
Phase 1 - Shadow Mode (Week 1-2):
  - Deploy Eunomia alongside existing auth
  - Log both decisions
  - Compare results
  - Fix discrepancies
  
Phase 2 - Canary (Week 3):
  - Route 5% of traffic through Eunomia
  - Monitor for errors/latency
  - Gradually increase to 25%
  
Phase 3 - Majority (Week 4):
  - Route 75% through Eunomia
  - Keep fallback to old system
  - Monitor carefully

Phase 4 - Complete (Week 5+):
  - Route 100% through Eunomia
  - Remove old authorization code
  - Archive for reference
```

---

## 4. Implementation Phase

### 4.1 Translating Authorization Logic

#### If-Else Chains to Rego

**Before (Pseudocode):**
```python
def can_delete_user(caller, target_user):
    if caller.role == "admin":
        return True
    if caller.id == target_user.id and caller.role == "user":
        return True  # Self-delete
    if caller.role == "support" and target_user.region == caller.region:
        return True
    return False
```

**After (Rego):**
```rego
package users_service.authz

import future.keywords.if
import future.keywords.in

default allow := false

# Admin can delete any user
allow if {
    input.caller.type == "user"
    "admin" in input.caller.roles
    input.operation_id == "deleteUser"
}

# User can delete themselves
allow if {
    input.caller.type == "user"
    "user" in input.caller.roles
    input.operation_id == "deleteUser"
    input.caller.user_id == input.context.target_user_id
}

# Support can delete users in their region
allow if {
    input.caller.type == "user"
    "support" in input.caller.roles
    input.operation_id == "deleteUser"
    input.caller.region == input.context.target_user_region
}
```

#### Permission Lookups to Data

**Before (Database):**
```sql
SELECT permission FROM role_permissions 
WHERE role = :user_role AND permission = :required_permission;
```

**After (data.json):**
```json
{
  "role_permissions": {
    "admin": ["users:read", "users:write", "users:delete"],
    "editor": ["users:read", "users:write"],
    "viewer": ["users:read"]
  }
}
```

**Rego:**
```rego
import data.role_permissions

allow if {
    input.caller.type == "user"
    some role in input.caller.roles
    required_permission in role_permissions[role]
}

required_permission := "users:delete" if input.operation_id == "deleteUser"
required_permission := "users:write" if input.operation_id in {"createUser", "updateUser"}
required_permission := "users:read" if input.operation_id in {"getUser", "listUsers"}
```

### 4.2 Handling External Data

If your authorization requires database lookups:

**Option 1: Include in Input Context**
```rust
// Application code builds rich context
let context = json!({
    "resource_owner": db.get_resource_owner(resource_id),
    "resource_status": db.get_resource_status(resource_id),
    "user_subscription": db.get_user_subscription(user_id)
});

let input = json!({
    "caller": caller_identity,
    "operation_id": operation,
    "context": context
});
```

**Option 2: Bundle Static Data**
```json
// data.json - bundled with policy
{
  "feature_flags": {
    "self_delete_enabled": true,
    "support_regional_access": true
  },
  "rate_limits": {
    "free": 100,
    "pro": 10000
  }
}
```

**Option 3: OPA Bundle Data API (Advanced)**
```rego
// Fetch from external data source (OPA feature)
resource := http.send({
    "method": "GET",
    "url": sprintf("http://data-service/resources/%s", [input.resource_id])
}).body
```

---

## 5. Testing Phase

### 5.1 Create Baseline Tests

Before migrating, capture current behavior:

```python
# Generate test cases from production logs
test_cases = []
for log in production_auth_logs[-1000:]:
    test_cases.append({
        "name": f"{log.operation}_{log.result}_{log.id}",
        "input": {
            "caller": log.caller,
            "operation_id": log.operation,
            "context": log.context
        },
        "expected_allowed": log.result == "allowed"
    })

# Save as fixtures
with open("migration_baseline_fixtures.json", "w") as f:
    json.dump({"fixtures": test_cases}, f)
```

### 5.2 Shadow Testing

Run both authorization systems in parallel:

```rust
async fn authorize(&self, request: &AuthRequest) -> AuthResult {
    // Run old system (source of truth during migration)
    let old_result = self.old_auth.check(request).await?;
    
    // Run Eunomia in parallel
    let new_result = self.eunomia.evaluate(request).await;
    
    // Compare and log discrepancies
    if old_result.allowed != new_result.allowed {
        log::warn!(
            "Authorization mismatch! Old: {}, New: {}, Request: {:?}",
            old_result.allowed,
            new_result.allowed,
            request
        );
        self.metrics.auth_mismatch.inc();
    }
    
    // Return old result during shadow phase
    old_result
}
```

### 5.3 Validate with Fixtures

```bash
# Run baseline tests against new policy
eunomia test policies/ --fixtures migration_baseline_fixtures.json

# Expected: 100% pass rate
# If failures, investigate and fix policy or update baseline
```

---

## 6. Deployment Phase

### 6.1 Feature Flag Rollout

```rust
// Use feature flags for gradual rollout
async fn authorize(&self, request: &AuthRequest) -> AuthResult {
    let use_eunomia = self.feature_flags
        .get_percentage("eunomia_enabled", request.user_id);
    
    if use_eunomia {
        self.eunomia.evaluate(request).await
    } else {
        self.old_auth.check(request).await
    }
}
```

### 6.2 Monitoring

Set up monitoring for:

- **Latency**: Policy evaluation time
- **Error rate**: Evaluation failures
- **Decision rate**: Allow vs deny ratio
- **Mismatch rate**: During shadow mode

```rust
// Prometheus metrics example
let auth_latency = Histogram::new("auth_latency_seconds");
let auth_decisions = Counter::new("auth_decisions_total", &["result"]);
let auth_errors = Counter::new("auth_errors_total");
```

### 6.3 Rollback Plan

```yaml
Rollback Triggers:
  - Error rate > 1%
  - P99 latency > 100ms
  - Mismatch rate > 0.1% (during shadow)
  - Any security incident

Rollback Steps:
  1. Set feature flag to 0%
  2. Verify traffic using old system
  3. Investigate root cause
  4. Fix and re-test
  5. Resume gradual rollout
```

---

## 7. Common Migration Scenarios

### 7.1 Spring Security to Eunomia

**Before (Spring Security):**
```java
@PreAuthorize("hasRole('ADMIN') or (hasRole('USER') and #userId == authentication.principal.id)")
public User getUser(@PathVariable String userId) { ... }
```

**After (Rego):**
```rego
allow if {
    input.caller.type == "user"
    "admin" in input.caller.roles
    input.operation_id == "getUser"
}

allow if {
    input.caller.type == "user"
    "user" in input.caller.roles
    input.operation_id == "getUser"
    input.caller.user_id == input.context.user_id
}
```

### 7.2 Express.js Middleware to Eunomia

**Before (Express):**
```javascript
const authorize = (requiredRoles) => (req, res, next) => {
  if (!req.user) return res.status(401).send('Unauthorized');
  const hasRole = requiredRoles.some(role => req.user.roles.includes(role));
  if (!hasRole) return res.status(403).send('Forbidden');
  next();
};

app.delete('/users/:id', authorize(['admin']), deleteUser);
```

**After (Rego):**
```rego
allow if {
    input.caller.type == "user"
    "admin" in input.caller.roles
    input.operation_id == "deleteUser"
}
```

**Integration:**
```javascript
const { EunomiaClient } = require('@eunomia/client');

const authorize = async (req, res, next) => {
  const decision = await eunomia.evaluate({
    caller: {
      type: 'user',
      user_id: req.user?.id,
      roles: req.user?.roles || []
    },
    operation_id: req.route.operationId,
    method: req.method,
    path: req.path,
    context: req.params
  });
  
  if (!decision.allowed) {
    return res.status(403).json({ error: decision.reason });
  }
  next();
};
```

### 7.3 AWS IAM-Style to Eunomia

**Before (IAM-style JSON):**
```json
{
  "Version": "2012-10-17",
  "Statement": [{
    "Effect": "Allow",
    "Action": ["users:Get*", "users:List*"],
    "Resource": "arn:service:users:*:*:user/*",
    "Condition": {
      "StringEquals": {"users:department": "${aws:PrincipalTag/department}"}
    }
  }]
}
```

**After (Rego):**
```rego
allow if {
    input.operation_id in {"getUser", "listUsers"}
    input.caller.department == input.context.resource_department
}
```

---

## 8. Troubleshooting

### Common Issues

**"Authorization is more permissive than before"**

- Check for missing deny conditions
- Verify all edge cases are covered
- Add explicit tests for deny scenarios

**"Authorization is more restrictive than before"**

- Check if all allow conditions are translated
- Verify input schema mapping is complete
- Check for typos in role/permission names

**"Latency increased significantly"**

- Pre-compute expensive operations in application
- Use bundle data instead of HTTP calls
- Cache policy evaluations where safe

**"Decision reasons are unclear"**

- Add `reason` rule to policy
- Include context in deny messages
- Use structured logging

### Debug Checklist

- [ ] Input schema matches expected format
- [ ] All Rego files load without errors (`eunomia validate`)
- [ ] Tests pass (`eunomia test`)
- [ ] Bundle builds successfully (`eunomia build`)
- [ ] Shadow mode shows 0% mismatch
- [ ] Latency is within acceptable range

---

## Appendix: Migration Checklist

```
PRE-MIGRATION
[ ] Complete authorization inventory
[ ] Document all business rules
[ ] Set migration priority
[ ] Define success criteria

DESIGN
[ ] Define input schema
[ ] Design package structure
[ ] Plan phased rollout
[ ] Set up monitoring

IMPLEMENTATION
[ ] Write Rego policies
[ ] Create comprehensive tests
[ ] Build and sign bundles
[ ] Set up distribution

TESTING
[ ] Generate baseline fixtures
[ ] Run shadow testing
[ ] Achieve 0% mismatch rate
[ ] Performance benchmarks pass

DEPLOYMENT
[ ] Deploy to staging
[ ] Gradual production rollout
[ ] Monitor for issues
[ ] Complete rollout

POST-MIGRATION
[ ] Remove old authorization code
[ ] Update documentation
[ ] Train team on Eunomia
[ ] Archive migration artifacts
```

---

## See Also

- [Policy Authoring Guide](policy-authoring-guide.md)
- [Testing Guide](testing-guide.md)
- [Example Policies](../examples/policies/README.md)
- [Design Document](design.md)
