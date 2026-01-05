//! Error types for the distributor crate.
//!
//! This module defines all errors that can occur during policy distribution.

use thiserror::Error;

/// Result type alias for distributor operations.
pub type Result<T> = std::result::Result<T, DistributorError>;

/// Errors that can occur during policy distribution.
#[derive(Error, Debug)]
pub enum DistributorError {
    /// No instances found for the target service.
    #[error("no instances found for service '{service}'")]
    NoInstancesFound {
        /// The service that had no instances.
        service: String,
    },

    /// Instance is unreachable.
    #[error("instance '{instance_id}' at '{endpoint}' is unreachable: {reason}")]
    InstanceUnreachable {
        /// Instance identifier.
        instance_id: String,
        /// Instance endpoint.
        endpoint: String,
        /// Reason for unreachability.
        reason: String,
    },

    /// Instance rejected the policy update.
    #[error("instance '{instance_id}' rejected policy update: {reason}")]
    PolicyRejected {
        /// Instance identifier.
        instance_id: String,
        /// Reason for rejection.
        reason: String,
    },

    /// Discovery failed.
    #[error("discovery failed for service '{service}': {reason}")]
    DiscoveryFailed {
        /// Service name.
        service: String,
        /// Failure reason.
        reason: String,
    },

    /// Health check failed.
    #[error("health check failed for instance '{instance_id}': {reason}")]
    HealthCheckFailed {
        /// Instance identifier.
        instance_id: String,
        /// Failure reason.
        reason: String,
    },

    /// Deployment already in progress.
    #[error("deployment already in progress for service '{service}': {deployment_id}")]
    DeploymentInProgress {
        /// Service name.
        service: String,
        /// Active deployment ID.
        deployment_id: String,
    },

    /// Deployment not found.
    #[error("deployment '{deployment_id}' not found")]
    DeploymentNotFound {
        /// Deployment identifier.
        deployment_id: String,
    },

    /// Invalid configuration.
    #[error("invalid configuration: {reason}")]
    InvalidConfig {
        /// Reason for invalidity.
        reason: String,
    },

    /// Connection error.
    #[error("connection error: {0}")]
    Connection(#[from] ConnectionError),

    /// Timeout error.
    #[error("operation timed out: {operation}")]
    Timeout {
        /// The operation that timed out.
        operation: String,
    },

    /// gRPC transport error.
    #[error("gRPC transport error: {0}")]
    Transport(String),

    /// gRPC status error.
    #[error("gRPC error: {0}")]
    GrpcStatus(String),

    /// IO error.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// State tracking error.
    #[error("state error: {reason}")]
    StateError {
        /// Reason for state error.
        reason: String,
    },

    /// Internal error.
    #[error("internal error: {0}")]
    Internal(String),
}

/// Connection-related errors.
#[derive(Error, Debug)]
pub enum ConnectionError {
    /// Failed to establish connection.
    #[error("failed to connect to {endpoint}: {reason}")]
    ConnectFailed {
        /// Target endpoint.
        endpoint: String,
        /// Failure reason.
        reason: String,
    },

    /// Connection was refused.
    #[error("connection refused by {endpoint}")]
    ConnectionRefused {
        /// Target endpoint.
        endpoint: String,
    },

    /// DNS resolution failed.
    #[error("DNS resolution failed for {host}: {reason}")]
    DnsResolutionFailed {
        /// Hostname.
        host: String,
        /// Failure reason.
        reason: String,
    },

    /// TLS handshake failed.
    #[error("TLS handshake failed with {endpoint}: {reason}")]
    TlsFailed {
        /// Target endpoint.
        endpoint: String,
        /// Failure reason.
        reason: String,
    },
}

impl DistributorError {
    /// Returns true if this error is retryable.
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::InstanceUnreachable { .. }
                | Self::Connection(
                    ConnectionError::ConnectFailed { .. }
                        | ConnectionError::ConnectionRefused { .. }
                )
                | Self::Timeout { .. }
                | Self::Transport(_)
        )
    }

    /// Returns the error code for gRPC responses.
    pub fn grpc_code(&self) -> i32 {
        match self {
            Self::NoInstancesFound { .. } | Self::DeploymentNotFound { .. } => 5, // NOT_FOUND
            Self::PolicyRejected { .. } | Self::InvalidConfig { .. } => 3, // INVALID_ARGUMENT
            Self::DeploymentInProgress { .. } => 6,                        // ALREADY_EXISTS
            Self::Timeout { .. } => 4,                                     // DEADLINE_EXCEEDED
            Self::InstanceUnreachable { .. } | Self::Connection(_) | Self::Transport(_) => 14, // UNAVAILABLE
            Self::GrpcStatus(_)
            | Self::Io(_)
            | Self::StateError { .. }
            | Self::Internal(_)
            | Self::DiscoveryFailed { .. }
            | Self::HealthCheckFailed { .. } => 13, // INTERNAL
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = DistributorError::NoInstancesFound {
            service: "test-service".to_string(),
        };
        assert!(err.to_string().contains("test-service"));
    }

    #[test]
    fn test_instance_unreachable_is_retryable() {
        let err = DistributorError::InstanceUnreachable {
            instance_id: "inst-1".to_string(),
            endpoint: "localhost:8080".to_string(),
            reason: "connection refused".to_string(),
        };
        assert!(err.is_retryable());
    }

    #[test]
    fn test_policy_rejected_not_retryable() {
        let err = DistributorError::PolicyRejected {
            instance_id: "inst-1".to_string(),
            reason: "invalid policy".to_string(),
        };
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_timeout_is_retryable() {
        let err = DistributorError::Timeout {
            operation: "push".to_string(),
        };
        assert!(err.is_retryable());
    }

    #[test]
    fn test_grpc_codes() {
        assert_eq!(
            DistributorError::NoInstancesFound {
                service: "s".to_string()
            }
            .grpc_code(),
            5
        );

        assert_eq!(
            DistributorError::Timeout {
                operation: "o".to_string()
            }
            .grpc_code(),
            4
        );
    }

    #[test]
    fn test_connection_error_display() {
        let err = ConnectionError::ConnectFailed {
            endpoint: "localhost:8080".to_string(),
            reason: "refused".to_string(),
        };
        assert!(err.to_string().contains("localhost:8080"));
        assert!(err.to_string().contains("refused"));
    }
}
