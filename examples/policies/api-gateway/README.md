# API Gateway Example

This example demonstrates **scope-based authorization** for API keys with support for scope hierarchies and rate limiting metadata.

## Features

1. **Scope-Based Access** - Operations require specific scopes
2. **Scope Hierarchy** - Admin scopes include sub-scopes
3. **Multiple Scope Requirements** - Some operations require multiple scopes
4. **API Key Expiration** - Time-based key validity
5. **Rate Limiting Tiers** - Metadata for rate limit enforcement
6. **Service-to-Service** - SPIFFE-based internal service access

## Scopes

| Scope | Description |
|-------|-------------|
| `users:read` | Read user information |
| `users:write` | Create/update users |
| `users:delete` | Delete users |
| `orders:read` | Read orders |
| `orders:write` | Create/cancel orders |
| `products:read` | Read products |
| `products:write` | Update products |
| `products:delete` | Delete products |
| `products:admin` | Full product management |
| `analytics:read` | View analytics |
| `analytics:export` | Export analytics data |
| `analytics:admin` | Full analytics access |
| `admin` | Full system access |

## Scope Hierarchy

```
admin
├── users:read
├── users:write
├── users:delete
├── orders:read
├── orders:write
├── products:admin
│   ├── products:read
│   ├── products:write
│   └── products:delete
└── analytics:admin
    ├── analytics:read
    └── analytics:export
```

## Rate Limit Tiers

| Tier | Requests/Minute | Requests/Day |
|------|-----------------|--------------|
| enterprise | 10,000 | 1,000,000 |
| professional | 1,000 | 100,000 |
| starter | 100 | 10,000 |
| default | 10 | 1,000 |

## Usage

Run tests:
```bash
eunomia test examples/policies/api-gateway/
```

## Example Requests

**API key with read scope (allowed):**
```json
{
  "caller": {
    "type": "api_key",
    "key_id": "key-123",
    "key_status": "active",
    "scopes": ["users:read"],
    "rate_limit_tier": "professional"
  },
  "operation_id": "getUser",
  "timestamp": "2026-01-05T10:00:00Z"
}
```

**Expired API key (denied):**
```json
{
  "caller": {
    "type": "api_key",
    "key_id": "key-456",
    "key_status": "active",
    "scopes": ["admin"],
    "expires_at": "2025-01-01T00:00:00Z"
  },
  "operation_id": "getUser",
  "timestamp": "2026-01-05T10:00:00Z"
}
```

**Operation requiring multiple scopes:**
```json
{
  "caller": {
    "type": "api_key",
    "key_id": "key-789",
    "key_status": "active",
    "scopes": ["analytics:read", "analytics:export"]
  },
  "operation_id": "generateReport",
  "timestamp": "2026-01-05T10:00:00Z"
}
```
