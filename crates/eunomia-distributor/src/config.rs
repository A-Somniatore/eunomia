//! Configuration types for the distributor.
//!
//! This module provides configuration options for the distributor service,
//! including discovery sources, push settings, and health check parameters.

use std::time::Duration;

use crate::discovery::{Discovery, DiscoverySource, StaticDiscovery};
use crate::error::{DistributorError, Result};
use crate::health::HealthConfig;
use crate::pusher::PushConfig;
use crate::scheduler::SchedulerConfig;

/// Configuration for the distributor service.
#[derive(Debug, Clone, Default)]
pub struct DistributorConfig {
    /// Discovery source configuration.
    pub discovery: DiscoveryConfig,

    /// Push configuration.
    pub push_config: PushConfig,

    /// Health check configuration.
    pub health_config: HealthConfig,

    /// Scheduler configuration.
    pub scheduler_config: SchedulerConfig,

    /// Enable TLS for connections.
    pub tls_enabled: bool,

    /// TLS certificate path (if TLS enabled).
    pub tls_cert_path: Option<String>,

    /// TLS key path (if TLS enabled).
    pub tls_key_path: Option<String>,

    /// CA certificate path for verifying instance certificates.
    pub ca_cert_path: Option<String>,
}

impl DistributorConfig {
    /// Creates a new configuration builder.
    pub fn builder() -> DistributorConfigBuilder {
        DistributorConfigBuilder::default()
    }

    /// Creates the discovery source from configuration.
    pub fn create_discovery(&self) -> Result<Box<dyn Discovery>> {
        match &self.discovery.source {
            DiscoverySource::Static { endpoints } => {
                Ok(Box::new(StaticDiscovery::new(endpoints.clone())))
            }
            DiscoverySource::Kubernetes { namespace, .. } => {
                // For now, return an error - K8s discovery will be implemented later
                Err(DistributorError::InvalidConfig {
                    reason: format!(
                        "Kubernetes discovery not yet implemented (namespace: {})",
                        namespace.as_deref().unwrap_or("default")
                    ),
                })
            }
            DiscoverySource::Dns { hosts, port, .. } => {
                // For now, return an error - DNS discovery will be implemented later
                Err(DistributorError::InvalidConfig {
                    reason: format!(
                        "DNS discovery not yet implemented (hosts: {}, port: {})",
                        hosts.len(),
                        port
                    ),
                })
            }
        }
    }
}

/// Discovery configuration.
#[derive(Debug, Clone)]
pub struct DiscoveryConfig {
    /// Discovery source.
    pub source: DiscoverySource,

    /// Discovery refresh interval.
    pub refresh_interval: Duration,

    /// Enable caching of discovered instances.
    pub cache_enabled: bool,

    /// Cache TTL.
    pub cache_ttl: Duration,
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            source: DiscoverySource::Static {
                endpoints: Vec::new(),
            },
            refresh_interval: Duration::from_secs(30),
            cache_enabled: true,
            cache_ttl: Duration::from_secs(60),
        }
    }
}

/// Builder for `DistributorConfig`.
#[derive(Debug, Default)]
pub struct DistributorConfigBuilder {
    discovery: Option<DiscoveryConfig>,
    push_config: Option<PushConfig>,
    health_config: Option<HealthConfig>,
    scheduler_config: Option<SchedulerConfig>,
    tls_enabled: bool,
    tls_cert_path: Option<String>,
    tls_key_path: Option<String>,
    ca_cert_path: Option<String>,
}

impl DistributorConfigBuilder {
    /// Sets the discovery configuration.
    pub fn discovery(mut self, config: DiscoveryConfig) -> Self {
        self.discovery = Some(config);
        self
    }

    /// Sets the push configuration.
    pub fn push_config(mut self, config: PushConfig) -> Self {
        self.push_config = Some(config);
        self
    }

    /// Sets the health check configuration.
    pub fn health_config(mut self, config: HealthConfig) -> Self {
        self.health_config = Some(config);
        self
    }

    /// Sets the scheduler configuration.
    pub fn scheduler_config(mut self, config: SchedulerConfig) -> Self {
        self.scheduler_config = Some(config);
        self
    }

    /// Enables TLS.
    pub fn tls(mut self, cert_path: String, key_path: String) -> Self {
        self.tls_enabled = true;
        self.tls_cert_path = Some(cert_path);
        self.tls_key_path = Some(key_path);
        self
    }

    /// Sets the CA certificate path.
    pub fn ca_cert(mut self, path: String) -> Self {
        self.ca_cert_path = Some(path);
        self
    }

    /// Sets static endpoints for discovery.
    pub fn static_endpoints(mut self, endpoints: Vec<String>) -> Self {
        self.discovery = Some(DiscoveryConfig {
            source: DiscoverySource::Static { endpoints },
            ..DiscoveryConfig::default()
        });
        self
    }

    /// Builds the configuration.
    pub fn build(self) -> DistributorConfig {
        DistributorConfig {
            discovery: self.discovery.unwrap_or_default(),
            push_config: self.push_config.unwrap_or_default(),
            health_config: self.health_config.unwrap_or_default(),
            scheduler_config: self.scheduler_config.unwrap_or_default(),
            tls_enabled: self.tls_enabled,
            tls_cert_path: self.tls_cert_path,
            tls_key_path: self.tls_key_path,
            ca_cert_path: self.ca_cert_path,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = DistributorConfig::default();
        assert!(!config.tls_enabled);
        assert!(config.tls_cert_path.is_none());
        assert!(config.health_config.check_interval.as_secs() > 0);
    }

    #[test]
    fn test_builder() {
        let config = DistributorConfig::builder()
            .static_endpoints(vec!["localhost:8080".to_string()])
            .tls("cert.pem".to_string(), "key.pem".to_string())
            .ca_cert("ca.pem".to_string())
            .build();

        assert!(config.tls_enabled);
        assert_eq!(config.tls_cert_path, Some("cert.pem".to_string()));
        assert_eq!(config.tls_key_path, Some("key.pem".to_string()));
        assert_eq!(config.ca_cert_path, Some("ca.pem".to_string()));
    }

    #[test]
    fn test_builder_static_endpoints() {
        let config = DistributorConfig::builder()
            .static_endpoints(vec!["host1:8080".to_string(), "host2:8080".to_string()])
            .build();

        if let DiscoverySource::Static { endpoints } = config.discovery.source {
            assert_eq!(endpoints.len(), 2);
        } else {
            panic!("Expected static discovery");
        }
    }

    #[test]
    fn test_create_static_discovery() {
        let config = DistributorConfig::builder()
            .static_endpoints(vec!["localhost:8080".to_string()])
            .build();

        let discovery = config.create_discovery();
        assert!(discovery.is_ok());
    }

    #[test]
    fn test_discovery_config_default() {
        let config = DiscoveryConfig::default();
        assert_eq!(config.refresh_interval, Duration::from_secs(30));
        assert!(config.cache_enabled);
        assert_eq!(config.cache_ttl, Duration::from_secs(60));
    }
}
