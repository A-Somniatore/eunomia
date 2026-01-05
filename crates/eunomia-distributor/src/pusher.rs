//! Policy push client for Archimedes instances.
//!
//! This module provides the client for pushing policy bundles
//! to Archimedes instances via gRPC.

use std::time::{Duration, Instant};

use crate::error::{DistributorError, Result};
use crate::health::HealthCheck;
use crate::instance::Instance;

/// Configuration for the policy pusher.
#[derive(Debug, Clone)]
pub struct PushConfig {
    /// Connection timeout.
    pub connect_timeout: Duration,

    /// Request timeout.
    pub request_timeout: Duration,

    /// Number of retries for failed pushes.
    pub max_retries: u32,

    /// Delay between retries.
    pub retry_delay: Duration,

    /// Enable compression for policy bundles.
    pub compression_enabled: bool,
}

impl Default for PushConfig {
    fn default() -> Self {
        Self {
            connect_timeout: Duration::from_secs(10),
            request_timeout: Duration::from_secs(30),
            max_retries: 3,
            retry_delay: Duration::from_millis(500),
            compression_enabled: true,
        }
    }
}

impl PushConfig {
    /// Creates a configuration builder.
    pub fn builder() -> PushConfigBuilder {
        PushConfigBuilder::default()
    }
}

/// Builder for `PushConfig`.
#[derive(Debug, Default)]
pub struct PushConfigBuilder {
    connect_timeout: Option<Duration>,
    request_timeout: Option<Duration>,
    max_retries: Option<u32>,
    retry_delay: Option<Duration>,
    compression_enabled: Option<bool>,
}

impl PushConfigBuilder {
    /// Sets the connection timeout.
    pub fn connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = Some(timeout);
        self
    }

    /// Sets the request timeout.
    pub fn request_timeout(mut self, timeout: Duration) -> Self {
        self.request_timeout = Some(timeout);
        self
    }

    /// Sets the maximum number of retries.
    pub fn max_retries(mut self, retries: u32) -> Self {
        self.max_retries = Some(retries);
        self
    }

    /// Sets the retry delay.
    pub fn retry_delay(mut self, delay: Duration) -> Self {
        self.retry_delay = Some(delay);
        self
    }

    /// Enables or disables compression.
    pub fn compression(mut self, enabled: bool) -> Self {
        self.compression_enabled = Some(enabled);
        self
    }

    /// Builds the configuration.
    pub fn build(self) -> PushConfig {
        let defaults = PushConfig::default();
        PushConfig {
            connect_timeout: self.connect_timeout.unwrap_or(defaults.connect_timeout),
            request_timeout: self.request_timeout.unwrap_or(defaults.request_timeout),
            max_retries: self.max_retries.unwrap_or(defaults.max_retries),
            retry_delay: self.retry_delay.unwrap_or(defaults.retry_delay),
            compression_enabled: self
                .compression_enabled
                .unwrap_or(defaults.compression_enabled),
        }
    }
}

/// Result of a push operation.
#[derive(Debug, Clone)]
pub struct PushResult {
    /// Instance that was pushed to.
    pub instance_id: String,

    /// Whether the push was successful.
    pub success: bool,

    /// Time taken for the push.
    pub duration: Duration,

    /// Policy version that was pushed.
    pub version: String,

    /// Number of attempts made.
    pub attempts: u32,

    /// Error message if failed.
    pub error: Option<String>,
}

/// Policy pusher client.
///
/// Handles pushing policy bundles to individual Archimedes instances
/// with retry logic and health checking.
pub struct PolicyPusher {
    config: PushConfig,
}

impl PolicyPusher {
    /// Creates a new policy pusher.
    pub fn new(config: PushConfig) -> Self {
        Self { config }
    }

    /// Pushes a policy to an instance.
    ///
    /// # Arguments
    ///
    /// * `instance` - Target instance
    /// * `service` - Service name
    /// * `version` - Policy version to push
    ///
    /// # Returns
    ///
    /// Result of the push operation.
    pub async fn push(
        &self,
        instance: &Instance,
        service: &str,
        version: &str,
    ) -> Result<PushResult> {
        let start = Instant::now();
        let mut attempts = 0;
        let mut last_error: Option<String> = None;

        while attempts < self.config.max_retries {
            attempts += 1;

            match self.try_push(instance, service, version).await {
                Ok(()) => {
                    return Ok(PushResult {
                        instance_id: instance.id.clone(),
                        success: true,
                        duration: start.elapsed(),
                        version: version.to_string(),
                        attempts,
                        error: None,
                    });
                }
                Err(e) => {
                    last_error = Some(e.to_string());

                    if !e.is_retryable() {
                        break;
                    }

                    if attempts < self.config.max_retries {
                        tracing::debug!(
                            instance_id = %instance.id,
                            attempt = attempts,
                            error = %e,
                            "push failed, retrying"
                        );
                        tokio::time::sleep(self.config.retry_delay).await;
                    }
                }
            }
        }

        let error = last_error.unwrap_or_else(|| "unknown error".to_string());
        Ok(PushResult {
            instance_id: instance.id.clone(),
            success: false,
            duration: start.elapsed(),
            version: version.to_string(),
            attempts,
            error: Some(error),
        })
    }

    /// Performs a health check on an instance.
    pub async fn health_check(&self, instance: &Instance) -> Result<HealthCheck> {
        let start = Instant::now();

        // Simulate health check (actual gRPC implementation will be added later)
        // For now, we return a mock response based on instance state
        match &instance.status {
            crate::instance::InstanceStatus::Healthy { policy_version, .. } => Ok(
                HealthCheck::healthy(policy_version.clone(), start.elapsed()),
            ),
            crate::instance::InstanceStatus::Unhealthy { reason, .. } => {
                Ok(HealthCheck::unhealthy(reason.clone()))
            }
            crate::instance::InstanceStatus::Unreachable { last_error, .. } => {
                Ok(HealthCheck::unreachable(last_error.clone()))
            }
            _ => Ok(HealthCheck::unknown()),
        }
    }

    /// Internal push implementation (will use gRPC in full implementation).
    async fn try_push(&self, instance: &Instance, _service: &str, _version: &str) -> Result<()> {
        // Check if instance is reachable first
        if let crate::instance::InstanceStatus::Unreachable { last_error, .. } = &instance.status {
            return Err(DistributorError::InstanceUnreachable {
                instance_id: instance.id.clone(),
                endpoint: instance.endpoint.to_uri(),
                reason: last_error.clone(),
            });
        }

        // Simulate connection timeout
        tokio::time::sleep(Duration::from_millis(10)).await;

        // In the real implementation, this would:
        // 1. Connect to the instance's gRPC endpoint
        // 2. Call PolicyReceiver.UpdatePolicy
        // 3. Wait for acknowledgment
        // 4. Handle errors appropriately

        tracing::debug!(
            instance_id = %instance.id,
            endpoint = %instance.endpoint.to_uri(),
            "simulated push (gRPC implementation pending)"
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::health::HealthState;
    use crate::instance::{Instance, InstanceMetadata, InstanceStatus};

    #[test]
    fn test_push_config_default() {
        let config = PushConfig::default();
        assert_eq!(config.connect_timeout, Duration::from_secs(10));
        assert_eq!(config.request_timeout, Duration::from_secs(30));
        assert_eq!(config.max_retries, 3);
        assert!(config.compression_enabled);
    }

    #[test]
    fn test_push_config_builder() {
        let config = PushConfig::builder()
            .connect_timeout(Duration::from_secs(5))
            .request_timeout(Duration::from_secs(15))
            .max_retries(5)
            .retry_delay(Duration::from_secs(1))
            .compression(false)
            .build();

        assert_eq!(config.connect_timeout, Duration::from_secs(5));
        assert_eq!(config.request_timeout, Duration::from_secs(15));
        assert_eq!(config.max_retries, 5);
        assert_eq!(config.retry_delay, Duration::from_secs(1));
        assert!(!config.compression_enabled);
    }

    #[tokio::test]
    async fn test_push_to_healthy_instance() {
        let pusher = PolicyPusher::new(PushConfig::default());

        let mut instance = Instance::new("inst-1", "localhost:8080")
            .with_metadata(InstanceMetadata::for_service("test-service"));
        instance.update_status(InstanceStatus::Healthy {
            policy_version: Some("0.9.0".to_string()),
            last_check: std::time::Instant::now(),
        });

        let result = pusher.push(&instance, "test-service", "1.0.0").await;
        assert!(result.is_ok());

        let push_result = result.unwrap();
        assert!(push_result.success);
        assert_eq!(push_result.version, "1.0.0");
        assert_eq!(push_result.attempts, 1);
        assert!(push_result.error.is_none());
    }

    #[tokio::test]
    async fn test_push_to_unreachable_instance() {
        let config = PushConfig {
            max_retries: 2,
            retry_delay: Duration::from_millis(10),
            ..PushConfig::default()
        };
        let pusher = PolicyPusher::new(config);

        let mut instance = Instance::new("inst-2", "localhost:9999");
        instance.update_status(InstanceStatus::Unreachable {
            last_error: "connection refused".to_string(),
            since: std::time::Instant::now(),
            failure_count: 3,
        });

        let result = pusher.push(&instance, "test-service", "1.0.0").await;
        assert!(result.is_ok());

        let push_result = result.unwrap();
        assert!(!push_result.success);
        assert!(push_result.error.is_some());
    }

    #[tokio::test]
    async fn test_health_check_healthy_instance() {
        let pusher = PolicyPusher::new(PushConfig::default());

        let mut instance = Instance::new("inst-1", "localhost:8080");
        instance.update_status(InstanceStatus::Healthy {
            policy_version: Some("1.0.0".to_string()),
            last_check: std::time::Instant::now(),
        });

        let health = pusher.health_check(&instance).await;
        assert!(health.is_ok());

        let check = health.unwrap();
        assert_eq!(check.state, HealthState::Healthy);
        assert_eq!(check.policy_version, Some("1.0.0".to_string()));
    }

    #[tokio::test]
    async fn test_health_check_unhealthy_instance() {
        let pusher = PolicyPusher::new(PushConfig::default());

        let mut instance = Instance::new("inst-1", "localhost:8080");
        instance.update_status(InstanceStatus::Unhealthy {
            reason: "high latency".to_string(),
            since: std::time::Instant::now(),
        });

        let health = pusher.health_check(&instance).await;
        assert!(health.is_ok());

        let check = health.unwrap();
        assert_eq!(check.state, HealthState::Unhealthy);
    }

    #[tokio::test]
    async fn test_health_check_unknown_instance() {
        let pusher = PolicyPusher::new(PushConfig::default());
        let instance = Instance::new("inst-1", "localhost:8080");

        let health = pusher.health_check(&instance).await;
        assert!(health.is_ok());

        let check = health.unwrap();
        assert_eq!(check.state, HealthState::Unknown);
    }

    #[test]
    fn test_push_result_success() {
        let result = PushResult {
            instance_id: "inst-1".to_string(),
            success: true,
            duration: Duration::from_millis(50),
            version: "1.0.0".to_string(),
            attempts: 1,
            error: None,
        };

        assert!(result.success);
        assert!(result.error.is_none());
    }

    #[test]
    fn test_push_result_failure() {
        let result = PushResult {
            instance_id: "inst-1".to_string(),
            success: false,
            duration: Duration::from_millis(500),
            version: "1.0.0".to_string(),
            attempts: 3,
            error: Some("connection refused".to_string()),
        };

        assert!(!result.success);
        assert_eq!(result.attempts, 3);
        assert!(result.error.is_some());
    }
}
