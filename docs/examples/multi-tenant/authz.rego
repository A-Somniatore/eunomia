# Multi-Tenant Authorization Policy
#
# This policy enforces tenant isolation for multi-tenant SaaS applications.
# Users can only access resources within their assigned tenant.
#
# Input schema:
# {
#   "caller": {
#     "type": "user" | "service",
#     "id": string,
#     "tenant_id": string,
#     "roles": [string]
#   },
#   "resource": {
#     "path": string,
#     "tenant_id": string
#   },
#   "action": "read" | "write" | "delete" | "admin"
# }

package authz

import data.tenant
import future.keywords.if
import future.keywords.in

# Default deny
default allow := false

# Main authorization rule - combines tenant check with permission check
allow if {
    tenant.same_tenant
    has_permission
}

# Super admins can cross tenant boundaries
allow if {
    is_super_admin
}

# Check if caller has required permission within their tenant
has_permission if {
    some role in input.caller.roles
    some permission in data.permissions.tenant_roles[role].permissions
    permission.action == input.action
    glob.match(permission.resource, ["/"], input.resource.path)
}

# Tenant admin has full access within their tenant
has_permission if {
    "tenant_admin" in input.caller.roles
}

# Check if caller is a platform super admin
is_super_admin if {
    "super_admin" in input.caller.roles
}

# Decision reason for audit logging
reason := msg if {
    allow
    tenant.same_tenant
    has_permission
    msg := sprintf("allowed: tenant %s, action %s", [input.caller.tenant_id, input.action])
}

reason := "super_admin: cross-tenant access" if {
    allow
    is_super_admin
}

reason := msg if {
    not allow
    not tenant.same_tenant
    msg := sprintf("denied: tenant mismatch (caller: %s, resource: %s)", 
        [input.caller.tenant_id, input.resource.tenant_id])
}

reason := "denied: insufficient permissions" if {
    not allow
    tenant.same_tenant
    not has_permission
}
