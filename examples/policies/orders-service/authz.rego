# METADATA
# title: Orders Service Authorization Policy
# description: Authorization rules for the orders-service
# authors:
#   - Themis Platform Team
# scope: service
package orders_service.authz

import future.keywords.if
import future.keywords.in
import data.common.authz as base

# Default deny
default allow := false

# =============================================================================
# Admin Access
# =============================================================================

# Admins have full access
allow if {
    base.is_admin
}

# =============================================================================
# Customer Access
# =============================================================================

# Customers can view their own orders
allow if {
    base.is_user
    input.operation_id == "getOrder"
    base.is_read_operation
    order_belongs_to_caller
}

# Customers can list their own orders
allow if {
    base.is_user
    input.operation_id == "listOrders"
    base.is_read_operation
    # Query parameter filtering is handled by the service
}

# Customers can create new orders
allow if {
    base.is_user
    input.operation_id == "createOrder"
    input.method == "POST"
}

# Customers can cancel their own pending orders
allow if {
    base.is_user
    input.operation_id == "cancelOrder"
    order_belongs_to_caller
    order_is_cancellable
}

# =============================================================================
# Service-to-Service Access
# =============================================================================

# Payment service can update order payment status
allow if {
    base.is_trusted_service("payment-service", "somniatore.com")
    input.operation_id == "updateOrderPaymentStatus"
    input.method in ["PUT", "PATCH"]
}

# Inventory service can check order items
allow if {
    base.is_trusted_service("inventory-service", "somniatore.com")
    input.operation_id in ["getOrder", "getOrderItems"]
    base.is_read_operation
}

# Shipping service can update order shipping status
allow if {
    base.is_trusted_service("shipping-service", "somniatore.com")
    input.operation_id == "updateOrderShippingStatus"
    input.method in ["PUT", "PATCH"]
}

# Notification service can read order details for notifications
allow if {
    base.is_trusted_service("notification-service", "somniatore.com")
    input.operation_id in ["getOrder", "getOrderCustomerInfo"]
    base.is_read_operation
}

# Analytics service can read order data for reporting
allow if {
    base.is_trusted_service("analytics-service", "somniatore.com")
    base.is_read_operation
}

# =============================================================================
# API Key Access
# =============================================================================

# API keys with orders:read can view orders
allow if {
    base.has_scope("orders:read")
    base.is_read_operation
}

# API keys with orders:write can create/update orders
allow if {
    base.has_scope("orders:write")
    input.method in ["POST", "PUT", "PATCH"]
}

# API keys with orders:admin can do anything
allow if {
    base.has_scope("orders:admin")
}

# =============================================================================
# Support Staff Access
# =============================================================================

# Support staff can view any order
allow if {
    base.has_role("support")
    base.is_read_operation
}

# Support staff can add notes to orders
allow if {
    base.has_role("support")
    input.operation_id == "addOrderNote"
    input.method == "POST"
}

# Support staff can escalate orders
allow if {
    base.has_role("support")
    input.operation_id == "escalateOrder"
    input.method == "POST"
}

# =============================================================================
# Helper Rules
# =============================================================================

# Check if the order belongs to the caller
order_belongs_to_caller if {
    base.is_user
    order_id := extract_order_id
    # In real implementation, this would check against data
    input.resource.owner_id == input.caller.user_id
}

# Extract order ID from path
extract_order_id := id if {
    parts := split(input.path, "/")
    count(parts) >= 3
    parts[1] == "orders"
    id := parts[2]
}

# Check if order can be cancelled (based on status in input)
order_is_cancellable if {
    input.resource.status in ["pending", "processing"]
}

# =============================================================================
# Deny Rules (explicit denials)
# =============================================================================

# Never allow deletion of fulfilled orders
deny if {
    input.operation_id == "deleteOrder"
    input.resource.status == "fulfilled"
}

# Final allow decision considers deny rules
final_allow if {
    allow
    not deny
}

# Deny is false by default
default deny := false
