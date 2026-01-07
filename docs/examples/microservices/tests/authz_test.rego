# Microservices Authorization Tests
#
# Test cases for service-to-service authorization.
# Run with: eunomia test .

package authz_test

import data.authz
import data.service_mesh

# ============================================================================
# Service Dependency Tests
# ============================================================================

test_api_gateway_can_call_users {
    authz.allow with input as {
        "caller": {
            "type": "service",
            "id": "api-gateway",
            "namespace": "production"
        },
        "target": {
            "service": "users-service",
            "method": "GetUser",
            "namespace": "production"
        }
    }
}

test_api_gateway_can_call_orders {
    authz.allow with input as {
        "caller": {
            "type": "service",
            "id": "api-gateway",
            "namespace": "production"
        },
        "target": {
            "service": "orders-service",
            "method": "CreateOrder",
            "namespace": "production"
        }
    }
}

test_api_gateway_cannot_call_inventory_directly {
    not authz.allow with input as {
        "caller": {
            "type": "service",
            "id": "api-gateway",
            "namespace": "production"
        },
        "target": {
            "service": "inventory-service",
            "method": "ReserveStock",
            "namespace": "production"
        }
    }
}

test_orders_can_call_inventory {
    authz.allow with input as {
        "caller": {
            "type": "service",
            "id": "orders-service",
            "namespace": "production"
        },
        "target": {
            "service": "inventory-service",
            "method": "ReserveStock",
            "namespace": "production"
        }
    }
}

# ============================================================================
# Method-Level Access Tests
# ============================================================================

test_allowed_method {
    authz.allow with input as {
        "caller": {
            "type": "service",
            "id": "api-gateway",
            "namespace": "production"
        },
        "target": {
            "service": "users-service",
            "method": "ValidateToken",
            "namespace": "production"
        }
    }
}

test_disallowed_method {
    not authz.allow with input as {
        "caller": {
            "type": "service",
            "id": "api-gateway",
            "namespace": "production"
        },
        "target": {
            "service": "users-service",
            "method": "DeleteUser",
            "namespace": "production"
        }
    }
}

test_wildcard_method_access {
    # payments-service has wildcard access to audit-service
    authz.allow with input as {
        "caller": {
            "type": "service",
            "id": "payments-service",
            "namespace": "production"
        },
        "target": {
            "service": "audit-service",
            "method": "LogTransaction",
            "namespace": "production"
        }
    }
}

# ============================================================================
# Health Check Tests
# ============================================================================

test_health_check_always_allowed {
    authz.allow with input as {
        "caller": {
            "type": "service",
            "id": "unknown-service",
            "namespace": "production"
        },
        "target": {
            "service": "users-service",
            "method": "HealthCheck",
            "namespace": "production"
        }
    }
}

test_health_endpoint_allowed {
    authz.allow with input as {
        "caller": {
            "type": "service",
            "id": "monitoring",
            "namespace": "production"
        },
        "target": {
            "service": "orders-service",
            "method": "health",
            "namespace": "production"
        }
    }
}

# ============================================================================
# Namespace Isolation Tests
# ============================================================================

test_same_namespace_allowed {
    authz.allow with input as {
        "caller": {
            "type": "service",
            "id": "api-gateway",
            "namespace": "production"
        },
        "target": {
            "service": "users-service",
            "method": "GetUser",
            "namespace": "production"
        }
    }
}

test_cross_namespace_denied {
    not authz.allow with input as {
        "caller": {
            "type": "service",
            "id": "api-gateway",
            "namespace": "production"
        },
        "target": {
            "service": "users-service",
            "method": "GetUser",
            "namespace": "staging"
        }
    }
}

test_dev_to_staging_allowed {
    authz.allow with input as {
        "caller": {
            "type": "service",
            "id": "api-gateway",
            "namespace": "development"
        },
        "target": {
            "service": "users-service",
            "method": "GetUser",
            "namespace": "staging"
        }
    }
}

test_staging_to_production_denied {
    not authz.allow with input as {
        "caller": {
            "type": "service",
            "id": "api-gateway",
            "namespace": "staging"
        },
        "target": {
            "service": "users-service",
            "method": "GetUser",
            "namespace": "production"
        }
    }
}

# ============================================================================
# Service Mesh Module Tests
# ============================================================================

test_service_mesh_valid_namespace {
    service_mesh.valid_namespace with input as {
        "caller": {"namespace": "production"},
        "target": {"namespace": "production"}
    }
}

test_service_mesh_invalid_namespace {
    not service_mesh.valid_namespace with input as {
        "caller": {"namespace": "unknown"},
        "target": {"namespace": "production"}
    }
}

test_service_mesh_same_environment {
    service_mesh.same_environment with input as {
        "caller": {"namespace": "staging"},
        "target": {"namespace": "staging"}
    }
}

# ============================================================================
# Circular Dependency Prevention Tests
# ============================================================================

test_notifications_cannot_call_orders {
    # Notifications should not be able to call orders (prevents circular deps)
    not authz.allow with input as {
        "caller": {
            "type": "service",
            "id": "notifications-service",
            "namespace": "production"
        },
        "target": {
            "service": "orders-service",
            "method": "GetOrder",
            "namespace": "production"
        }
    }
}

test_audit_cannot_call_payments {
    # Audit should not be able to call payments (prevents circular deps)
    not authz.allow with input as {
        "caller": {
            "type": "service",
            "id": "audit-service",
            "namespace": "production"
        },
        "target": {
            "service": "payments-service",
            "method": "ProcessPayment",
            "namespace": "production"
        }
    }
}
