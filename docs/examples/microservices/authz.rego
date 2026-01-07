# Microservices Authorization Policy
#
# This policy controls service-to-service communication in a microservices
# architecture. It enforces service dependencies and method-level access control.
#
# Input schema:
# {
#   "caller": {
#     "type": "service",
#     "id": string,           # Service name
#     "namespace": string,    # Deployment namespace
#     "certificate": {        # Optional mTLS info
#       "subject": string,
#       "issuer": string
#     }
#   },
#   "target": {
#     "service": string,      # Target service name
#     "method": string,       # RPC method or HTTP endpoint
#     "namespace": string     # Target namespace
#   }
# }

package authz

import data.service_mesh
import future.keywords.if
import future.keywords.in

# Default deny all service-to-service calls
default allow := false

# Allow if service dependency is registered and method is allowed
allow if {
    service_mesh.valid_namespace
    service_dependency_allowed
    method_allowed
}

# Check if caller is allowed to call the target service
service_dependency_allowed if {
    service_config := data.services[input.caller.id]
    input.target.service in service_config.allowed_dependencies
}

# Check if the specific method/endpoint is allowed
method_allowed if {
    service_config := data.services[input.caller.id]
    dep_config := service_config.dependency_config[input.target.service]
    
    # Check if method is in allowed list or wildcard
    allowed_method(dep_config.allowed_methods)
}

# Method is allowed if it matches exactly or wildcard
allowed_method(methods) if {
    "*" in methods
}

allowed_method(methods) if {
    input.target.method in methods
}

# Health checks are always allowed between services
allow if {
    input.target.method == "HealthCheck"
}

allow if {
    input.target.method == "health"
}

# Decision reason for observability
reason := msg if {
    allow
    msg := sprintf("allowed: %s -> %s.%s", 
        [input.caller.id, input.target.service, input.target.method])
}

reason := "denied: service dependency not allowed" if {
    not allow
    not service_dependency_allowed
}

reason := "denied: method not allowed" if {
    not allow
    service_dependency_allowed
    not method_allowed
}

reason := "denied: namespace isolation violation" if {
    not allow
    not service_mesh.valid_namespace
}

# Metrics for observability
service_calls[call] {
    call := {
        "caller": input.caller.id,
        "target": input.target.service,
        "method": input.target.method,
        "allowed": allow
    }
}
