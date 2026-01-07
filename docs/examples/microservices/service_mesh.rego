# Service Mesh Rules Module
#
# Provides namespace isolation and service mesh integration rules.

package service_mesh

import future.keywords.if
import future.keywords.in

# Valid namespaces for service communication
valid_namespaces := {"production", "staging", "development"}

# Check if caller and target are in valid namespaces
valid_namespace if {
    input.caller.namespace in valid_namespaces
    input.target.namespace in valid_namespaces
    same_environment
}

# Services must be in the same environment tier
# Production can only call production, staging only staging, etc.
same_environment if {
    input.caller.namespace == input.target.namespace
}

# Exception: Development can call staging for testing
same_environment if {
    input.caller.namespace == "development"
    input.target.namespace == "staging"
}

# Get the current service identity
current_service := input.caller.id

# Get the current namespace
current_namespace := input.caller.namespace

# Check if mTLS certificate is valid (if present)
valid_mtls if {
    input.caller.certificate.issuer == "cluster-ca"
}

# No certificate means no mTLS requirement (depends on mesh config)
valid_mtls if {
    not input.caller.certificate
}
