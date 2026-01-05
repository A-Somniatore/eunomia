# METADATA
# title: Multi-Tenant Service Authorization
# description: Authorization rules for multi-tenant SaaS applications
# scope: service
# entrypoint: true
package multi_tenant.authz

import future.keywords.if
import future.keywords.in
import future.keywords.contains
import future.keywords.every

# Default deny - all requests denied unless explicitly allowed
default allow := false

# =============================================================================
# Tenant Isolation
# =============================================================================

# Check if caller belongs to the target tenant
same_tenant if {
    input.caller.tenant_id == input.context.tenant_id
}

# Check if caller has cross-tenant access
has_cross_tenant_access if {
    "cross_tenant_access" in input.caller.permissions
}

# =============================================================================
# Tenant Role Definitions
# =============================================================================

# Roles within a tenant
tenant_roles := {
    "tenant_admin": {"tenant_admin", "tenant_member", "tenant_viewer"},
    "tenant_member": {"tenant_member", "tenant_viewer"},
    "tenant_viewer": {"tenant_viewer"}
}

# Check if user has a tenant role
has_tenant_role(required) if {
    some role in input.caller.tenant_roles
    required in tenant_roles[role]
}

# =============================================================================
# Operation Permissions
# =============================================================================

# Operations requiring tenant_admin role
admin_operations := {
    "inviteTenantMember",
    "removeTenantMember",
    "updateTenantSettings",
    "viewTenantBilling",
    "deleteTenant"
}

# Operations requiring tenant_member role
member_operations := {
    "createResource",
    "updateResource",
    "deleteResource",
    "shareResource"
}

# Operations requiring tenant_viewer role (read-only)
viewer_operations := {
    "listResources",
    "getResource",
    "viewTenantProfile"
}

# =============================================================================
# User Authorization Rules
# =============================================================================

# Tenant admins can perform admin operations within their tenant
allow if {
    input.caller.type == "user"
    same_tenant
    input.operation_id in admin_operations
    has_tenant_role("tenant_admin")
}

# Tenant members can perform member operations within their tenant
allow if {
    input.caller.type == "user"
    same_tenant
    input.operation_id in member_operations
    has_tenant_role("tenant_member")
}

# Tenant viewers can perform read-only operations within their tenant
allow if {
    input.caller.type == "user"
    same_tenant
    input.operation_id in viewer_operations
    has_tenant_role("tenant_viewer")
}

# =============================================================================
# Resource Ownership Rules
# =============================================================================

# Users can always access resources they own
allow if {
    input.caller.type == "user"
    same_tenant
    input.context.resource_owner == input.caller.user_id
    input.operation_id in {"getResource", "updateResource", "deleteResource"}
}

# Users can access shared resources
allow if {
    input.caller.type == "user"
    same_tenant
    input.caller.user_id in input.context.shared_with
    input.operation_id in {"getResource"}
}

# =============================================================================
# Service-to-Service Rules
# =============================================================================

# Internal services can access resources for their tenant
allow if {
    input.caller.type == "spiffe"
    input.caller.trust_domain == "somniatore.com"
    input.caller.service_name in {"analytics-service", "notification-service"}
    input.operation_id in viewer_operations
}

# =============================================================================
# Cross-Tenant Access (Platform Admin)
# =============================================================================

# Platform admins can access any tenant
allow if {
    input.caller.type == "user"
    has_cross_tenant_access
    "platform_admin" in input.caller.roles
}

# =============================================================================
# Decision Details
# =============================================================================

reason := msg if {
    allow
    msg := "Access granted"
} else := msg if {
    not same_tenant
    not has_cross_tenant_access
    msg := sprintf("Tenant isolation: caller tenant '%s' != resource tenant '%s'",
        [input.caller.tenant_id, input.context.tenant_id])
} else := msg if {
    input.operation_id in admin_operations
    not has_tenant_role("tenant_admin")
    msg := "Requires tenant_admin role"
} else := msg if {
    input.operation_id in member_operations
    not has_tenant_role("tenant_member")
    msg := "Requires tenant_member role"
} else := msg if {
    msg := "Access denied"
}
