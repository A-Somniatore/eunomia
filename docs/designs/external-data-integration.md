# External Data Integration Design

> **Status**: Research Complete (Week 15-16)  
> **Author**: Platform Team  
> **Created**: 2026-01-05  
> **Target**: Post-MVP (Phase E5+)

---

## 1. Overview

This document outlines how Eunomia policies can integrate with external data sources
such as Identity Providers (IdP), user attribute stores, and dynamic configuration systems.

### Problem Statement

Currently, policies can only use:

1. Static data bundled with the policy (`data.json`)
2. Request-time input provided by Archimedes

For production use cases, policies often need:

- Dynamic user attributes from IdP (roles, groups, permissions)
- Resource ownership data from databases
- Feature flags and dynamic configuration
- Time-based access control data (schedules, blackout periods)

---

## 2. OPA External Data Patterns

OPA supports several patterns for external data:

### 2.1 Bundle-Time Data (Current Support ✅)

Data included in the policy bundle at compile time:

```rego
# data.json bundled with policy
{
  "admin_users": ["user-1", "user-2"],
  "service_allowlist": ["orders-service", "users-service"]
}

# Policy references bundled data
allow if {
    input.caller.user_id in data.admin_users
}
```

**Pros**: Fast, no runtime dependencies  
**Cons**: Static, requires bundle republish for updates

### 2.2 Push-Based Data Updates (Recommended for MVP+)

Control plane pushes data updates to Archimedes instances:

```
┌─────────────┐     push data      ┌─────────────────┐
│   External  │ ──────────────────▶│   Archimedes    │
│   Data API  │                    │   OPA Instance  │
└─────────────┘                    └─────────────────┘
       ▲
       │ poll/webhook
       │
┌─────────────┐
│   Eunomia   │
│   Control   │
│   Plane     │
└─────────────┘
```

**Flow**:

1. Eunomia control plane polls external data sources (or receives webhooks)
2. Control plane pushes data updates to Archimedes instances
3. Archimedes merges data into OPA's data store

### 2.3 OPA HTTP.send (Not Recommended)

OPA can make HTTP calls during evaluation:

```rego
# NOT RECOMMENDED - adds latency and failure modes
user_roles := http.send({
    "url": "https://idp.example.com/users/{user_id}/roles",
    "method": "GET"
}).body.roles
```

**Cons**:

- Adds latency to every authorization decision
- Creates runtime dependency on external services
- Can cause cascading failures

---

## 3. Recommended Architecture

### 3.1 External Data Sync Service

New component in Eunomia control plane:

```
┌─────────────────────────────────────────────────────────────────┐
│                    Eunomia Control Plane                         │
│                                                                  │
│  ┌──────────────────┐    ┌──────────────────┐                   │
│  │  External Data   │    │   Data Pusher    │                   │
│  │  Sync Service    │───▶│   (to instances) │                   │
│  └──────────────────┘    └──────────────────┘                   │
│           │                                                      │
│           │ poll/subscribe                                       │
└───────────┼──────────────────────────────────────────────────────┘
            │
            ▼
    ┌───────────────────────────────────────────────┐
    │              External Data Sources             │
    │                                                │
    │  ┌─────────┐  ┌─────────┐  ┌───────────────┐  │
    │  │   IdP   │  │  LDAP   │  │  Config Store │  │
    │  │ (Okta)  │  │  (AD)   │  │  (Vault/K8s)  │  │
    │  └─────────┘  └─────────┘  └───────────────┘  │
    └───────────────────────────────────────────────┘
```

### 3.2 Data Sync Configuration

```yaml
# eunomia-control-plane/config.yaml
external_data:
  sources:
    - name: okta-groups
      type: okta
      endpoint: https://dev-123456.okta.com
      auth:
        type: api_key
        secret_ref: okta-api-key
      sync:
        interval: 5m
        path: users.groups # OPA data path
      transform:
        # Map Okta groups to Eunomia roles
        mapping:
          "Engineering": ["developer", "viewer"]
          "Engineering-Leads": ["developer", "editor", "viewer"]
          "Platform": ["admin", "developer", "editor", "viewer"]

    - name: feature-flags
      type: http
      endpoint: https://features.internal/api/v1/flags
      auth:
        type: bearer
        secret_ref: features-api-token
      sync:
        interval: 1m
        path: features
```

### 3.3 Data Schema

External data is merged into OPA's data store at configured paths:

```json
{
  "users": {
    "groups": {
      "user-123": ["Engineering", "Backend"],
      "user-456": ["Engineering-Leads", "Platform"]
    }
  },
  "features": {
    "new_auth_flow": true,
    "beta_features": ["user-123", "user-456"]
  }
}
```

### 3.4 Policy Usage

```rego
package users_service.authz

import data.users.groups
import data.features

# Check user's IdP groups
allow if {
    input.caller.type == "user"
    user_groups := groups[input.caller.user_id]
    "Engineering" in user_groups
}

# Feature flag check
allow if {
    input.caller.type == "user"
    features.new_auth_flow
    input.caller.user_id in features.beta_features
}
```

---

## 4. IdP Integration Patterns

### 4.1 Okta Integration

```rust
// eunomia-distributor/src/external_data/okta.rs (future)

pub struct OktaDataSource {
    client: OktaClient,
    config: OktaConfig,
}

impl ExternalDataSource for OktaDataSource {
    async fn sync(&self) -> Result<serde_json::Value, DataSyncError> {
        let users = self.client.list_users().await?;

        let mut groups: HashMap<String, Vec<String>> = HashMap::new();
        for user in users {
            let user_groups = self.client.get_user_groups(&user.id).await?;
            groups.insert(user.id.clone(), user_groups);
        }

        Ok(json!({ "groups": groups }))
    }

    fn data_path(&self) -> &str {
        &self.config.data_path
    }

    fn sync_interval(&self) -> Duration {
        self.config.sync_interval
    }
}
```

### 4.2 LDAP/Active Directory Integration

```rust
// eunomia-distributor/src/external_data/ldap.rs (future)

pub struct LdapDataSource {
    connection: LdapConnection,
    config: LdapConfig,
}

impl ExternalDataSource for LdapDataSource {
    async fn sync(&self) -> Result<serde_json::Value, DataSyncError> {
        let search = self.connection.search(
            &self.config.base_dn,
            LdapScope::Subtree,
            "(objectClass=user)",
            &["memberOf", "sAMAccountName"],
        ).await?;

        let mut user_groups: HashMap<String, Vec<String>> = HashMap::new();
        for entry in search {
            let username = entry.get("sAMAccountName")?;
            let groups = entry.get_all("memberOf")?
                .iter()
                .map(|dn| extract_cn(dn))
                .collect();
            user_groups.insert(username, groups);
        }

        Ok(json!({ "ad_groups": user_groups }))
    }
}
```

---

## 5. Caching and Resilience

### 5.1 Local Cache

Each Archimedes instance caches external data:

```rust
// In Archimedes OPA integration (future)
pub struct ExternalDataCache {
    data: RwLock<HashMap<String, CachedData>>,
    ttl: Duration,
}

struct CachedData {
    value: serde_json::Value,
    fetched_at: Instant,
    version: u64,
}

impl ExternalDataCache {
    pub fn get(&self, path: &str) -> Option<&serde_json::Value> {
        let data = self.data.read().unwrap();
        data.get(path)
            .filter(|d| d.fetched_at.elapsed() < self.ttl)
            .map(|d| &d.value)
    }

    pub fn set(&self, path: &str, value: serde_json::Value, version: u64) {
        let mut data = self.data.write().unwrap();
        data.insert(path.to_string(), CachedData {
            value,
            fetched_at: Instant::now(),
            version,
        });
    }
}
```

### 5.2 Fallback Behavior

When external data is unavailable:

```rego
# Policy should handle missing external data gracefully
default user_groups := []

user_groups := groups[input.caller.user_id] if {
    groups := data.users.groups
    input.caller.user_id in groups
}

# Deny if external data unavailable (safe default)
allow if {
    count(user_groups) > 0
    "required_group" in user_groups
}
```

---

## 6. Security Considerations

### 6.1 Data Sensitivity

- External data may contain PII (user attributes)
- Encrypt data in transit (TLS) and at rest (if cached)
- Apply same audit logging as policy changes

### 6.2 Access Control

- Control plane needs minimal permissions to external systems
- Use service accounts with read-only access
- Rotate credentials regularly

### 6.3 Data Freshness vs Security

| Scenario                | Recommended TTL | Notes                            |
| ----------------------- | --------------- | -------------------------------- |
| User group membership   | 1-5 minutes     | Balance freshness vs load        |
| Feature flags           | 30s - 1 minute  | Can be more frequent             |
| Role changes (security) | < 1 minute      | Fast propagation for revocations |
| Static config           | 5-15 minutes    | Rarely changes                   |

---

## 7. Implementation Phases

### Phase 1: Post-MVP (E5)

- [ ] Define `ExternalDataSource` trait
- [ ] Implement HTTP data source (generic)
- [ ] Add data push to Archimedes instances
- [ ] Local caching in Archimedes

### Phase 2: IdP Integration (E6)

- [ ] Okta integration
- [ ] Azure AD / Entra ID integration
- [ ] LDAP integration

### Phase 3: Advanced Features (E7+)

- [ ] Webhook support for real-time updates
- [ ] Data transformation pipelines
- [ ] Audit logging for external data changes

---

## 8. Alternatives Considered

### 8.1 OPA Bundles with Frequent Updates

Push new bundles more frequently with updated data.

**Rejected**: Bundle updates are heavyweight operations meant for policy changes, not data updates.

### 8.2 Sidecar Data Proxy

Dedicated sidecar that provides data to OPA via localhost HTTP.

**Rejected**: Adds operational complexity, extra container per pod.

### 8.3 OPA HTTP.send with Circuit Breaker

Allow HTTP.send with proper circuit breaker and caching.

**Deferred**: May consider for specific use cases where data must be real-time.

---

## 9. References

- [OPA External Data](https://www.openpolicyagent.org/docs/latest/external-data/)
- [OPA Bundle API](https://www.openpolicyagent.org/docs/latest/management-bundles/)
- [Okta Users API](https://developer.okta.com/docs/reference/api/users/)
- [Microsoft Graph API](https://docs.microsoft.com/en-us/graph/api/overview)
