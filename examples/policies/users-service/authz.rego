# METADATA
# title: Users Service Authorization Policy
# description: Authorization rules for the users-service
# authors:
#   - Themis Platform Team
# scope: service
# related_resources:
#   - https://docs.somniatore.com/policies/users-service
package users_service.authz

import future.keywords.if
import future.keywords.in

# Default deny - all requests are denied unless explicitly allowed
default allow := false

# =============================================================================
# Admin Access
# =============================================================================

# Admins can do anything
allow if {
    input.caller.type == "user"
    "admin" in input.caller.roles
}

# =============================================================================
# User Self-Service
# =============================================================================

# Users can read their own profile
allow if {
    input.caller.type == "user"
    input.operation_id == "getUser"
    input.method == "GET"
    user_id_from_path == input.caller.user_id
}

# Users can update their own profile
allow if {
    input.caller.type == "user"
    input.operation_id == "updateUser"
    input.method in ["PUT", "PATCH"]
    user_id_from_path == input.caller.user_id
}

# =============================================================================
# Service-to-Service Access
# =============================================================================

# Orders service can read user info (for order fulfillment)
allow if {
    input.caller.type == "spiffe"
    input.caller.service_name == "orders-service"
    input.caller.trust_domain == "somniatore.com"
    input.operation_id in ["getUser", "getUserEmail"]
    input.method == "GET"
}

# Billing service can read user billing info
allow if {
    input.caller.type == "spiffe"
    input.caller.service_name == "billing-service"
    input.caller.trust_domain == "somniatore.com"
    input.operation_id in ["getUser", "getUserBillingInfo"]
    input.method == "GET"
}

# Notification service can read user contact preferences
allow if {
    input.caller.type == "spiffe"
    input.caller.service_name == "notification-service"
    input.caller.trust_domain == "somniatore.com"
    input.operation_id == "getUserNotificationPreferences"
    input.method == "GET"
}

# =============================================================================
# API Key Access
# =============================================================================

# API keys with "users:read" scope can read user data
allow if {
    input.caller.type == "api_key"
    "users:read" in input.caller.scopes
    input.method == "GET"
}

# API keys with "users:write" scope can create/update users
allow if {
    input.caller.type == "api_key"
    "users:write" in input.caller.scopes
    input.method in ["POST", "PUT", "PATCH"]
}

# API keys with "users:admin" scope have full access
allow if {
    input.caller.type == "api_key"
    "users:admin" in input.caller.scopes
}

# =============================================================================
# Helper Rules
# =============================================================================

# Extract user ID from path like "/users/123" or "/users/user-abc"
user_id_from_path := id if {
    parts := split(input.path, "/")
    count(parts) >= 3
    parts[1] == "users"
    id := parts[2]
}
