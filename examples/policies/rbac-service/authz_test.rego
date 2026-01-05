package rbac_service.authz_test

import data.rbac_service.authz

# =============================================================================
# Admin Role Tests
# =============================================================================

test_admin_can_delete_user if {
    authz.allow with input as {
        "caller": {
            "type": "user",
            "user_id": "admin-1",
            "roles": ["admin"]
        },
        "operation_id": "deleteUser",
        "service": "rbac-service"
    }
}

test_admin_can_update_system_settings if {
    authz.allow with input as {
        "caller": {
            "type": "user",
            "user_id": "admin-1",
            "roles": ["admin"]
        },
        "operation_id": "updateSystemSettings",
        "service": "rbac-service"
    }
}

test_admin_can_perform_viewer_operations if {
    authz.allow with input as {
        "caller": {
            "type": "user",
            "user_id": "admin-1",
            "roles": ["admin"]
        },
        "operation_id": "viewContent",
        "service": "rbac-service"
    }
}

# =============================================================================
# Moderator Role Tests
# =============================================================================

test_moderator_can_ban_user if {
    authz.allow with input as {
        "caller": {
            "type": "user",
            "user_id": "mod-1",
            "roles": ["moderator"]
        },
        "operation_id": "banUser",
        "service": "rbac-service"
    }
}

test_moderator_cannot_delete_user if {
    not authz.allow with input as {
        "caller": {
            "type": "user",
            "user_id": "mod-1",
            "roles": ["moderator"]
        },
        "operation_id": "deleteUser",
        "service": "rbac-service"
    }
}

test_moderator_can_view_content if {
    authz.allow with input as {
        "caller": {
            "type": "user",
            "user_id": "mod-1",
            "roles": ["moderator"]
        },
        "operation_id": "viewContent",
        "service": "rbac-service"
    }
}

# =============================================================================
# Editor Role Tests
# =============================================================================

test_editor_can_create_content if {
    authz.allow with input as {
        "caller": {
            "type": "user",
            "user_id": "editor-1",
            "roles": ["editor"]
        },
        "operation_id": "createContent",
        "service": "rbac-service"
    }
}

test_editor_cannot_ban_user if {
    not authz.allow with input as {
        "caller": {
            "type": "user",
            "user_id": "editor-1",
            "roles": ["editor"]
        },
        "operation_id": "banUser",
        "service": "rbac-service"
    }
}

# =============================================================================
# Viewer Role Tests
# =============================================================================

test_viewer_can_view_content if {
    authz.allow with input as {
        "caller": {
            "type": "user",
            "user_id": "viewer-1",
            "roles": ["viewer"]
        },
        "operation_id": "viewContent",
        "service": "rbac-service"
    }
}

test_viewer_cannot_create_content if {
    not authz.allow with input as {
        "caller": {
            "type": "user",
            "user_id": "viewer-1",
            "roles": ["viewer"]
        },
        "operation_id": "createContent",
        "service": "rbac-service"
    }
}

# =============================================================================
# Guest Role Tests
# =============================================================================

test_guest_can_view_public_content if {
    authz.allow with input as {
        "caller": {
            "type": "user",
            "user_id": "guest-1",
            "roles": ["guest"]
        },
        "operation_id": "viewPublicContent",
        "service": "rbac-service"
    }
}

test_guest_cannot_view_private_content if {
    not authz.allow with input as {
        "caller": {
            "type": "user",
            "user_id": "guest-1",
            "roles": ["guest"]
        },
        "operation_id": "viewContent",
        "service": "rbac-service"
    }
}

# =============================================================================
# Invalid Caller Type Tests
# =============================================================================

test_anonymous_denied if {
    not authz.allow with input as {
        "caller": {
            "type": "anonymous"
        },
        "operation_id": "viewPublicContent",
        "service": "rbac-service"
    }
}

test_spiffe_denied_for_user_operations if {
    not authz.allow with input as {
        "caller": {
            "type": "spiffe",
            "service_name": "other-service",
            "trust_domain": "example.com"
        },
        "operation_id": "viewContent",
        "service": "rbac-service"
    }
}

# =============================================================================
# Unknown Operation Tests
# =============================================================================

test_unknown_operation_denied if {
    not authz.allow with input as {
        "caller": {
            "type": "user",
            "user_id": "admin-1",
            "roles": ["viewer"]
        },
        "operation_id": "unknownOperation",
        "service": "rbac-service"
    }
}

# =============================================================================
# Multiple Roles Tests
# =============================================================================

test_user_with_multiple_roles_gets_highest if {
    authz.allow with input as {
        "caller": {
            "type": "user",
            "user_id": "user-1",
            "roles": ["guest", "moderator"]
        },
        "operation_id": "banUser",
        "service": "rbac-service"
    }
}
