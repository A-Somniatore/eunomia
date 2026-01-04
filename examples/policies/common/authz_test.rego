# METADATA
# title: Common Authorization Patterns Tests
# description: Tests for reusable authorization utilities
# scope: test
package common.authz_test

import future.keywords.if
import data.common.authz

# =============================================================================
# Caller Type Tests
# =============================================================================

test_is_user_true if {
    authz.is_user with input as {
        "caller": {"type": "user"}
    }
}

test_is_user_false_for_service if {
    not authz.is_user with input as {
        "caller": {"type": "spiffe"}
    }
}

test_is_service_true if {
    authz.is_service with input as {
        "caller": {"type": "spiffe"}
    }
}

test_is_api_key_true if {
    authz.is_api_key with input as {
        "caller": {"type": "api_key"}
    }
}

# =============================================================================
# Role Helper Tests
# =============================================================================

test_has_role_true if {
    authz.has_role("admin") with input as {
        "caller": {"type": "user", "roles": ["admin", "user"]}
    }
}

test_has_role_false if {
    not authz.has_role("admin") with input as {
        "caller": {"type": "user", "roles": ["user"]}
    }
}

test_has_any_role_true if {
    authz.has_any_role(["admin", "moderator"]) with input as {
        "caller": {"type": "user", "roles": ["moderator"]}
    }
}

test_has_any_role_false if {
    not authz.has_any_role(["admin", "moderator"]) with input as {
        "caller": {"type": "user", "roles": ["user"]}
    }
}

test_has_all_roles_true if {
    authz.has_all_roles(["admin", "user"]) with input as {
        "caller": {"type": "user", "roles": ["admin", "user", "support"]}
    }
}

test_has_all_roles_false if {
    not authz.has_all_roles(["admin", "super_admin"]) with input as {
        "caller": {"type": "user", "roles": ["admin"]}
    }
}

test_is_admin_true if {
    authz.is_admin with input as {
        "caller": {"type": "user", "roles": ["admin"]}
    }
}

test_is_admin_false if {
    not authz.is_admin with input as {
        "caller": {"type": "user", "roles": ["user"]}
    }
}

# =============================================================================
# SPIFFE Identity Tests
# =============================================================================

test_service_name_extraction if {
    authz.service_name == "orders-service" with input as {
        "caller": {
            "type": "spiffe",
            "service_name": "orders-service",
            "trust_domain": "example.com"
        }
    }
}

test_trust_domain_extraction if {
    authz.trust_domain == "example.com" with input as {
        "caller": {
            "type": "spiffe",
            "service_name": "orders-service",
            "trust_domain": "example.com"
        }
    }
}

test_from_trust_domain_true if {
    authz.from_trust_domain("somniatore.com") with input as {
        "caller": {
            "type": "spiffe",
            "trust_domain": "somniatore.com"
        }
    }
}

test_from_trust_domain_false if {
    not authz.from_trust_domain("somniatore.com") with input as {
        "caller": {
            "type": "spiffe",
            "trust_domain": "malicious.com"
        }
    }
}

test_is_trusted_service_true if {
    authz.is_trusted_service("orders-service", "somniatore.com") with input as {
        "caller": {
            "type": "spiffe",
            "service_name": "orders-service",
            "trust_domain": "somniatore.com"
        }
    }
}

test_is_trusted_service_wrong_domain if {
    not authz.is_trusted_service("orders-service", "somniatore.com") with input as {
        "caller": {
            "type": "spiffe",
            "service_name": "orders-service",
            "trust_domain": "other.com"
        }
    }
}

# =============================================================================
# Scope Tests (API Keys)
# =============================================================================

test_has_scope_true if {
    authz.has_scope("users:read") with input as {
        "caller": {"type": "api_key", "scopes": ["users:read", "orders:read"]}
    }
}

test_has_scope_false if {
    not authz.has_scope("users:write") with input as {
        "caller": {"type": "api_key", "scopes": ["users:read"]}
    }
}

test_has_any_scope_true if {
    authz.has_any_scope(["users:read", "users:write"]) with input as {
        "caller": {"type": "api_key", "scopes": ["users:write"]}
    }
}

test_has_all_scopes_true if {
    authz.has_all_scopes(["users:read", "orders:read"]) with input as {
        "caller": {"type": "api_key", "scopes": ["users:read", "orders:read", "billing:read"]}
    }
}

test_has_all_scopes_false if {
    not authz.has_all_scopes(["users:read", "orders:read"]) with input as {
        "caller": {"type": "api_key", "scopes": ["users:read"]}
    }
}

# =============================================================================
# HTTP Method Tests
# =============================================================================

test_is_read_operation_get if {
    authz.is_read_operation with input as {"method": "GET"}
}

test_is_read_operation_head if {
    authz.is_read_operation with input as {"method": "HEAD"}
}

test_is_read_operation_options if {
    authz.is_read_operation with input as {"method": "OPTIONS"}
}

test_is_read_operation_false_for_post if {
    not authz.is_read_operation with input as {"method": "POST"}
}

test_is_write_operation_post if {
    authz.is_write_operation with input as {"method": "POST"}
}

test_is_write_operation_put if {
    authz.is_write_operation with input as {"method": "PUT"}
}

test_is_write_operation_patch if {
    authz.is_write_operation with input as {"method": "PATCH"}
}

test_is_write_operation_delete if {
    authz.is_write_operation with input as {"method": "DELETE"}
}

test_is_method_true if {
    authz.is_method("POST") with input as {"method": "POST"}
}

# =============================================================================
# Path Helper Tests
# =============================================================================

test_path_segments if {
    authz.path_segments == ["users", "123", "profile"] with input as {
        "path": "/users/123/profile"
    }
}

test_path_segment_index if {
    authz.path_segment(1) == "123" with input as {
        "path": "/users/123/profile"
    }
}

test_path_starts_with_true if {
    authz.path_starts_with("/users") with input as {
        "path": "/users/123"
    }
}

test_path_starts_with_false if {
    not authz.path_starts_with("/orders") with input as {
        "path": "/users/123"
    }
}

test_path_contains_true if {
    authz.path_contains("users") with input as {
        "path": "/users/123"
    }
}

# =============================================================================
# Time-Based Tests
# =============================================================================

test_is_business_hours_true if {
    authz.is_business_hours with input as {
        "time": {"hour": 14}
    }
}

test_is_business_hours_false_early if {
    not authz.is_business_hours with input as {
        "time": {"hour": 7}
    }
}

test_is_business_hours_false_late if {
    not authz.is_business_hours with input as {
        "time": {"hour": 20}
    }
}

test_is_weekday_true if {
    authz.is_weekday with input as {
        "time": {"day_of_week": "wednesday"}
    }
}

test_is_weekday_false if {
    not authz.is_weekday with input as {
        "time": {"day_of_week": "saturday"}
    }
}

# =============================================================================
# Default Deny Test
# =============================================================================

test_default_deny if {
    not authz.allow with input as {}
}
