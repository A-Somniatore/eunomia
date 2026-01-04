# METADATA
# title: Users Service Authorization Policy Tests
# description: Comprehensive tests for users-service authorization
# scope: test
package users_service.authz_test

import future.keywords.if
import data.users_service.authz

# =============================================================================
# Admin Access Tests
# =============================================================================

test_admin_can_read_any_user if {
    authz.allow with input as {
        "caller": {
            "type": "user",
            "user_id": "admin-1",
            "roles": ["admin"]
        },
        "operation_id": "getUser",
        "method": "GET",
        "path": "/users/other-user-123"
    }
}

test_admin_can_delete_any_user if {
    authz.allow with input as {
        "caller": {
            "type": "user",
            "user_id": "admin-1",
            "roles": ["admin"]
        },
        "operation_id": "deleteUser",
        "method": "DELETE",
        "path": "/users/user-to-delete"
    }
}

test_admin_can_list_all_users if {
    authz.allow with input as {
        "caller": {
            "type": "user",
            "user_id": "admin-1",
            "roles": ["admin", "support"]
        },
        "operation_id": "listUsers",
        "method": "GET",
        "path": "/users"
    }
}

# =============================================================================
# User Self-Service Tests
# =============================================================================

test_user_can_read_own_profile if {
    authz.allow with input as {
        "caller": {
            "type": "user",
            "user_id": "user-123",
            "roles": ["user"]
        },
        "operation_id": "getUser",
        "method": "GET",
        "path": "/users/user-123"
    }
}

test_user_can_update_own_profile if {
    authz.allow with input as {
        "caller": {
            "type": "user",
            "user_id": "user-123",
            "roles": ["user"]
        },
        "operation_id": "updateUser",
        "method": "PUT",
        "path": "/users/user-123"
    }
}

test_user_can_patch_own_profile if {
    authz.allow with input as {
        "caller": {
            "type": "user",
            "user_id": "user-123",
            "roles": ["user"]
        },
        "operation_id": "updateUser",
        "method": "PATCH",
        "path": "/users/user-123"
    }
}

test_user_cannot_read_other_profile if {
    not authz.allow with input as {
        "caller": {
            "type": "user",
            "user_id": "user-123",
            "roles": ["user"]
        },
        "operation_id": "getUser",
        "method": "GET",
        "path": "/users/other-user-456"
    }
}

test_user_cannot_update_other_profile if {
    not authz.allow with input as {
        "caller": {
            "type": "user",
            "user_id": "user-123",
            "roles": ["user"]
        },
        "operation_id": "updateUser",
        "method": "PUT",
        "path": "/users/other-user-456"
    }
}

test_user_cannot_delete_own_profile if {
    not authz.allow with input as {
        "caller": {
            "type": "user",
            "user_id": "user-123",
            "roles": ["user"]
        },
        "operation_id": "deleteUser",
        "method": "DELETE",
        "path": "/users/user-123"
    }
}

# =============================================================================
# Service-to-Service Tests
# =============================================================================

test_orders_service_can_read_user if {
    authz.allow with input as {
        "caller": {
            "type": "spiffe",
            "service_name": "orders-service",
            "trust_domain": "somniatore.com"
        },
        "operation_id": "getUser",
        "method": "GET",
        "path": "/users/user-123"
    }
}

test_orders_service_can_get_user_email if {
    authz.allow with input as {
        "caller": {
            "type": "spiffe",
            "service_name": "orders-service",
            "trust_domain": "somniatore.com"
        },
        "operation_id": "getUserEmail",
        "method": "GET",
        "path": "/users/user-123/email"
    }
}

test_orders_service_cannot_update_user if {
    not authz.allow with input as {
        "caller": {
            "type": "spiffe",
            "service_name": "orders-service",
            "trust_domain": "somniatore.com"
        },
        "operation_id": "updateUser",
        "method": "PUT",
        "path": "/users/user-123"
    }
}

test_billing_service_can_read_billing_info if {
    authz.allow with input as {
        "caller": {
            "type": "spiffe",
            "service_name": "billing-service",
            "trust_domain": "somniatore.com"
        },
        "operation_id": "getUserBillingInfo",
        "method": "GET",
        "path": "/users/user-123/billing"
    }
}

test_notification_service_can_read_preferences if {
    authz.allow with input as {
        "caller": {
            "type": "spiffe",
            "service_name": "notification-service",
            "trust_domain": "somniatore.com"
        },
        "operation_id": "getUserNotificationPreferences",
        "method": "GET",
        "path": "/users/user-123/preferences"
    }
}

test_unknown_service_cannot_read_user if {
    not authz.allow with input as {
        "caller": {
            "type": "spiffe",
            "service_name": "unknown-service",
            "trust_domain": "somniatore.com"
        },
        "operation_id": "getUser",
        "method": "GET",
        "path": "/users/user-123"
    }
}

test_wrong_trust_domain_denied if {
    not authz.allow with input as {
        "caller": {
            "type": "spiffe",
            "service_name": "orders-service",
            "trust_domain": "malicious.com"
        },
        "operation_id": "getUser",
        "method": "GET",
        "path": "/users/user-123"
    }
}

# =============================================================================
# API Key Tests
# =============================================================================

test_api_key_with_read_scope_can_get_user if {
    authz.allow with input as {
        "caller": {
            "type": "api_key",
            "scopes": ["users:read"]
        },
        "operation_id": "getUser",
        "method": "GET",
        "path": "/users/user-123"
    }
}

test_api_key_with_write_scope_can_create_user if {
    authz.allow with input as {
        "caller": {
            "type": "api_key",
            "scopes": ["users:write"]
        },
        "operation_id": "createUser",
        "method": "POST",
        "path": "/users"
    }
}

test_api_key_with_write_scope_can_update_user if {
    authz.allow with input as {
        "caller": {
            "type": "api_key",
            "scopes": ["users:write"]
        },
        "operation_id": "updateUser",
        "method": "PUT",
        "path": "/users/user-123"
    }
}

test_api_key_with_read_scope_cannot_update if {
    not authz.allow with input as {
        "caller": {
            "type": "api_key",
            "scopes": ["users:read"]
        },
        "operation_id": "updateUser",
        "method": "PUT",
        "path": "/users/user-123"
    }
}

test_api_key_with_admin_scope_can_delete if {
    authz.allow with input as {
        "caller": {
            "type": "api_key",
            "scopes": ["users:admin"]
        },
        "operation_id": "deleteUser",
        "method": "DELETE",
        "path": "/users/user-123"
    }
}

test_api_key_with_no_relevant_scope_denied if {
    not authz.allow with input as {
        "caller": {
            "type": "api_key",
            "scopes": ["orders:read", "billing:write"]
        },
        "operation_id": "getUser",
        "method": "GET",
        "path": "/users/user-123"
    }
}

# =============================================================================
# Default Deny Tests
# =============================================================================

test_unknown_caller_type_denied if {
    not authz.allow with input as {
        "caller": {
            "type": "unknown"
        },
        "operation_id": "getUser",
        "method": "GET",
        "path": "/users/user-123"
    }
}

test_empty_input_denied if {
    not authz.allow with input as {}
}

test_missing_caller_denied if {
    not authz.allow with input as {
        "operation_id": "getUser",
        "method": "GET",
        "path": "/users/user-123"
    }
}
