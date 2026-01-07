# Multi-Tenant Authorization Tests
#
# Test cases for tenant isolation and permissions.
# Run with: eunomia test .

package authz_test

import data.authz
import data.tenant

# ============================================================================
# Tenant Isolation Tests
# ============================================================================

test_same_tenant_allowed {
    authz.allow with input as {
        "caller": {
            "type": "user",
            "id": "user-1",
            "tenant_id": "tenant-abc",
            "roles": ["member"]
        },
        "resource": {
            "path": "/api/documents/123",
            "tenant_id": "tenant-abc"
        },
        "action": "read"
    }
}

test_different_tenant_denied {
    not authz.allow with input as {
        "caller": {
            "type": "user",
            "id": "user-1",
            "tenant_id": "tenant-abc",
            "roles": ["member"]
        },
        "resource": {
            "path": "/api/documents/123",
            "tenant_id": "tenant-xyz"
        },
        "action": "read"
    }
}

# ============================================================================
# Super Admin Tests
# ============================================================================

test_super_admin_cross_tenant {
    authz.allow with input as {
        "caller": {
            "type": "user",
            "id": "admin-1",
            "tenant_id": "platform",
            "roles": ["super_admin"]
        },
        "resource": {
            "path": "/api/documents/123",
            "tenant_id": "tenant-abc"
        },
        "action": "read"
    }
}

test_super_admin_any_action {
    authz.allow with input as {
        "caller": {
            "type": "user",
            "id": "admin-1",
            "tenant_id": "platform",
            "roles": ["super_admin"]
        },
        "resource": {
            "path": "/api/admin/settings",
            "tenant_id": "tenant-abc"
        },
        "action": "admin"
    }
}

# ============================================================================
# Tenant Admin Tests
# ============================================================================

test_tenant_admin_full_access {
    authz.allow with input as {
        "caller": {
            "type": "user",
            "id": "tadmin-1",
            "tenant_id": "tenant-abc",
            "roles": ["tenant_admin"]
        },
        "resource": {
            "path": "/api/documents/123",
            "tenant_id": "tenant-abc"
        },
        "action": "delete"
    }
}

test_tenant_admin_cannot_cross_tenant {
    not authz.allow with input as {
        "caller": {
            "type": "user",
            "id": "tadmin-1",
            "tenant_id": "tenant-abc",
            "roles": ["tenant_admin"]
        },
        "resource": {
            "path": "/api/documents/123",
            "tenant_id": "tenant-xyz"
        },
        "action": "read"
    }
}

# ============================================================================
# Member Role Tests
# ============================================================================

test_member_can_read_documents {
    authz.allow with input as {
        "caller": {
            "type": "user",
            "id": "member-1",
            "tenant_id": "tenant-abc",
            "roles": ["member"]
        },
        "resource": {
            "path": "/api/documents/123",
            "tenant_id": "tenant-abc"
        },
        "action": "read"
    }
}

test_member_can_write_documents {
    authz.allow with input as {
        "caller": {
            "type": "user",
            "id": "member-1",
            "tenant_id": "tenant-abc",
            "roles": ["member"]
        },
        "resource": {
            "path": "/api/documents/123",
            "tenant_id": "tenant-abc"
        },
        "action": "write"
    }
}

test_member_cannot_delete {
    not authz.allow with input as {
        "caller": {
            "type": "user",
            "id": "member-1",
            "tenant_id": "tenant-abc",
            "roles": ["member"]
        },
        "resource": {
            "path": "/api/documents/123",
            "tenant_id": "tenant-abc"
        },
        "action": "delete"
    }
}

test_member_cannot_access_billing {
    not authz.allow with input as {
        "caller": {
            "type": "user",
            "id": "member-1",
            "tenant_id": "tenant-abc",
            "roles": ["member"]
        },
        "resource": {
            "path": "/api/billing/invoices",
            "tenant_id": "tenant-abc"
        },
        "action": "read"
    }
}

# ============================================================================
# Viewer Role Tests
# ============================================================================

test_viewer_can_read {
    authz.allow with input as {
        "caller": {
            "type": "user",
            "id": "viewer-1",
            "tenant_id": "tenant-abc",
            "roles": ["viewer"]
        },
        "resource": {
            "path": "/api/documents/123",
            "tenant_id": "tenant-abc"
        },
        "action": "read"
    }
}

test_viewer_cannot_write {
    not authz.allow with input as {
        "caller": {
            "type": "user",
            "id": "viewer-1",
            "tenant_id": "tenant-abc",
            "roles": ["viewer"]
        },
        "resource": {
            "path": "/api/documents/123",
            "tenant_id": "tenant-abc"
        },
        "action": "write"
    }
}

# ============================================================================
# Billing Role Tests
# ============================================================================

test_billing_can_access_billing {
    authz.allow with input as {
        "caller": {
            "type": "user",
            "id": "billing-1",
            "tenant_id": "tenant-abc",
            "roles": ["billing"]
        },
        "resource": {
            "path": "/api/billing/invoices",
            "tenant_id": "tenant-abc"
        },
        "action": "read"
    }
}

test_billing_cannot_access_documents {
    not authz.allow with input as {
        "caller": {
            "type": "user",
            "id": "billing-1",
            "tenant_id": "tenant-abc",
            "roles": ["billing"]
        },
        "resource": {
            "path": "/api/documents/123",
            "tenant_id": "tenant-abc"
        },
        "action": "read"
    }
}

# ============================================================================
# Tenant Module Tests
# ============================================================================

test_tenant_same_tenant_true {
    tenant.same_tenant with input as {
        "caller": {"tenant_id": "tenant-abc"},
        "resource": {"tenant_id": "tenant-abc"}
    }
}

test_tenant_same_tenant_false {
    not tenant.same_tenant with input as {
        "caller": {"tenant_id": "tenant-abc"},
        "resource": {"tenant_id": "tenant-xyz"}
    }
}

test_tenant_valid_tenant_id {
    tenant.valid_tenant_id with input as {
        "caller": {"tenant_id": "tenant-abc"}
    }
}
