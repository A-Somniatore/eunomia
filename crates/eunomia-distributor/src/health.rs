//! Health checking for Archimedes instances.
//!
//! This module provides health monitoring capabilities for tracking
//! the state of Archimedes instances.

use std::time::{Duration, Instant};

/// Health state of an instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthState {
    /// Unknown state (not yet checked).
    Unknown,

    /// Instance is healthy and operational.
    Healthy,

    /// Instance is degraded but still operational.
    Degraded,

    /// Instance is unhealthy.
    Unhealthy,

    /// Instance is unreachable.
    Unreachable,
}

impl HealthState {
    /// Returns true if the instance is operational.
    pub fn is_operational(&self) -> bool {
        matches!(self, Self::Healthy | Self::Degraded)
    }

    /// Returns a string representation for display.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Healthy => "healthy",
            Self::Degraded => "degraded",
            Self::Unhealthy => "unhealthy",
            Self::Unreachable => "unreachable",
        }
    }
}

impl std::fmt::Display for HealthState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Health check result.
#[derive(Debug, Clone)]
pub struct HealthCheck {
    /// Current health state.
    pub state: HealthState,

    /// When the check was performed.
    pub checked_at: Instant,

    /// Current policy version (if healthy).
    pub policy_version: Option<String>,

    /// Response time for the health check.
    pub response_time: Option<Duration>,

    /// Optional message with details.
    pub message: Option<String>,
}

impl HealthCheck {
    /// Creates a healthy result.
    pub fn healthy(policy_version: Option<String>, response_time: Duration) -> Self {
        Self {
            state: HealthState::Healthy,
            checked_at: Instant::now(),
            policy_version,
            response_time: Some(response_time),
            message: None,
        }
    }

    /// Creates an unhealthy result.
    pub fn unhealthy(message: impl Into<String>) -> Self {
        Self {
            state: HealthState::Unhealthy,
            checked_at: Instant::now(),
            policy_version: None,
            response_time: None,
            message: Some(message.into()),
        }
    }

    /// Creates a degraded result.
    pub fn degraded(message: impl Into<String>, policy_version: Option<String>) -> Self {
        Self {
            state: HealthState::Degraded,
            checked_at: Instant::now(),
            policy_version,
            response_time: None,
            message: Some(message.into()),
        }
    }

    /// Creates an unreachable result.
    pub fn unreachable(message: impl Into<String>) -> Self {
        Self {
            state: HealthState::Unreachable,
            checked_at: Instant::now(),
            policy_version: None,
            response_time: None,
            message: Some(message.into()),
        }
    }

    /// Creates an unknown result.
    pub fn unknown() -> Self {
        Self {
            state: HealthState::Unknown,
            checked_at: Instant::now(),
            policy_version: None,
            response_time: None,
            message: None,
        }
    }

    /// Returns how long ago the check was performed.
    pub fn age(&self) -> Duration {
        self.checked_at.elapsed()
    }
}

/// Configuration for health checking.
#[derive(Debug, Clone)]
pub struct HealthConfig {
    /// Interval between health checks.
    pub check_interval: Duration,

    /// Timeout for health check requests.
    pub timeout: Duration,

    /// Number of consecutive failures before marking unhealthy.
    pub unhealthy_threshold: u32,

    /// Number of consecutive successes before marking healthy.
    pub healthy_threshold: u32,

    /// Enable detailed health metrics.
    pub detailed_metrics: bool,
}

impl Default for HealthConfig {
    fn default() -> Self {
        Self {
            check_interval: Duration::from_secs(10),
            timeout: Duration::from_secs(5),
            unhealthy_threshold: 3,
            healthy_threshold: 2,
            detailed_metrics: true,
        }
    }
}

impl HealthConfig {
    /// Creates a configuration builder.
    pub fn builder() -> HealthConfigBuilder {
        HealthConfigBuilder::default()
    }
}

/// Builder for `HealthConfig`.
#[derive(Debug, Default)]
pub struct HealthConfigBuilder {
    check_interval: Option<Duration>,
    timeout: Option<Duration>,
    unhealthy_threshold: Option<u32>,
    healthy_threshold: Option<u32>,
    detailed_metrics: Option<bool>,
}

impl HealthConfigBuilder {
    /// Sets the check interval.
    pub fn check_interval(mut self, interval: Duration) -> Self {
        self.check_interval = Some(interval);
        self
    }

    /// Sets the timeout.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Sets the unhealthy threshold.
    pub fn unhealthy_threshold(mut self, threshold: u32) -> Self {
        self.unhealthy_threshold = Some(threshold);
        self
    }

    /// Sets the healthy threshold.
    pub fn healthy_threshold(mut self, threshold: u32) -> Self {
        self.healthy_threshold = Some(threshold);
        self
    }

    /// Enables or disables detailed metrics.
    pub fn detailed_metrics(mut self, enabled: bool) -> Self {
        self.detailed_metrics = Some(enabled);
        self
    }

    /// Builds the configuration.
    pub fn build(self) -> HealthConfig {
        let defaults = HealthConfig::default();
        HealthConfig {
            check_interval: self.check_interval.unwrap_or(defaults.check_interval),
            timeout: self.timeout.unwrap_or(defaults.timeout),
            unhealthy_threshold: self
                .unhealthy_threshold
                .unwrap_or(defaults.unhealthy_threshold),
            healthy_threshold: self.healthy_threshold.unwrap_or(defaults.healthy_threshold),
            detailed_metrics: self.detailed_metrics.unwrap_or(defaults.detailed_metrics),
        }
    }
}

/// Health check tracker for an instance.
#[derive(Debug)]
pub struct HealthTracker {
    /// Current state.
    pub state: HealthState,

    /// Last check result.
    pub last_check: Option<HealthCheck>,

    /// Consecutive success count.
    pub consecutive_successes: u32,

    /// Consecutive failure count.
    pub consecutive_failures: u32,

    /// Total checks performed.
    pub total_checks: u64,

    /// Configuration thresholds.
    config: HealthConfig,
}

impl HealthTracker {
    /// Creates a new health tracker.
    pub fn new(config: HealthConfig) -> Self {
        Self {
            state: HealthState::Unknown,
            last_check: None,
            consecutive_successes: 0,
            consecutive_failures: 0,
            total_checks: 0,
            config,
        }
    }

    /// Records a health check result.
    pub fn record(&mut self, check: HealthCheck) {
        self.total_checks += 1;

        match check.state {
            HealthState::Healthy => {
                self.consecutive_successes += 1;
                self.consecutive_failures = 0;

                if self.consecutive_successes >= self.config.healthy_threshold {
                    self.state = HealthState::Healthy;
                }
            }
            HealthState::Degraded => {
                self.consecutive_successes = 0;
                self.state = HealthState::Degraded;
            }
            HealthState::Unhealthy | HealthState::Unreachable => {
                self.consecutive_failures += 1;
                self.consecutive_successes = 0;

                if self.consecutive_failures >= self.config.unhealthy_threshold {
                    self.state = check.state;
                }
            }
            HealthState::Unknown => {
                // Don't change counters
            }
        }

        self.last_check = Some(check);
    }

    /// Returns true if a check is due.
    pub fn is_check_due(&self) -> bool {
        self.last_check
            .as_ref()
            .is_none_or(|check| check.age() >= self.config.check_interval)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_state_is_operational() {
        assert!(HealthState::Healthy.is_operational());
        assert!(HealthState::Degraded.is_operational());
        assert!(!HealthState::Unhealthy.is_operational());
        assert!(!HealthState::Unreachable.is_operational());
        assert!(!HealthState::Unknown.is_operational());
    }

    #[test]
    fn test_health_state_display() {
        assert_eq!(HealthState::Healthy.to_string(), "healthy");
        assert_eq!(HealthState::Unhealthy.as_str(), "unhealthy");
    }

    #[test]
    fn test_health_check_healthy() {
        let check = HealthCheck::healthy(Some("1.0.0".to_string()), Duration::from_millis(50));
        assert_eq!(check.state, HealthState::Healthy);
        assert_eq!(check.policy_version, Some("1.0.0".to_string()));
        assert!(check.response_time.is_some());
    }

    #[test]
    fn test_health_check_unhealthy() {
        let check = HealthCheck::unhealthy("connection failed");
        assert_eq!(check.state, HealthState::Unhealthy);
        assert_eq!(check.message, Some("connection failed".to_string()));
    }

    #[test]
    fn test_health_check_unreachable() {
        let check = HealthCheck::unreachable("timeout");
        assert_eq!(check.state, HealthState::Unreachable);
    }

    #[test]
    fn test_health_config_default() {
        let config = HealthConfig::default();
        assert_eq!(config.check_interval, Duration::from_secs(10));
        assert_eq!(config.timeout, Duration::from_secs(5));
        assert_eq!(config.unhealthy_threshold, 3);
        assert_eq!(config.healthy_threshold, 2);
    }

    #[test]
    fn test_health_config_builder() {
        let config = HealthConfig::builder()
            .check_interval(Duration::from_secs(30))
            .timeout(Duration::from_secs(10))
            .unhealthy_threshold(5)
            .healthy_threshold(3)
            .build();

        assert_eq!(config.check_interval, Duration::from_secs(30));
        assert_eq!(config.timeout, Duration::from_secs(10));
        assert_eq!(config.unhealthy_threshold, 5);
        assert_eq!(config.healthy_threshold, 3);
    }

    #[test]
    fn test_health_tracker_new() {
        let tracker = HealthTracker::new(HealthConfig::default());
        assert_eq!(tracker.state, HealthState::Unknown);
        assert!(tracker.last_check.is_none());
        assert_eq!(tracker.consecutive_successes, 0);
        assert_eq!(tracker.consecutive_failures, 0);
    }

    #[test]
    fn test_health_tracker_record_healthy() {
        let config = HealthConfig {
            healthy_threshold: 2,
            ..HealthConfig::default()
        };
        let mut tracker = HealthTracker::new(config);

        // First healthy check - not yet healthy (threshold = 2)
        let check = HealthCheck::healthy(None, Duration::from_millis(10));
        tracker.record(check);
        assert_eq!(tracker.consecutive_successes, 1);
        assert_eq!(tracker.state, HealthState::Unknown); // Not yet at threshold

        // Second healthy check - now healthy
        let check = HealthCheck::healthy(None, Duration::from_millis(10));
        tracker.record(check);
        assert_eq!(tracker.consecutive_successes, 2);
        assert_eq!(tracker.state, HealthState::Healthy);
    }

    #[test]
    fn test_health_tracker_record_unhealthy() {
        let config = HealthConfig {
            unhealthy_threshold: 2,
            ..HealthConfig::default()
        };
        let mut tracker = HealthTracker::new(config);

        // First failure
        let check = HealthCheck::unhealthy("error");
        tracker.record(check);
        assert_eq!(tracker.consecutive_failures, 1);
        assert_eq!(tracker.state, HealthState::Unknown);

        // Second failure - now unhealthy
        let check = HealthCheck::unhealthy("error");
        tracker.record(check);
        assert_eq!(tracker.consecutive_failures, 2);
        assert_eq!(tracker.state, HealthState::Unhealthy);
    }

    #[test]
    fn test_health_tracker_resets_on_state_change() {
        let config = HealthConfig {
            healthy_threshold: 1,
            unhealthy_threshold: 1,
            ..HealthConfig::default()
        };
        let mut tracker = HealthTracker::new(config);

        // Become healthy
        tracker.record(HealthCheck::healthy(None, Duration::from_millis(10)));
        assert_eq!(tracker.state, HealthState::Healthy);
        assert_eq!(tracker.consecutive_successes, 1);

        // Fail - should reset success counter
        tracker.record(HealthCheck::unhealthy("error"));
        assert_eq!(tracker.consecutive_successes, 0);
        assert_eq!(tracker.consecutive_failures, 1);
    }

    #[test]
    fn test_health_tracker_is_check_due() {
        let config = HealthConfig {
            check_interval: Duration::from_millis(10),
            ..HealthConfig::default()
        };
        let tracker = HealthTracker::new(config);

        // No checks yet - check is due
        assert!(tracker.is_check_due());
    }
}
