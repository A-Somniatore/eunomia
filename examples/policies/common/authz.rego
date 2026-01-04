# METADATA
# title: Common Authorization Patterns
# description: Reusable authorization utilities and base rules
# authors:
#   - Themis Platform Team
# scope: library
# entrypoint: false
package common.authz

import future.keywords.if
import future.keywords.in
import future.keywords.contains

# =============================================================================
# Caller Type Checks
# =============================================================================

# Check if caller is a user
is_user if {
    input.caller.type == "user"
}

# Check if caller is a SPIFFE identity (service)
is_service if {
    input.caller.type == "spiffe"
}

# Check if caller is an API key
is_api_key if {
    input.caller.type == "api_key"
}

# =============================================================================
# Role Helpers
# =============================================================================

# Check if user has a specific role
has_role(role) if {
    is_user
    role in input.caller.roles
}

# Check if user has any of the specified roles
has_any_role(roles) if {
    is_user
    some role in roles
    role in input.caller.roles
}

# Check if user has all specified roles
has_all_roles(roles) if {
    is_user
    every role in roles {
        role in input.caller.roles
    }
}

# Check if user is an admin
is_admin if {
    has_role("admin")
}

# Check if user is a super admin
is_super_admin if {
    has_role("super_admin")
}

# =============================================================================
# SPIFFE Identity Helpers
# =============================================================================

# Get the service name from SPIFFE identity
service_name := input.caller.service_name if {
    is_service
}

# Get the trust domain from SPIFFE identity
trust_domain := input.caller.trust_domain if {
    is_service
}

# Check if service is from a specific trust domain
from_trust_domain(domain) if {
    is_service
    input.caller.trust_domain == domain
}

# Check if caller is a specific service from the trusted domain
is_trusted_service(service, domain) if {
    is_service
    input.caller.service_name == service
    input.caller.trust_domain == domain
}

# =============================================================================
# Scope Helpers (for API Keys)
# =============================================================================

# Check if API key has a specific scope
has_scope(scope) if {
    is_api_key
    scope in input.caller.scopes
}

# Check if API key has any of the specified scopes
has_any_scope(scopes) if {
    is_api_key
    some scope in scopes
    scope in input.caller.scopes
}

# Check if API key has all specified scopes
has_all_scopes(scopes) if {
    is_api_key
    every scope in scopes {
        scope in input.caller.scopes
    }
}

# =============================================================================
# HTTP Method Helpers
# =============================================================================

# Check if request is a read operation
is_read_operation if {
    input.method in ["GET", "HEAD", "OPTIONS"]
}

# Check if request is a write operation
is_write_operation if {
    input.method in ["POST", "PUT", "PATCH", "DELETE"]
}

# Check if request is a specific method
is_method(method) if {
    input.method == method
}

# =============================================================================
# Path Helpers
# =============================================================================

# Extract path segments as array
path_segments := segments if {
    trimmed := trim_prefix(input.path, "/")
    segments := split(trimmed, "/")
}

# Get segment at index (0-based)
path_segment(index) := segment if {
    segment := path_segments[index]
}

# Check if path matches a pattern (simple prefix match)
path_starts_with(prefix) if {
    startswith(input.path, prefix)
}

# Check if path contains a segment
path_contains(segment) if {
    segment in path_segments
}

# =============================================================================
# Time-Based Access Control
# =============================================================================

# Check if current time is within business hours (9 AM - 6 PM UTC)
# Note: This requires time to be provided in input
is_business_hours if {
    hour := input.time.hour
    hour >= 9
    hour < 18
}

# Check if request is during weekdays
is_weekday if {
    day := input.time.day_of_week
    day in ["monday", "tuesday", "wednesday", "thursday", "friday"]
}

# =============================================================================
# Rate Limiting Helpers (for use with external data)
# =============================================================================

# Get the current rate limit count for caller
# This assumes rate limit data is provided via data.rate_limits
rate_limit_count := count if {
    caller_id := input.caller.id
    count := data.rate_limits[caller_id].current_count
}

# Check if rate limit is exceeded
rate_limit_exceeded if {
    rate_limit_count > data.rate_limits[input.caller.id].max_requests
}

# =============================================================================
# Audit Logging Helpers
# =============================================================================

# Generate an audit log entry
# This can be used in decisions that need to return structured data
audit_entry := entry if {
    entry := {
        "timestamp": input.time.timestamp,
        "caller_type": input.caller.type,
        "caller_id": input.caller.id,
        "operation": input.operation_id,
        "method": input.method,
        "path": input.path,
        "allowed": allow,
    }
}

# Default allow is false (to be overridden by importing policies)
default allow := false
