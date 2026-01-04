# METADATA
# title: Orders Service Authorization Policy Tests
# description: Tests for orders-service authorization rules
# scope: test
package orders_service.authz_test

import future.keywords.if
import data.orders_service.authz

# =============================================================================
# Admin Access Tests
# =============================================================================

test_admin_can_access_any_order if {
    authz.allow with input as {
        "caller": {
            "type": "user",
            "user_id": "admin-1",
            "roles": ["admin"]
        },
        "operation_id": "getOrder",
        "method": "GET",
        "path": "/orders/order-123"
    }
}

test_admin_can_delete_order if {
    authz.allow with input as {
        "caller": {
            "type": "user",
            "user_id": "admin-1",
            "roles": ["admin"]
        },
        "operation_id": "deleteOrder",
        "method": "DELETE",
        "path": "/orders/order-123"
    }
}

# =============================================================================
# Customer Access Tests
# =============================================================================

test_customer_can_create_order if {
    authz.allow with input as {
        "caller": {
            "type": "user",
            "user_id": "user-123",
            "roles": ["user"]
        },
        "operation_id": "createOrder",
        "method": "POST",
        "path": "/orders"
    }
}

test_customer_can_view_own_order if {
    authz.allow with input as {
        "caller": {
            "type": "user",
            "user_id": "user-123",
            "roles": ["user"]
        },
        "operation_id": "getOrder",
        "method": "GET",
        "path": "/orders/order-456",
        "resource": {
            "owner_id": "user-123"
        }
    }
}

test_customer_cannot_view_other_order if {
    not authz.allow with input as {
        "caller": {
            "type": "user",
            "user_id": "user-123",
            "roles": ["user"]
        },
        "operation_id": "getOrder",
        "method": "GET",
        "path": "/orders/order-456",
        "resource": {
            "owner_id": "other-user"
        }
    }
}

test_customer_can_cancel_pending_order if {
    authz.allow with input as {
        "caller": {
            "type": "user",
            "user_id": "user-123",
            "roles": ["user"]
        },
        "operation_id": "cancelOrder",
        "method": "POST",
        "path": "/orders/order-456/cancel",
        "resource": {
            "owner_id": "user-123",
            "status": "pending"
        }
    }
}

test_customer_cannot_cancel_shipped_order if {
    not authz.allow with input as {
        "caller": {
            "type": "user",
            "user_id": "user-123",
            "roles": ["user"]
        },
        "operation_id": "cancelOrder",
        "method": "POST",
        "path": "/orders/order-456/cancel",
        "resource": {
            "owner_id": "user-123",
            "status": "shipped"
        }
    }
}

# =============================================================================
# Service-to-Service Tests
# =============================================================================

test_payment_service_can_update_payment_status if {
    authz.allow with input as {
        "caller": {
            "type": "spiffe",
            "service_name": "payment-service",
            "trust_domain": "somniatore.com"
        },
        "operation_id": "updateOrderPaymentStatus",
        "method": "PUT",
        "path": "/orders/order-123/payment-status"
    }
}

test_inventory_service_can_read_order if {
    authz.allow with input as {
        "caller": {
            "type": "spiffe",
            "service_name": "inventory-service",
            "trust_domain": "somniatore.com"
        },
        "operation_id": "getOrderItems",
        "method": "GET",
        "path": "/orders/order-123/items"
    }
}

test_shipping_service_can_update_shipping_status if {
    authz.allow with input as {
        "caller": {
            "type": "spiffe",
            "service_name": "shipping-service",
            "trust_domain": "somniatore.com"
        },
        "operation_id": "updateOrderShippingStatus",
        "method": "PATCH",
        "path": "/orders/order-123/shipping-status"
    }
}

test_analytics_service_can_read_orders if {
    authz.allow with input as {
        "caller": {
            "type": "spiffe",
            "service_name": "analytics-service",
            "trust_domain": "somniatore.com"
        },
        "operation_id": "listOrders",
        "method": "GET",
        "path": "/orders"
    }
}

test_unknown_service_denied if {
    not authz.allow with input as {
        "caller": {
            "type": "spiffe",
            "service_name": "malicious-service",
            "trust_domain": "somniatore.com"
        },
        "operation_id": "getOrder",
        "method": "GET",
        "path": "/orders/order-123"
    }
}

# =============================================================================
# Support Staff Tests
# =============================================================================

test_support_can_view_any_order if {
    authz.allow with input as {
        "caller": {
            "type": "user",
            "user_id": "support-1",
            "roles": ["support"]
        },
        "operation_id": "getOrder",
        "method": "GET",
        "path": "/orders/order-123"
    }
}

test_support_can_add_note if {
    authz.allow with input as {
        "caller": {
            "type": "user",
            "user_id": "support-1",
            "roles": ["support"]
        },
        "operation_id": "addOrderNote",
        "method": "POST",
        "path": "/orders/order-123/notes"
    }
}

test_support_can_escalate_order if {
    authz.allow with input as {
        "caller": {
            "type": "user",
            "user_id": "support-1",
            "roles": ["support"]
        },
        "operation_id": "escalateOrder",
        "method": "POST",
        "path": "/orders/order-123/escalate"
    }
}

test_support_cannot_delete_order if {
    not authz.allow with input as {
        "caller": {
            "type": "user",
            "user_id": "support-1",
            "roles": ["support"]
        },
        "operation_id": "deleteOrder",
        "method": "DELETE",
        "path": "/orders/order-123"
    }
}

# =============================================================================
# API Key Tests
# =============================================================================

test_api_key_with_read_scope if {
    authz.allow with input as {
        "caller": {
            "type": "api_key",
            "scopes": ["orders:read"]
        },
        "operation_id": "getOrder",
        "method": "GET",
        "path": "/orders/order-123"
    }
}

test_api_key_with_write_scope_can_create if {
    authz.allow with input as {
        "caller": {
            "type": "api_key",
            "scopes": ["orders:write"]
        },
        "operation_id": "createOrder",
        "method": "POST",
        "path": "/orders"
    }
}

test_api_key_read_only_cannot_create if {
    not authz.allow with input as {
        "caller": {
            "type": "api_key",
            "scopes": ["orders:read"]
        },
        "operation_id": "createOrder",
        "method": "POST",
        "path": "/orders"
    }
}

# =============================================================================
# Deny Rule Tests
# =============================================================================

test_cannot_delete_fulfilled_order if {
    authz.deny with input as {
        "operation_id": "deleteOrder",
        "resource": {
            "status": "fulfilled"
        }
    }
}

# =============================================================================
# Default Deny Tests
# =============================================================================

test_unknown_caller_denied if {
    not authz.allow with input as {
        "caller": {
            "type": "unknown"
        },
        "operation_id": "getOrder",
        "method": "GET",
        "path": "/orders/order-123"
    }
}

test_empty_input_denied if {
    not authz.allow with input as {}
}
