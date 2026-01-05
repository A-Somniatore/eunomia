# Multi-Tenant Service Example

This example demonstrates **multi-tenant authorization** for SaaS applications with complete tenant isolation.

## Tenant Structure

Each user belongs to exactly one tenant and has roles within that tenant:

```
Platform
├── Tenant A
│   ├── tenant_admin (can manage tenant)
│   ├── tenant_member (can create/modify resources)
│   └── tenant_viewer (read-only access)
├── Tenant B
│   └── ...
└── Platform Admins (cross-tenant access)
```

## Key Features

1. **Tenant Isolation** - Users can only access resources in their own tenant
2. **Tenant-Scoped Roles** - Different roles within each tenant
3. **Resource Ownership** - Users can always access their own resources
4. **Resource Sharing** - Explicit sharing within a tenant
5. **Cross-Tenant Access** - Platform admins can access any tenant

## Operations

| Category | Operations | Required Role |
|----------|------------|---------------|
| Admin | `inviteTenantMember`, `removeTenantMember`, `updateTenantSettings` | tenant_admin |
| Member | `createResource`, `updateResource`, `deleteResource`, `shareResource` | tenant_member |
| Viewer | `listResources`, `getResource`, `viewTenantProfile` | tenant_viewer |

## Usage

Run tests:
```bash
eunomia test examples/policies/multi-tenant/
```

## Example Requests

**Tenant admin inviting a member (allowed):**
```json
{
  "caller": {
    "type": "user",
    "user_id": "admin-1",
    "tenant_id": "tenant-a",
    "tenant_roles": ["tenant_admin"]
  },
  "operation_id": "inviteTenantMember",
  "context": {
    "tenant_id": "tenant-a"
  }
}
```

**User accessing another tenant (denied):**
```json
{
  "caller": {
    "type": "user",
    "user_id": "user-1",
    "tenant_id": "tenant-a",
    "tenant_roles": ["tenant_member"]
  },
  "operation_id": "listResources",
  "context": {
    "tenant_id": "tenant-b"
  }
}
```

**Accessing own resource (allowed):**
```json
{
  "caller": {
    "type": "user",
    "user_id": "user-1",
    "tenant_id": "tenant-a",
    "tenant_roles": ["tenant_viewer"]
  },
  "operation_id": "updateResource",
  "context": {
    "tenant_id": "tenant-a",
    "resource_owner": "user-1"
  }
}
```
