# RBAC Service Example

This example demonstrates **Role-Based Access Control (RBAC)** with hierarchical roles.

## Role Hierarchy

```
admin
  └── moderator
        └── editor
              └── viewer
                    └── guest
```

Each role inherits permissions from roles below it.

## Operations

| Operation | Required Role |
|-----------|---------------|
| `deleteUser` | admin |
| `updateSystemSettings` | admin |
| `viewAuditLog` | admin |
| `banUser` | moderator |
| `removeContent` | moderator |
| `viewReports` | moderator |
| `createContent` | editor |
| `updateContent` | editor |
| `deleteOwnContent` | editor |
| `viewContent` | viewer |
| `listContent` | viewer |
| `getProfile` | viewer |
| `viewPublicContent` | guest |
| `viewPublicProfile` | guest |

## Usage

Run tests:
```bash
eunomia test examples/policies/rbac-service/
```

Build bundle:
```bash
eunomia build \
  --dir examples/policies/rbac-service \
  --service rbac-service \
  --version 1.0.0 \
  --output bundles/
```

## Example Requests

**Admin deleting a user (allowed):**
```json
{
  "caller": {
    "type": "user",
    "user_id": "admin-1",
    "roles": ["admin"]
  },
  "operation_id": "deleteUser",
  "service": "rbac-service"
}
```

**Viewer trying to create content (denied):**
```json
{
  "caller": {
    "type": "user",
    "user_id": "viewer-1",
    "roles": ["viewer"]
  },
  "operation_id": "createContent",
  "service": "rbac-service"
}
```
