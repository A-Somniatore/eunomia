# Tenant Isolation Module
#
# Provides tenant boundary enforcement rules.
# This module ensures users can only access resources within their tenant.

package tenant

import future.keywords.if

# Check if caller and resource belong to the same tenant
same_tenant if {
    input.caller.tenant_id == input.resource.tenant_id
}

# Check if tenant_id is valid (not empty or null)
valid_tenant_id if {
    input.caller.tenant_id
    input.caller.tenant_id != ""
}

# Check if resource has valid tenant context
valid_resource_tenant if {
    input.resource.tenant_id
    input.resource.tenant_id != ""
}

# Get the current tenant context
current_tenant := input.caller.tenant_id
