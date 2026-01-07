# Authorization Policy Tests
#
# Test cases for the basic RBAC policy.
# Run with: eunomia test .

package authz_test

import data.authz

# ============================================================================
# Admin Role Tests
# ============================================================================

test_admin_can_read {
    authz.allow with input as {
        "caller": {"type": "user", "id": "admin-1", "roles": ["admin"]},
        "action": "read",
        "resource": "/api/posts/123"
    }
}

test_admin_can_write {
    authz.allow with input as {
        "caller": {"type": "user", "id": "admin-1", "roles": ["admin"]},
        "action": "write",
        "resource": "/api/admin/settings"
    }
}

test_admin_can_delete {
    authz.allow with input as {
        "caller": {"type": "user", "id": "admin-1", "roles": ["admin"]},
        "action": "delete",
        "resource": "/api/users/456"
    }
}

test_admin_can_admin {
    authz.allow with input as {
        "caller": {"type": "user", "id": "admin-1", "roles": ["admin"]},
        "action": "admin",
        "resource": "/api/admin/audit"
    }
}

# ============================================================================
# Editor Role Tests
# ============================================================================

test_editor_can_read_posts {
    authz.allow with input as {
        "caller": {"type": "user", "id": "editor-1", "roles": ["editor"]},
        "action": "read",
        "resource": "/api/posts/123"
    }
}

test_editor_can_write_posts {
    authz.allow with input as {
        "caller": {"type": "user", "id": "editor-1", "roles": ["editor"]},
        "action": "write",
        "resource": "/api/posts/123"
    }
}

test_editor_can_read_comments {
    authz.allow with input as {
        "caller": {"type": "user", "id": "editor-1", "roles": ["editor"]},
        "action": "read",
        "resource": "/api/comments/456"
    }
}

test_editor_can_write_comments {
    authz.allow with input as {
        "caller": {"type": "user", "id": "editor-1", "roles": ["editor"]},
        "action": "write",
        "resource": "/api/comments/456"
    }
}

test_editor_cannot_delete_posts {
    not authz.allow with input as {
        "caller": {"type": "user", "id": "editor-1", "roles": ["editor"]},
        "action": "delete",
        "resource": "/api/posts/123"
    }
}

test_editor_cannot_access_admin {
    not authz.allow with input as {
        "caller": {"type": "user", "id": "editor-1", "roles": ["editor"]},
        "action": "admin",
        "resource": "/api/admin/settings"
    }
}

# ============================================================================
# Viewer Role Tests
# ============================================================================

test_viewer_can_read_posts {
    authz.allow with input as {
        "caller": {"type": "user", "id": "viewer-1", "roles": ["viewer"]},
        "action": "read",
        "resource": "/api/posts/123"
    }
}

test_viewer_cannot_write_posts {
    not authz.allow with input as {
        "caller": {"type": "user", "id": "viewer-1", "roles": ["viewer"]},
        "action": "write",
        "resource": "/api/posts/123"
    }
}

test_viewer_cannot_delete {
    not authz.allow with input as {
        "caller": {"type": "user", "id": "viewer-1", "roles": ["viewer"]},
        "action": "delete",
        "resource": "/api/posts/123"
    }
}

# ============================================================================
# No Role Tests
# ============================================================================

test_no_role_denied_read {
    not authz.allow with input as {
        "caller": {"type": "user", "id": "anon-1", "roles": []},
        "action": "read",
        "resource": "/api/posts/123"
    }
}

test_no_role_denied_write {
    not authz.allow with input as {
        "caller": {"type": "user", "id": "anon-1", "roles": []},
        "action": "write",
        "resource": "/api/posts/123"
    }
}

# ============================================================================
# Multiple Roles Tests
# ============================================================================

test_multiple_roles_inherits_all {
    # User with both editor and viewer roles should have editor permissions
    authz.allow with input as {
        "caller": {"type": "user", "id": "user-1", "roles": ["editor", "viewer"]},
        "action": "write",
        "resource": "/api/posts/123"
    }
}

# ============================================================================
# Reason Tests
# ============================================================================

test_reason_provided_on_allow {
    authz.reason with input as {
        "caller": {"type": "user", "id": "admin-1", "roles": ["admin"]},
        "action": "read",
        "resource": "/api/posts/123"
    }
}

test_reason_provided_on_deny {
    authz.reason == "access denied: no matching permission" with input as {
        "caller": {"type": "user", "id": "anon-1", "roles": []},
        "action": "read",
        "resource": "/api/posts/123"
    }
}
