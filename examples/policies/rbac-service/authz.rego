# METADATA
# title: RBAC Service Authorization
# description: Role-based access control with hierarchical roles
# scope: service
# entrypoint: true
package rbac_service.authz

import future.keywords.if
import future.keywords.in
import future.keywords.contains
import future.keywords.every

# Default deny - all requests denied unless explicitly allowed
default allow := false

# =============================================================================
# Role Hierarchy
# =============================================================================
# Roles inherit permissions from lower roles:
# admin > moderator > editor > viewer > guest

role_hierarchy := {
    "admin": {"admin", "moderator", "editor", "viewer", "guest"},
    "moderator": {"moderator", "editor", "viewer", "guest"},
    "editor": {"editor", "viewer", "guest"},
    "viewer": {"viewer", "guest"},
    "guest": {"guest"}
}

# Check if user has required role (including inherited roles)
has_role(required) if {
    some user_role in input.caller.roles
    required in role_hierarchy[user_role]
}

# =============================================================================
# Operation Permissions
# =============================================================================
# Map operations to minimum required role

operation_roles := {
    # Admin operations
    "deleteUser": "admin",
    "updateSystemSettings": "admin",
    "viewAuditLog": "admin",
    
    # Moderator operations
    "banUser": "moderator",
    "removeContent": "moderator",
    "viewReports": "moderator",
    
    # Editor operations
    "createContent": "editor",
    "updateContent": "editor",
    "deleteOwnContent": "editor",
    
    # Viewer operations
    "viewContent": "viewer",
    "listContent": "viewer",
    "getProfile": "viewer",
    
    # Guest operations
    "viewPublicContent": "guest",
    "viewPublicProfile": "guest"
}

# =============================================================================
# Authorization Rules
# =============================================================================

# Allow if user has the required role for the operation
allow if {
    input.caller.type == "user"
    required_role := operation_roles[input.operation_id]
    has_role(required_role)
}

# Admins can always access everything
allow if {
    input.caller.type == "user"
    has_role("admin")
}

# =============================================================================
# Decision Details (for debugging/logging)
# =============================================================================

# Provide reason for decision
reason := msg if {
    allow
    msg := sprintf("User has required role for %s", [input.operation_id])
} else := msg if {
    not input.caller.type == "user"
    msg := sprintf("Invalid caller type: %s", [input.caller.type])
} else := msg if {
    required := operation_roles[input.operation_id]
    msg := sprintf("User lacks required role '%s' for operation '%s'", [required, input.operation_id])
} else := msg if {
    not operation_roles[input.operation_id]
    msg := sprintf("Unknown operation: %s", [input.operation_id])
}
