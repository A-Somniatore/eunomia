# Basic RBAC Authorization Policy
#
# This policy implements role-based access control for a REST API.
# Users are granted permissions based on their assigned roles.
#
# Input schema:
# {
#   "caller": {
#     "type": "user" | "service",
#     "id": string,
#     "roles": [string]
#   },
#   "action": "read" | "write" | "delete" | "admin",
#   "resource": string (path pattern)
# }

package authz

import future.keywords.if
import future.keywords.in

# Default deny - all requests are denied unless explicitly allowed
default allow := false

# Allow rule - checks if the caller has permission for the action on resource
allow if {
    some role in input.caller.roles
    some permission in data.roles[role].permissions
    permission.action == input.action
    glob.match(permission.resource, ["/"], input.resource)
}

# Admin role has full access to everything
allow if {
    "admin" in input.caller.roles
}

# Service accounts can access their designated endpoints
allow if {
    input.caller.type == "service"
    some permission in data.service_permissions[input.caller.id]
    permission.action == input.action
    glob.match(permission.resource, ["/"], input.resource)
}

# Reason for the decision (for audit logging)
reason := msg if {
    allow
    some role in input.caller.roles
    some permission in data.roles[role].permissions
    permission.action == input.action
    glob.match(permission.resource, ["/"], input.resource)
    msg := sprintf("%s can %s on %s", [role, input.action, permission.resource])
}

reason := "admin has full access" if {
    allow
    "admin" in input.caller.roles
}

reason := "service account authorized" if {
    allow
    input.caller.type == "service"
}

reason := "access denied: no matching permission" if {
    not allow
}
