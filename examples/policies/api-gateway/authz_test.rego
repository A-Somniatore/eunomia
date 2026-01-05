package api_gateway.authz_test

import data.api_gateway.authz

# =============================================================================
# API Key Basic Tests
# =============================================================================

test_api_key_with_read_scope_can_get_user if {
    authz.allow with input as {
        "caller": {
            "type": "api_key",
            "key_id": "key-1",
            "key_status": "active",
            "scopes": ["users:read"]
        },
        "operation_id": "getUser",
        "timestamp": "2026-01-05T10:00:00Z"
    }
}

test_api_key_without_scope_denied if {
    not authz.allow with input as {
        "caller": {
            "type": "api_key",
            "key_id": "key-1",
            "key_status": "active",
            "scopes": ["products:read"]
        },
        "operation_id": "getUser",
        "timestamp": "2026-01-05T10:00:00Z"
    }
}

test_api_key_with_write_scope_can_create if {
    authz.allow with input as {
        "caller": {
            "type": "api_key",
            "key_id": "key-1",
            "key_status": "active",
            "scopes": ["users:write"]
        },
        "operation_id": "createUser",
        "timestamp": "2026-01-05T10:00:00Z"
    }
}

test_api_key_read_scope_cannot_write if {
    not authz.allow with input as {
        "caller": {
            "type": "api_key",
            "key_id": "key-1",
            "key_status": "active",
            "scopes": ["users:read"]
        },
        "operation_id": "createUser",
        "timestamp": "2026-01-05T10:00:00Z"
    }
}

# =============================================================================
# Scope Hierarchy Tests
# =============================================================================

test_admin_scope_includes_all if {
    authz.allow with input as {
        "caller": {
            "type": "api_key",
            "key_id": "key-admin",
            "key_status": "active",
            "scopes": ["admin"]
        },
        "operation_id": "deleteUser",
        "timestamp": "2026-01-05T10:00:00Z"
    }
}

test_products_admin_includes_product_scopes if {
    authz.allow with input as {
        "caller": {
            "type": "api_key",
            "key_id": "key-1",
            "key_status": "active",
            "scopes": ["products:admin"]
        },
        "operation_id": "deleteProduct",
        "timestamp": "2026-01-05T10:00:00Z"
    }
}

# =============================================================================
# Multiple Scopes Required Tests
# =============================================================================

test_generate_report_requires_multiple_scopes if {
    authz.allow with input as {
        "caller": {
            "type": "api_key",
            "key_id": "key-1",
            "key_status": "active",
            "scopes": ["analytics:read", "analytics:export"]
        },
        "operation_id": "generateReport",
        "timestamp": "2026-01-05T10:00:00Z"
    }
}

test_generate_report_fails_with_only_one_scope if {
    not authz.allow with input as {
        "caller": {
            "type": "api_key",
            "key_id": "key-1",
            "key_status": "active",
            "scopes": ["analytics:read"]
        },
        "operation_id": "generateReport",
        "timestamp": "2026-01-05T10:00:00Z"
    }
}

test_analytics_admin_implies_both_scopes if {
    authz.allow with input as {
        "caller": {
            "type": "api_key",
            "key_id": "key-1",
            "key_status": "active",
            "scopes": ["analytics:admin"]
        },
        "operation_id": "generateReport",
        "timestamp": "2026-01-05T10:00:00Z"
    }
}

# =============================================================================
# API Key Status Tests
# =============================================================================

test_inactive_key_denied if {
    not authz.allow with input as {
        "caller": {
            "type": "api_key",
            "key_id": "key-1",
            "key_status": "inactive",
            "scopes": ["admin"]
        },
        "operation_id": "getUser",
        "timestamp": "2026-01-05T10:00:00Z"
    }
}

test_revoked_key_denied if {
    not authz.allow with input as {
        "caller": {
            "type": "api_key",
            "key_id": "key-1",
            "key_status": "revoked",
            "scopes": ["admin"]
        },
        "operation_id": "getUser",
        "timestamp": "2026-01-05T10:00:00Z"
    }
}

# =============================================================================
# Key Expiration Tests
# =============================================================================

test_expired_key_denied if {
    not authz.allow with input as {
        "caller": {
            "type": "api_key",
            "key_id": "key-1",
            "key_status": "active",
            "scopes": ["admin"],
            "expires_at": "2025-01-01T00:00:00Z"
        },
        "operation_id": "getUser",
        "timestamp": "2026-01-05T10:00:00Z"
    }
}

test_non_expired_key_allowed if {
    authz.allow with input as {
        "caller": {
            "type": "api_key",
            "key_id": "key-1",
            "key_status": "active",
            "scopes": ["users:read"],
            "expires_at": "2027-01-01T00:00:00Z"
        },
        "operation_id": "getUser",
        "timestamp": "2026-01-05T10:00:00Z"
    }
}

test_key_without_expiry_allowed if {
    authz.allow with input as {
        "caller": {
            "type": "api_key",
            "key_id": "key-1",
            "key_status": "active",
            "scopes": ["users:read"]
        },
        "operation_id": "getUser",
        "timestamp": "2026-01-05T10:00:00Z"
    }
}

# =============================================================================
# Service-to-Service Tests
# =============================================================================

test_orders_service_can_read_users if {
    authz.allow with input as {
        "caller": {
            "type": "spiffe",
            "service_name": "orders-service",
            "trust_domain": "somniatore.com"
        },
        "operation_id": "getUser",
        "timestamp": "2026-01-05T10:00:00Z"
    }
}

test_orders_service_cannot_write_users if {
    not authz.allow with input as {
        "caller": {
            "type": "spiffe",
            "service_name": "orders-service",
            "trust_domain": "somniatore.com"
        },
        "operation_id": "createUser",
        "timestamp": "2026-01-05T10:00:00Z"
    }
}

test_admin_service_has_full_access if {
    authz.allow with input as {
        "caller": {
            "type": "spiffe",
            "service_name": "admin-service",
            "trust_domain": "somniatore.com"
        },
        "operation_id": "deleteUser",
        "timestamp": "2026-01-05T10:00:00Z"
    }
}

test_untrusted_service_denied if {
    not authz.allow with input as {
        "caller": {
            "type": "spiffe",
            "service_name": "orders-service",
            "trust_domain": "external.com"
        },
        "operation_id": "getUser",
        "timestamp": "2026-01-05T10:00:00Z"
    }
}

# =============================================================================
# User Authorization Tests
# =============================================================================

test_admin_user_can_delete if {
    authz.allow with input as {
        "caller": {
            "type": "user",
            "user_id": "admin-1",
            "roles": ["admin"]
        },
        "operation_id": "deleteUser",
        "timestamp": "2026-01-05T10:00:00Z"
    }
}

test_regular_user_can_read if {
    authz.allow with input as {
        "caller": {
            "type": "user",
            "user_id": "user-1",
            "roles": ["user"]
        },
        "operation_id": "getUser",
        "timestamp": "2026-01-05T10:00:00Z"
    }
}

test_regular_user_cannot_delete if {
    not authz.allow with input as {
        "caller": {
            "type": "user",
            "user_id": "user-1",
            "roles": ["user"]
        },
        "operation_id": "deleteUser",
        "timestamp": "2026-01-05T10:00:00Z"
    }
}

# =============================================================================
# Rate Limit Tests
# =============================================================================

test_enterprise_rate_limit if {
    authz.current_rate_limit == {"requests_per_minute": 10000, "requests_per_day": 1000000} with input as {
        "caller": {
            "type": "api_key",
            "key_id": "key-1",
            "key_status": "active",
            "scopes": ["users:read"],
            "rate_limit_tier": "enterprise"
        }
    }
}

test_default_rate_limit if {
    authz.current_rate_limit == {"requests_per_minute": 10, "requests_per_day": 1000} with input as {
        "caller": {
            "type": "api_key",
            "key_id": "key-1",
            "key_status": "active",
            "scopes": ["users:read"]
        }
    }
}
