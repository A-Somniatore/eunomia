# METADATA
# title: API Gateway Authorization
# description: Scope-based authorization for API keys with rate limiting metadata
# scope: service
# entrypoint: true
package api_gateway.authz

import future.keywords.if
import future.keywords.in
import future.keywords.contains
import future.keywords.every

# Default deny - all requests denied unless explicitly allowed
default allow := false

# =============================================================================
# Scope Definitions
# =============================================================================

# Map operations to required scopes
operation_scopes := {
    # User operations
    "getUser": ["users:read"],
    "listUsers": ["users:read"],
    "createUser": ["users:write"],
    "updateUser": ["users:write"],
    "deleteUser": ["users:delete"],
    
    # Order operations
    "getOrder": ["orders:read"],
    "listOrders": ["orders:read"],
    "createOrder": ["orders:write"],
    "cancelOrder": ["orders:write"],
    
    # Product operations
    "getProduct": ["products:read"],
    "listProducts": ["products:read"],
    "createProduct": ["products:write", "products:admin"],
    "updateProduct": ["products:write"],
    "deleteProduct": ["products:delete", "products:admin"],
    
    # Analytics operations (require multiple scopes)
    "getAnalytics": ["analytics:read"],
    "generateReport": ["analytics:read", "analytics:export"],
    "deleteAnalytics": ["analytics:admin"]
}

# =============================================================================
# Scope Hierarchy
# =============================================================================

# Some scopes imply others
scope_includes := {
    "admin": {"admin", "users:read", "users:write", "users:delete", 
              "orders:read", "orders:write", "products:read", 
              "products:write", "products:delete", "products:admin",
              "analytics:read", "analytics:export", "analytics:admin"},
    "products:admin": {"products:admin", "products:read", "products:write", "products:delete"},
    "analytics:admin": {"analytics:admin", "analytics:read", "analytics:export"}
}

# Get all effective scopes for a scope (including implied scopes)
effective_scopes(scope) := scope_includes[scope] if {
    scope_includes[scope]
} else := {scope}

# Get all effective scopes for a list of scopes
all_effective_scopes contains scope if {
    some s in input.caller.scopes
    scope in effective_scopes(s)
}

# Check if caller has all required scopes
has_all_scopes(required) if {
    every scope in required {
        scope in all_effective_scopes
    }
}

# =============================================================================
# API Key Authorization
# =============================================================================

allow if {
    input.caller.type == "api_key"
    is_key_active
    required := operation_scopes[input.operation_id]
    has_all_scopes(required)
}

# Check if API key is active and not expired
is_key_active if {
    input.caller.key_status == "active"
    not key_expired
}

key_expired if {
    input.caller.expires_at
    ts := time.parse_rfc3339_ns(input.caller.expires_at)
    now := time.parse_rfc3339_ns(input.timestamp)
    ts < now
}

# =============================================================================
# User Authorization (for comparison)
# =============================================================================

# Users with specific roles get corresponding scopes
user_scopes contains scope if {
    input.caller.type == "user"
    "admin" in input.caller.roles
    scope in scope_includes["admin"]
}

user_scopes contains "users:read" if {
    input.caller.type == "user"
    "user" in input.caller.roles
}

user_scopes contains "orders:read" if {
    input.caller.type == "user"
    "user" in input.caller.roles
}

allow if {
    input.caller.type == "user"
    required := operation_scopes[input.operation_id]
    every scope in required {
        scope in user_scopes
    }
}

# =============================================================================
# Service-to-Service Authorization
# =============================================================================

# Trusted internal services with specific permissions
service_scopes := {
    "orders-service": {"users:read"},
    "analytics-service": {"users:read", "orders:read", "products:read"},
    "admin-service": {"admin"}
}

allow if {
    input.caller.type == "spiffe"
    input.caller.trust_domain == "somniatore.com"
    allowed := service_scopes[input.caller.service_name]
    required := operation_scopes[input.operation_id]
    every scope in required {
        some s in allowed
        scope in effective_scopes(s)
    }
}

# =============================================================================
# Rate Limiting Metadata
# =============================================================================

# Return rate limit tier based on key
rate_limit_tier := tier if {
    input.caller.type == "api_key"
    tier := input.caller.rate_limit_tier
} else := "default"

rate_limits := {
    "enterprise": {"requests_per_minute": 10000, "requests_per_day": 1000000},
    "professional": {"requests_per_minute": 1000, "requests_per_day": 100000},
    "starter": {"requests_per_minute": 100, "requests_per_day": 10000},
    "default": {"requests_per_minute": 10, "requests_per_day": 1000}
}

# Expose rate limit for the caller
current_rate_limit := rate_limits[rate_limit_tier]

# =============================================================================
# Decision Details
# =============================================================================

reason := msg if {
    allow
    msg := sprintf("Access granted with scopes: %v", [all_effective_scopes])
} else := msg if {
    not is_key_active
    key_expired
    msg := "API key has expired"
} else := msg if {
    not is_key_active
    msg := sprintf("API key status is not active: %s", [input.caller.key_status])
} else := msg if {
    required := operation_scopes[input.operation_id]
    missing := {s | some s in required; not s in all_effective_scopes}
    msg := sprintf("Missing required scopes: %v", [missing])
} else := msg if {
    not operation_scopes[input.operation_id]
    msg := sprintf("Unknown operation: %s", [input.operation_id])
} else := msg if {
    msg := "Access denied"
}
