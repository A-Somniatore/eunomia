//! Instance types for Archimedes instances.
//!
//! This module provides types for representing and managing Archimedes
//! instances that receive policy updates.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::{Duration, Instant};

use crate::health::HealthState;

/// Unique identifier for an Archimedes instance.
pub type InstanceId = String;

/// Represents an Archimedes instance.
#[derive(Debug, Clone)]
pub struct Instance {
    /// Unique instance identifier.
    pub id: InstanceId,

    /// gRPC endpoint address.
    pub endpoint: InstanceEndpoint,

    /// Instance metadata (labels, annotations, etc.).
    pub metadata: InstanceMetadata,

    /// Current status of the instance.
    pub status: InstanceStatus,

    /// Last time this instance was seen/updated.
    pub last_seen: Instant,
}

impl Instance {
    /// Creates a new instance.
    pub fn new(id: impl Into<String>, endpoint: impl Into<String>) -> Self {
        let endpoint_string = endpoint.into();
        Self {
            id: id.into(),
            endpoint: InstanceEndpoint::from_string(&endpoint_string),
            metadata: InstanceMetadata::default(),
            status: InstanceStatus::Unknown,
            last_seen: Instant::now(),
        }
    }

    /// Creates a new instance with metadata.
    pub fn with_metadata(mut self, metadata: InstanceMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    /// Returns the service name from metadata.
    pub fn service(&self) -> Option<&str> {
        self.metadata.service.as_deref()
    }

    /// Returns true if the instance is healthy.
    pub fn is_healthy(&self) -> bool {
        matches!(self.status, InstanceStatus::Healthy { .. })
    }

    /// Updates the instance status.
    pub fn update_status(&mut self, status: InstanceStatus) {
        self.status = status;
        self.last_seen = Instant::now();
    }

    /// Returns how long ago this instance was seen.
    pub fn time_since_seen(&self) -> Duration {
        self.last_seen.elapsed()
    }
}

/// Endpoint information for an instance.
#[derive(Debug, Clone)]
pub struct InstanceEndpoint {
    /// Host address (IP or hostname).
    pub host: String,

    /// Port number.
    pub port: u16,

    /// Whether to use TLS.
    pub use_tls: bool,
}

impl InstanceEndpoint {
    /// Creates an endpoint from a string like "host:port" or "https://host:port".
    pub fn from_string(s: &str) -> Self {
        let use_tls = s.starts_with("https://");
        let s = s
            .strip_prefix("https://")
            .or_else(|| s.strip_prefix("http://"))
            .unwrap_or(s);

        let parts: Vec<&str> = s.split(':').collect();
        let host = parts.first().map(ToString::to_string).unwrap_or_default();
        let port = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(8080);

        Self {
            host,
            port,
            use_tls,
        }
    }

    /// Returns the endpoint as a URI string.
    pub fn to_uri(&self) -> String {
        let scheme = if self.use_tls { "https" } else { "http" };
        format!("{}://{}:{}", scheme, self.host, self.port)
    }

    /// Tries to parse as a socket address.
    pub fn to_socket_addr(&self) -> Option<SocketAddr> {
        format!("{}:{}", self.host, self.port).parse().ok()
    }
}

impl std::fmt::Display for InstanceEndpoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.host, self.port)
    }
}

/// Metadata associated with an instance.
#[derive(Debug, Clone, Default)]
pub struct InstanceMetadata {
    /// Service name this instance belongs to.
    pub service: Option<String>,

    /// Labels (e.g., from Kubernetes).
    pub labels: HashMap<String, String>,

    /// Annotations.
    pub annotations: HashMap<String, String>,

    /// Kubernetes namespace (if applicable).
    pub namespace: Option<String>,

    /// Pod name (if Kubernetes).
    pub pod_name: Option<String>,

    /// Node name (if Kubernetes).
    pub node_name: Option<String>,

    /// Version of the Archimedes instance.
    pub version: Option<String>,
}

impl InstanceMetadata {
    /// Creates new metadata with a service name.
    pub fn for_service(service: impl Into<String>) -> Self {
        Self {
            service: Some(service.into()),
            ..Default::default()
        }
    }

    /// Adds a label.
    pub fn with_label(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.labels.insert(key.into(), value.into());
        self
    }

    /// Adds an annotation.
    pub fn with_annotation(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.annotations.insert(key.into(), value.into());
        self
    }

    /// Sets the Kubernetes namespace.
    pub fn with_namespace(mut self, namespace: impl Into<String>) -> Self {
        self.namespace = Some(namespace.into());
        self
    }

    /// Gets a label value.
    pub fn get_label(&self, key: &str) -> Option<&str> {
        self.labels.get(key).map(String::as_str)
    }
}

/// Current status of an instance.
#[derive(Debug, Clone, Default)]
pub enum InstanceStatus {
    /// Status is unknown (not yet checked).
    #[default]
    Unknown,

    /// Instance is healthy.
    Healthy {
        /// Current policy version.
        policy_version: Option<String>,

        /// Last health check time.
        last_check: Instant,
    },

    /// Instance is unhealthy.
    Unhealthy {
        /// Reason for unhealthy state.
        reason: String,

        /// When the instance became unhealthy.
        since: Instant,
    },

    /// Instance is degraded but operational.
    Degraded {
        /// Reason for degradation.
        reason: String,
    },

    /// Instance is unreachable.
    Unreachable {
        /// Last error message.
        last_error: String,

        /// When the instance became unreachable.
        since: Instant,

        /// Number of consecutive failures.
        failure_count: u32,
    },
}

impl InstanceStatus {
    /// Returns the corresponding health state.
    pub fn to_health_state(&self) -> HealthState {
        match self {
            Self::Unknown => HealthState::Unknown,
            Self::Healthy { .. } => HealthState::Healthy,
            Self::Unhealthy { .. } => HealthState::Unhealthy,
            Self::Degraded { .. } => HealthState::Degraded,
            Self::Unreachable { .. } => HealthState::Unreachable,
        }
    }

    /// Returns the current policy version if known.
    pub fn policy_version(&self) -> Option<&str> {
        match self {
            Self::Healthy { policy_version, .. } => policy_version.as_deref(),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_instance_creation() {
        let instance = Instance::new("inst-1", "localhost:8080");
        assert_eq!(instance.id, "inst-1");
        assert_eq!(instance.endpoint.host, "localhost");
        assert_eq!(instance.endpoint.port, 8080);
        assert!(!instance.endpoint.use_tls);
    }

    #[test]
    fn test_instance_with_metadata() {
        let metadata = InstanceMetadata::for_service("my-service")
            .with_label("env", "prod")
            .with_namespace("default");

        let instance = Instance::new("inst-1", "localhost:8080").with_metadata(metadata);

        assert_eq!(instance.service(), Some("my-service"));
        assert_eq!(instance.metadata.get_label("env"), Some("prod"));
        assert_eq!(instance.metadata.namespace, Some("default".to_string()));
    }

    #[test]
    fn test_endpoint_parsing() {
        let ep = InstanceEndpoint::from_string("localhost:8080");
        assert_eq!(ep.host, "localhost");
        assert_eq!(ep.port, 8080);
        assert!(!ep.use_tls);

        let ep_tls = InstanceEndpoint::from_string("https://api.example.com:443");
        assert_eq!(ep_tls.host, "api.example.com");
        assert_eq!(ep_tls.port, 443);
        assert!(ep_tls.use_tls);

        let ep_http = InstanceEndpoint::from_string("http://localhost:9090");
        assert_eq!(ep_http.host, "localhost");
        assert_eq!(ep_http.port, 9090);
        assert!(!ep_http.use_tls);
    }

    #[test]
    fn test_endpoint_to_uri() {
        let ep = InstanceEndpoint {
            host: "localhost".to_string(),
            port: 8080,
            use_tls: false,
        };
        assert_eq!(ep.to_uri(), "http://localhost:8080");

        let ep_tls = InstanceEndpoint {
            host: "secure.example.com".to_string(),
            port: 443,
            use_tls: true,
        };
        assert_eq!(ep_tls.to_uri(), "https://secure.example.com:443");
    }

    #[test]
    fn test_instance_status() {
        let status = InstanceStatus::Healthy {
            policy_version: Some("1.0.0".to_string()),
            last_check: Instant::now(),
        };
        assert_eq!(status.to_health_state(), HealthState::Healthy);
        assert_eq!(status.policy_version(), Some("1.0.0"));

        let unknown = InstanceStatus::Unknown;
        assert_eq!(unknown.to_health_state(), HealthState::Unknown);
        assert_eq!(unknown.policy_version(), None);
    }

    #[test]
    fn test_instance_is_healthy() {
        let mut instance = Instance::new("inst-1", "localhost:8080");
        assert!(!instance.is_healthy());

        instance.update_status(InstanceStatus::Healthy {
            policy_version: Some("1.0".to_string()),
            last_check: Instant::now(),
        });
        assert!(instance.is_healthy());

        instance.update_status(InstanceStatus::Unhealthy {
            reason: "error".to_string(),
            since: Instant::now(),
        });
        assert!(!instance.is_healthy());
    }

    #[test]
    fn test_metadata_builder() {
        let metadata = InstanceMetadata::for_service("api")
            .with_label("version", "v1")
            .with_label("tier", "frontend")
            .with_annotation("owner", "team-a")
            .with_namespace("production");

        assert_eq!(metadata.service, Some("api".to_string()));
        assert_eq!(metadata.get_label("version"), Some("v1"));
        assert_eq!(metadata.get_label("tier"), Some("frontend"));
        assert_eq!(
            metadata.annotations.get("owner"),
            Some(&"team-a".to_string())
        );
        assert_eq!(metadata.namespace, Some("production".to_string()));
    }
}
