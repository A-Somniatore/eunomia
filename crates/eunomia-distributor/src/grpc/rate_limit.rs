//! Rate limiting middleware for gRPC endpoints.
//!
//! This module provides configurable rate limiting for the Control Plane gRPC server
//! to protect against abuse and ensure fair resource allocation.
//!
//! # Example
//!
//! ```rust,ignore
//! use eunomia_distributor::grpc::rate_limit::{RateLimitConfig, RateLimitLayer};
//!
//! let config = RateLimitConfig::default()
//!     .with_requests_per_second(100)
//!     .with_burst_size(50);
//!
//! let layer = RateLimitLayer::new(config);
//! ```

#![allow(clippy::result_large_err)] // Status is from tonic, can't change its size

use std::num::NonZeroU32;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

use parking_lot::Mutex;
use tonic::{Code, Status};

/// Configuration for rate limiting.
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Maximum requests per second allowed.
    pub requests_per_second: NonZeroU32,
    /// Maximum burst size (number of requests that can be made immediately).
    pub burst_size: NonZeroU32,
    /// Whether rate limiting is enabled.
    pub enabled: bool,
    /// Custom message for rate limit exceeded errors.
    pub exceeded_message: String,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_second: NonZeroU32::new(100).expect("100 > 0"),
            burst_size: NonZeroU32::new(50).expect("50 > 0"),
            enabled: true,
            exceeded_message: "Rate limit exceeded. Please retry later.".to_string(),
        }
    }
}

impl RateLimitConfig {
    /// Create a new rate limit config with the given requests per second.
    ///
    /// # Panics
    ///
    /// Panics if `rps` is 0.
    pub fn new(rps: u32) -> Self {
        Self {
            requests_per_second: NonZeroU32::new(rps).expect("requests per second must be > 0"),
            ..Default::default()
        }
    }

    /// Set the requests per second limit.
    ///
    /// # Panics
    ///
    /// Panics if `rps` is 0.
    #[must_use]
    pub fn with_requests_per_second(mut self, rps: u32) -> Self {
        self.requests_per_second = NonZeroU32::new(rps).expect("requests per second must be > 0");
        self
    }

    /// Set the burst size.
    ///
    /// # Panics
    ///
    /// Panics if `size` is 0.
    #[must_use]
    pub fn with_burst_size(mut self, size: u32) -> Self {
        self.burst_size = NonZeroU32::new(size).expect("burst size must be > 0");
        self
    }

    /// Enable or disable rate limiting.
    #[must_use]
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Set custom exceeded message.
    #[must_use]
    pub fn with_exceeded_message(mut self, message: impl Into<String>) -> Self {
        self.exceeded_message = message.into();
        self
    }

    /// Create a disabled rate limit config.
    #[must_use]
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            ..Default::default()
        }
    }

    /// Check if rate limiting is enabled.
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}

/// Token bucket rate limiter.
///
/// Uses the token bucket algorithm to rate limit requests.
/// Tokens are added at a fixed rate up to the burst size.
#[derive(Debug)]
pub struct TokenBucket {
    /// Current number of tokens available.
    tokens: Mutex<f64>,
    /// Last time tokens were updated.
    last_update: Mutex<Instant>,
    /// Configuration for the rate limiter.
    config: RateLimitConfig,
    /// Statistics: total requests received.
    total_requests: AtomicU64,
    /// Statistics: requests that were rate limited.
    limited_requests: AtomicU64,
}

impl TokenBucket {
    /// Create a new token bucket rate limiter.
    #[must_use]
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            tokens: Mutex::new(f64::from(config.burst_size.get())),
            last_update: Mutex::new(Instant::now()),
            config,
            total_requests: AtomicU64::new(0),
            limited_requests: AtomicU64::new(0),
        }
    }

    /// Try to acquire a token for a request.
    ///
    /// Returns `Ok(())` if the request is allowed, or `Err(Status)` if rate limited.
    pub fn try_acquire(&self) -> Result<(), Status> {
        self.total_requests.fetch_add(1, Ordering::Relaxed);

        if !self.config.enabled {
            return Ok(());
        }

        let mut tokens = self.tokens.lock();
        let mut last_update = self.last_update.lock();

        // Calculate tokens to add based on elapsed time
        let now = Instant::now();
        let elapsed = now.duration_since(*last_update);
        let tokens_to_add =
            elapsed.as_secs_f64() * f64::from(self.config.requests_per_second.get());

        // Add tokens up to burst size
        *tokens = (*tokens + tokens_to_add).min(f64::from(self.config.burst_size.get()));
        *last_update = now;

        // Try to consume a token
        if *tokens >= 1.0 {
            *tokens -= 1.0;
            Ok(())
        } else {
            self.limited_requests.fetch_add(1, Ordering::Relaxed);
            Err(Status::new(
                Code::ResourceExhausted,
                &self.config.exceeded_message,
            ))
        }
    }

    /// Get the current number of available tokens.
    #[must_use]
    pub fn available_tokens(&self) -> f64 {
        let mut tokens = self.tokens.lock();
        let mut last_update = self.last_update.lock();

        let now = Instant::now();
        let elapsed = now.duration_since(*last_update);
        let tokens_to_add =
            elapsed.as_secs_f64() * f64::from(self.config.requests_per_second.get());

        *tokens = (*tokens + tokens_to_add).min(f64::from(self.config.burst_size.get()));
        *last_update = now;

        *tokens
    }

    /// Get the rate limit configuration.
    #[must_use]
    pub fn config(&self) -> &RateLimitConfig {
        &self.config
    }

    /// Get total requests count.
    #[must_use]
    pub fn total_requests(&self) -> u64 {
        self.total_requests.load(Ordering::Relaxed)
    }

    /// Get limited requests count.
    #[must_use]
    pub fn limited_requests(&self) -> u64 {
        self.limited_requests.load(Ordering::Relaxed)
    }

    /// Reset statistics.
    pub fn reset_stats(&self) {
        self.total_requests.store(0, Ordering::Relaxed);
        self.limited_requests.store(0, Ordering::Relaxed);
    }
}

/// Shared rate limiter for use across gRPC services.
pub type SharedRateLimiter = Arc<TokenBucket>;

/// Create a new shared rate limiter.
#[must_use]
pub fn create_rate_limiter(config: RateLimitConfig) -> SharedRateLimiter {
    Arc::new(TokenBucket::new(config))
}

/// Per-endpoint rate limit configuration.
#[derive(Debug, Clone)]
pub struct EndpointRateLimits {
    /// Default rate limit for endpoints without specific config.
    pub default: RateLimitConfig,
    /// Rate limit for `DeployPolicy` endpoint (higher limit for critical operations).
    pub deploy_policy: Option<RateLimitConfig>,
    /// Rate limit for `RollbackPolicy` endpoint.
    pub rollback_policy: Option<RateLimitConfig>,
    /// Rate limit for `GetPolicyStatus` endpoint (read-heavy, can be higher).
    pub get_status: Option<RateLimitConfig>,
    /// Rate limit for `ListInstances` endpoint.
    pub list_instances: Option<RateLimitConfig>,
    /// Rate limit for `HealthCheck` endpoint (should be high to not affect monitoring).
    pub health_check: Option<RateLimitConfig>,
}

impl Default for EndpointRateLimits {
    fn default() -> Self {
        Self {
            default: RateLimitConfig::default(),
            deploy_policy: Some(RateLimitConfig::new(50).with_burst_size(25)),
            rollback_policy: Some(RateLimitConfig::new(50).with_burst_size(25)),
            get_status: Some(RateLimitConfig::new(200).with_burst_size(100)),
            list_instances: Some(RateLimitConfig::new(200).with_burst_size(100)),
            health_check: Some(RateLimitConfig::new(1000).with_burst_size(500)),
        }
    }
}

impl EndpointRateLimits {
    /// Create a new endpoint rate limits configuration.
    pub fn new(default: RateLimitConfig) -> Self {
        Self {
            default,
            deploy_policy: None,
            rollback_policy: None,
            get_status: None,
            list_instances: None,
            health_check: None,
        }
    }

    /// Set the rate limit for `DeployPolicy` endpoint.
    #[must_use]
    pub fn with_deploy_policy(mut self, config: RateLimitConfig) -> Self {
        self.deploy_policy = Some(config);
        self
    }

    /// Set the rate limit for `RollbackPolicy` endpoint.
    #[must_use]
    pub fn with_rollback_policy(mut self, config: RateLimitConfig) -> Self {
        self.rollback_policy = Some(config);
        self
    }

    /// Set the rate limit for `GetPolicyStatus` endpoint.
    #[must_use]
    pub fn with_get_status(mut self, config: RateLimitConfig) -> Self {
        self.get_status = Some(config);
        self
    }

    /// Set the rate limit for `ListInstances` endpoint.
    #[must_use]
    pub fn with_list_instances(mut self, config: RateLimitConfig) -> Self {
        self.list_instances = Some(config);
        self
    }

    /// Set the rate limit for `HealthCheck` endpoint.
    #[must_use]
    pub fn with_health_check(mut self, config: RateLimitConfig) -> Self {
        self.health_check = Some(config);
        self
    }

    /// Get rate limit config for a specific endpoint.
    #[must_use]
    pub fn get_config(&self, endpoint: &str) -> &RateLimitConfig {
        match endpoint {
            "DeployPolicy" => self.deploy_policy.as_ref().unwrap_or(&self.default),
            "RollbackPolicy" => self.rollback_policy.as_ref().unwrap_or(&self.default),
            "GetPolicyStatus" => self.get_status.as_ref().unwrap_or(&self.default),
            "ListInstances" => self.list_instances.as_ref().unwrap_or(&self.default),
            "HealthCheck" => self.health_check.as_ref().unwrap_or(&self.default),
            _ => &self.default,
        }
    }

    /// Disable all rate limiting.
    #[must_use]
    pub fn disabled() -> Self {
        Self {
            default: RateLimitConfig::disabled(),
            deploy_policy: Some(RateLimitConfig::disabled()),
            rollback_policy: Some(RateLimitConfig::disabled()),
            get_status: Some(RateLimitConfig::disabled()),
            list_instances: Some(RateLimitConfig::disabled()),
            health_check: Some(RateLimitConfig::disabled()),
        }
    }
}

/// Rate limiter registry for managing per-endpoint limiters.
#[derive(Debug)]
pub struct RateLimiterRegistry {
    /// Default rate limiter.
    default: SharedRateLimiter,
    /// Deploy policy rate limiter.
    deploy_policy: Option<SharedRateLimiter>,
    /// Rollback policy rate limiter.
    rollback_policy: Option<SharedRateLimiter>,
    /// Get status rate limiter.
    get_status: Option<SharedRateLimiter>,
    /// List instances rate limiter.
    list_instances: Option<SharedRateLimiter>,
    /// Health check rate limiter.
    health_check: Option<SharedRateLimiter>,
}

impl RateLimiterRegistry {
    /// Create a new registry from endpoint rate limits.
    #[must_use]
    pub fn new(config: EndpointRateLimits) -> Self {
        Self {
            default: create_rate_limiter(config.default),
            deploy_policy: config.deploy_policy.map(create_rate_limiter),
            rollback_policy: config.rollback_policy.map(create_rate_limiter),
            get_status: config.get_status.map(create_rate_limiter),
            list_instances: config.list_instances.map(create_rate_limiter),
            health_check: config.health_check.map(create_rate_limiter),
        }
    }

    /// Get the rate limiter for a specific endpoint.
    #[must_use]
    pub fn get(&self, endpoint: &str) -> &SharedRateLimiter {
        match endpoint {
            "DeployPolicy" => self.deploy_policy.as_ref().unwrap_or(&self.default),
            "RollbackPolicy" => self.rollback_policy.as_ref().unwrap_or(&self.default),
            "GetPolicyStatus" => self.get_status.as_ref().unwrap_or(&self.default),
            "ListInstances" => self.list_instances.as_ref().unwrap_or(&self.default),
            "HealthCheck" => self.health_check.as_ref().unwrap_or(&self.default),
            _ => &self.default,
        }
    }

    /// Check rate limit for an endpoint.
    ///
    /// Returns `Ok(())` if allowed, `Err(Status)` if rate limited.
    pub fn check(&self, endpoint: &str) -> Result<(), Status> {
        self.get(endpoint).try_acquire()
    }

    /// Get statistics for all endpoints.
    #[must_use]
    pub fn stats(&self) -> RateLimitStats {
        RateLimitStats {
            default: EndpointStats::from_limiter(&self.default),
            deploy_policy: self
                .deploy_policy
                .as_ref()
                .map(|l| EndpointStats::from_limiter(l)),
            rollback_policy: self
                .rollback_policy
                .as_ref()
                .map(|l| EndpointStats::from_limiter(l)),
            get_status: self
                .get_status
                .as_ref()
                .map(|l| EndpointStats::from_limiter(l)),
            list_instances: self
                .list_instances
                .as_ref()
                .map(|l| EndpointStats::from_limiter(l)),
            health_check: self
                .health_check
                .as_ref()
                .map(|l| EndpointStats::from_limiter(l)),
        }
    }
}

/// Statistics for a single endpoint.
#[derive(Debug, Clone)]
pub struct EndpointStats {
    /// Total requests received.
    pub total_requests: u64,
    /// Requests that were rate limited.
    pub limited_requests: u64,
    /// Current available tokens.
    pub available_tokens: f64,
}

impl EndpointStats {
    fn from_limiter(limiter: &TokenBucket) -> Self {
        Self {
            total_requests: limiter.total_requests(),
            limited_requests: limiter.limited_requests(),
            available_tokens: limiter.available_tokens(),
        }
    }
}

/// Statistics for all rate limiters.
#[derive(Debug, Clone)]
pub struct RateLimitStats {
    /// Default endpoint stats.
    pub default: EndpointStats,
    /// Deploy policy stats.
    pub deploy_policy: Option<EndpointStats>,
    /// Rollback policy stats.
    pub rollback_policy: Option<EndpointStats>,
    /// Get status stats.
    pub get_status: Option<EndpointStats>,
    /// List instances stats.
    pub list_instances: Option<EndpointStats>,
    /// Health check stats.
    pub health_check: Option<EndpointStats>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_rate_limit_config_default() {
        let config = RateLimitConfig::default();
        assert_eq!(config.requests_per_second.get(), 100);
        assert_eq!(config.burst_size.get(), 50);
        assert!(config.enabled);
    }

    #[test]
    fn test_rate_limit_config_builder() {
        let config = RateLimitConfig::new(200)
            .with_burst_size(100)
            .with_exceeded_message("Custom message");

        assert_eq!(config.requests_per_second.get(), 200);
        assert_eq!(config.burst_size.get(), 100);
        assert_eq!(config.exceeded_message, "Custom message");
    }

    #[test]
    fn test_rate_limit_config_disabled() {
        let config = RateLimitConfig::disabled();
        assert!(!config.enabled);
        assert!(!config.is_enabled());
    }

    #[test]
    fn test_token_bucket_allows_within_limit() {
        let config = RateLimitConfig::new(100).with_burst_size(10);
        let bucket = TokenBucket::new(config);

        // Should allow up to burst size requests immediately
        for _ in 0..10 {
            assert!(bucket.try_acquire().is_ok());
        }
    }

    #[test]
    fn test_token_bucket_rejects_over_limit() {
        let config = RateLimitConfig::new(100).with_burst_size(5);
        let bucket = TokenBucket::new(config);

        // Consume all tokens
        for _ in 0..5 {
            let _ = bucket.try_acquire();
        }

        // Next request should be rejected
        let result = bucket.try_acquire();
        assert!(result.is_err());

        let status = result.unwrap_err();
        assert_eq!(status.code(), Code::ResourceExhausted);
    }

    #[test]
    fn test_token_bucket_replenishes() {
        let config = RateLimitConfig::new(1000).with_burst_size(1);
        let bucket = TokenBucket::new(config);

        // Consume the token
        assert!(bucket.try_acquire().is_ok());
        assert!(bucket.try_acquire().is_err());

        // Wait for replenishment (1000 rps = 1 token per ms)
        thread::sleep(Duration::from_millis(5));

        // Should be able to acquire again
        assert!(bucket.try_acquire().is_ok());
    }

    #[test]
    fn test_token_bucket_disabled() {
        let config = RateLimitConfig::disabled();
        let bucket = TokenBucket::new(config);

        // Should always allow when disabled
        for _ in 0..1000 {
            assert!(bucket.try_acquire().is_ok());
        }
    }

    #[test]
    fn test_token_bucket_stats() {
        let config = RateLimitConfig::new(100).with_burst_size(5);
        let bucket = TokenBucket::new(config);

        for _ in 0..7 {
            let _ = bucket.try_acquire();
        }

        assert_eq!(bucket.total_requests(), 7);
        assert_eq!(bucket.limited_requests(), 2); // 7 - 5 = 2 rejected

        bucket.reset_stats();
        assert_eq!(bucket.total_requests(), 0);
        assert_eq!(bucket.limited_requests(), 0);
    }

    #[test]
    fn test_endpoint_rate_limits_default() {
        let limits = EndpointRateLimits::default();

        // Default limits
        assert_eq!(limits.default.requests_per_second.get(), 100);

        // Deploy policy has specific limits
        let deploy = limits.deploy_policy.as_ref().unwrap();
        assert_eq!(deploy.requests_per_second.get(), 50);

        // Health check has higher limits
        let health = limits.health_check.as_ref().unwrap();
        assert_eq!(health.requests_per_second.get(), 1000);
    }

    #[test]
    fn test_endpoint_rate_limits_get_config() {
        let limits = EndpointRateLimits::default();

        let deploy_config = limits.get_config("DeployPolicy");
        assert_eq!(deploy_config.requests_per_second.get(), 50);

        let unknown_config = limits.get_config("UnknownEndpoint");
        assert_eq!(unknown_config.requests_per_second.get(), 100); // Falls back to default
    }

    #[test]
    fn test_rate_limiter_registry() {
        let registry = RateLimiterRegistry::new(EndpointRateLimits::default());

        // Check DeployPolicy limit
        let deploy_limiter = registry.get("DeployPolicy");
        assert_eq!(deploy_limiter.config().requests_per_second.get(), 50);

        // Check default limiter for unknown endpoint
        let unknown_limiter = registry.get("Unknown");
        assert_eq!(unknown_limiter.config().requests_per_second.get(), 100);
    }

    #[test]
    fn test_rate_limiter_registry_check() {
        let registry = RateLimiterRegistry::new(
            EndpointRateLimits::new(RateLimitConfig::new(100).with_burst_size(2))
                .with_deploy_policy(RateLimitConfig::new(50).with_burst_size(1)),
        );

        // First deploy request should succeed
        assert!(registry.check("DeployPolicy").is_ok());

        // Second should fail (burst size = 1)
        assert!(registry.check("DeployPolicy").is_err());
    }

    #[test]
    fn test_rate_limit_stats() {
        let registry = RateLimiterRegistry::new(EndpointRateLimits::default());

        // Make some requests
        for _ in 0..5 {
            let _ = registry.check("DeployPolicy");
        }

        let stats = registry.stats();
        assert!(stats.deploy_policy.is_some());

        let deploy_stats = stats.deploy_policy.unwrap();
        assert_eq!(deploy_stats.total_requests, 5);
    }

    #[test]
    fn test_shared_rate_limiter() {
        let limiter = create_rate_limiter(RateLimitConfig::new(100).with_burst_size(10));
        let limiter2 = Arc::clone(&limiter);

        // Both references share the same state
        assert!(limiter.try_acquire().is_ok());
        assert!(limiter2.try_acquire().is_ok());
        assert_eq!(limiter.total_requests(), 2);
    }

    #[test]
    fn test_custom_exceeded_message() {
        // Note: burst_size of 0 would panic, so we use 1
        let config = RateLimitConfig::new(100)
            .with_burst_size(1)
            .with_exceeded_message("Too many requests!");

        let bucket = TokenBucket::new(config);
        let _ = bucket.try_acquire(); // Consume the token

        let result = bucket.try_acquire();
        assert!(result.is_err());

        let status = result.unwrap_err();
        assert!(status.message().contains("Too many requests!"));
    }

    #[test]
    fn test_endpoint_rate_limits_disabled() {
        let limits = EndpointRateLimits::disabled();

        assert!(!limits.default.is_enabled());
        assert!(!limits.deploy_policy.as_ref().unwrap().is_enabled());
        assert!(!limits.health_check.as_ref().unwrap().is_enabled());
    }

    #[test]
    fn test_available_tokens() {
        let config = RateLimitConfig::new(100).with_burst_size(10);
        let bucket = TokenBucket::new(config);

        // Initially should have burst_size tokens
        let initial = bucket.available_tokens();
        assert!((initial - 10.0).abs() < 0.1);

        // After consuming some tokens
        bucket.try_acquire().unwrap();
        bucket.try_acquire().unwrap();

        let after = bucket.available_tokens();
        assert!((after - 8.0).abs() < 0.1);
    }
}
