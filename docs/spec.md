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
