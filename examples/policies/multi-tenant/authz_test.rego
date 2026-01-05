package multi_tenant.authz_test

import data.multi_tenant.authz

# =============================================================================
# Tenant Isolation Tests
# =============================================================================

test_user_can_access_own_tenant if {
    authz.allow with input as {
        "caller": {
            "type": "user",
            "user_id": "user-1",
            "tenant_id": "tenant-a",
            "tenant_roles": ["tenant_viewer"]
        },
        "operation_id": "listResources",
        "context": {
            "tenant_id": "tenant-a"
        }
    }
}

test_user_cannot_access_other_tenant if {
    not authz.allow with input as {
        "caller": {
            "type": "user",
            "user_id": "user-1",
            "tenant_id": "tenant-a",
            "tenant_roles": ["tenant_admin"]
        },
        "operation_id": "listResources",
        "context": {
            "tenant_id": "tenant-b"
        }
    }
}

# =============================================================================
# Tenant Admin Tests
# =============================================================================

test_tenant_admin_can_invite_members if {
    authz.allow with input as {
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
}

test_tenant_admin_can_remove_members if {
    authz.allow with input as {
        "caller": {
            "type": "user",
            "user_id": "admin-1",
            "tenant_id": "tenant-a",
            "tenant_roles": ["tenant_admin"]
        },
        "operation_id": "removeTenantMember",
        "context": {
            "tenant_id": "tenant-a"
        }
    }
}

test_tenant_member_cannot_invite if {
    not authz.allow with input as {
        "caller": {
            "type": "user",
            "user_id": "member-1",
            "tenant_id": "tenant-a",
            "tenant_roles": ["tenant_member"]
        },
        "operation_id": "inviteTenantMember",
        "context": {
            "tenant_id": "tenant-a"
        }
    }
}

# =============================================================================
# Tenant Member Tests
# =============================================================================

test_tenant_member_can_create_resource if {
    authz.allow with input as {
        "caller": {
            "type": "user",
            "user_id": "member-1",
            "tenant_id": "tenant-a",
            "tenant_roles": ["tenant_member"]
        },
        "operation_id": "createResource",
        "context": {
            "tenant_id": "tenant-a"
        }
    }
}

test_tenant_viewer_cannot_create_resource if {
    not authz.allow with input as {
        "caller": {
            "type": "user",
            "user_id": "viewer-1",
            "tenant_id": "tenant-a",
            "tenant_roles": ["tenant_viewer"]
        },
        "operation_id": "createResource",
        "context": {
            "tenant_id": "tenant-a"
        }
    }
}

# =============================================================================
# Resource Ownership Tests
# =============================================================================

test_user_can_access_own_resource if {
    authz.allow with input as {
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
}

test_user_cannot_modify_others_resource if {
    not authz.allow with input as {
        "caller": {
            "type": "user",
            "user_id": "user-1",
            "tenant_id": "tenant-a",
            "tenant_roles": ["tenant_viewer"]
        },
        "operation_id": "updateResource",
        "context": {
            "tenant_id": "tenant-a",
            "resource_owner": "user-2"
        }
    }
}

# =============================================================================
# Shared Resource Tests
# =============================================================================

test_user_can_view_shared_resource if {
    authz.allow with input as {
        "caller": {
            "type": "user",
            "user_id": "user-2",
            "tenant_id": "tenant-a",
            "tenant_roles": ["tenant_viewer"]
        },
        "operation_id": "getResource",
        "context": {
            "tenant_id": "tenant-a",
            "resource_owner": "user-1",
            "shared_with": ["user-2", "user-3"]
        }
    }
}

test_user_cannot_view_unshared_resource if {
    not authz.allow with input as {
        "caller": {
            "type": "user",
            "user_id": "user-2",
            "tenant_id": "tenant-a",
            "tenant_roles": ["tenant_viewer"]
        },
        "operation_id": "getResource",
        "context": {
            "tenant_id": "tenant-a",
            "resource_owner": "user-1",
            "shared_with": ["user-3"]
        }
    }
}

# =============================================================================
# Cross-Tenant Access Tests
# =============================================================================

test_platform_admin_can_access_any_tenant if {
    authz.allow with input as {
        "caller": {
            "type": "user",
            "user_id": "platform-admin-1",
            "tenant_id": "platform",
            "roles": ["platform_admin"],
            "permissions": ["cross_tenant_access"]
        },
        "operation_id": "listResources",
        "context": {
            "tenant_id": "tenant-a"
        }
    }
}

test_regular_admin_cannot_access_other_tenant if {
    not authz.allow with input as {
        "caller": {
            "type": "user",
            "user_id": "admin-1",
            "tenant_id": "tenant-a",
            "tenant_roles": ["tenant_admin"],
            "roles": [],
            "permissions": []
        },
        "operation_id": "listResources",
        "context": {
            "tenant_id": "tenant-b"
        }
    }
}

# =============================================================================
# Service-to-Service Tests
# =============================================================================

test_internal_service_can_view_resources if {
    authz.allow with input as {
        "caller": {
            "type": "spiffe",
            "service_name": "analytics-service",
            "trust_domain": "somniatore.com"
        },
        "operation_id": "listResources",
        "context": {
            "tenant_id": "tenant-a"
        }
    }
}

test_external_service_denied if {
    not authz.allow with input as {
        "caller": {
            "type": "spiffe",
            "service_name": "external-service",
            "trust_domain": "external.com"
        },
        "operation_id": "listResources",
        "context": {
            "tenant_id": "tenant-a"
        }
    }
}

test_internal_service_cannot_create_resources if {
    not authz.allow with input as {
        "caller": {
            "type": "spiffe",
            "service_name": "analytics-service",
            "trust_domain": "somniatore.com"
        },
        "operation_id": "createResource",
        "context": {
            "tenant_id": "tenant-a"
        }
    }
}
