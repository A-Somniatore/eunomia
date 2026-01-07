//! Rollback controller for policy deployments.
//!
//! This module provides the rollback functionality for reverting policy
//! deployments to previous versions, including:
//!
//! - Manual rollback triggers
//! - Automatic rollback on health failures
//! - Rollback history tracking
//! - Configurable rollback strategies
//!
//! # Example
//!
//! ```rust,ignore
//! use eunomia_distributor::rollback::{RollbackController, RollbackConfig, RollbackTrigger};
//!
//! let config = RollbackConfig::default();
//! let controller = RollbackController::new(config);
//!
//! // Manual rollback
//! let result = controller.trigger(
//!     RollbackTrigger::manual("users-service", "1.0.0", "Performance regression")
//! ).await?;
//!
//! // Auto-rollback is triggered by health monitor
//! ```

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use eunomia_audit::{AuditLogger, DistributionEvent};
use serde::{Deserialize, Serialize};

use crate::error::{DistributorError, Result};

/// Configuration for the rollback controller.
#[derive(Debug, Clone)]
pub struct RollbackConfig {
    /// Enable automatic rollback on health failures.
    pub auto_rollback_enabled: bool,

    /// Number of consecutive health check failures before triggering auto-rollback.
    pub failure_threshold: u32,

    /// Time window for counting failures.
    pub failure_window: Duration,

    /// Cooldown period between auto-rollbacks.
    pub cooldown_period: Duration,

    /// Maximum rollback history entries to keep per service.
    pub max_history_entries: usize,
}

impl Default for RollbackConfig {
    fn default() -> Self {
        Self {
            auto_rollback_enabled: true,
            failure_threshold: 3,
            failure_window: Duration::from_secs(60),
            cooldown_period: Duration::from_secs(300),
            max_history_entries: 10,
        }
    }
}

impl RollbackConfig {
    /// Creates a new rollback configuration builder.
    pub fn builder() -> RollbackConfigBuilder {
        RollbackConfigBuilder::default()
    }

    /// Disables automatic rollback.
    #[must_use]
    pub const fn without_auto_rollback(mut self) -> Self {
        self.auto_rollback_enabled = false;
        self
    }
}

/// Builder for `RollbackConfig`.
#[derive(Debug, Default)]
pub struct RollbackConfigBuilder {
    auto_rollback_enabled: Option<bool>,
    failure_threshold: Option<u32>,
    failure_window: Option<Duration>,
    cooldown_period: Option<Duration>,
    max_history_entries: Option<usize>,
}

impl RollbackConfigBuilder {
    /// Enables or disables automatic rollback.
    #[must_use]
    pub const fn auto_rollback(mut self, enabled: bool) -> Self {
        self.auto_rollback_enabled = Some(enabled);
        self
    }

    /// Sets the failure threshold for auto-rollback.
    #[must_use]
    pub const fn failure_threshold(mut self, threshold: u32) -> Self {
        self.failure_threshold = Some(threshold);
        self
    }

    /// Sets the failure window duration.
    #[must_use]
    pub const fn failure_window(mut self, window: Duration) -> Self {
        self.failure_window = Some(window);
        self
    }

    /// Sets the cooldown period between auto-rollbacks.
    #[must_use]
    pub const fn cooldown_period(mut self, period: Duration) -> Self {
        self.cooldown_period = Some(period);
        self
    }

    /// Builds the configuration.
    #[must_use]
    pub fn build(self) -> RollbackConfig {
        let defaults = RollbackConfig::default();
        RollbackConfig {
            auto_rollback_enabled: self
                .auto_rollback_enabled
                .unwrap_or(defaults.auto_rollback_enabled),
            failure_threshold: self
                .failure_threshold
                .unwrap_or(defaults.failure_threshold),
            failure_window: self.failure_window.unwrap_or(defaults.failure_window),
            cooldown_period: self.cooldown_period.unwrap_or(defaults.cooldown_period),
            max_history_entries: self
                .max_history_entries
                .unwrap_or(defaults.max_history_entries),
        }
    }
}

/// Trigger for initiating a rollback.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackTrigger {
    /// Service to rollback.
    pub service: String,

    /// Target version to rollback to.
    pub target_version: String,

    /// Reason for the rollback.
    pub reason: String,

    /// Whether this is an automatic rollback.
    pub is_automatic: bool,

    /// Specific instance IDs to rollback (None = all instances).
    pub target_instances: Option<Vec<String>>,

    /// Force rollback even if pre-checks fail.
    pub force: bool,
}

impl RollbackTrigger {
    /// Creates a manual rollback trigger.
    #[must_use]
    pub fn manual(service: &str, target_version: &str, reason: &str) -> Self {
        Self {
            service: service.to_string(),
            target_version: target_version.to_string(),
            reason: reason.to_string(),
            is_automatic: false,
            target_instances: None,
            force: false,
        }
    }

    /// Creates an automatic rollback trigger.
    #[must_use]
    pub fn automatic(service: &str, target_version: &str, reason: &str) -> Self {
        Self {
            service: service.to_string(),
            target_version: target_version.to_string(),
            reason: reason.to_string(),
            is_automatic: true,
            target_instances: None,
            force: false,
        }
    }

    /// Sets specific instances to target.
    #[must_use]
    pub fn with_instances(mut self, instances: Vec<String>) -> Self {
        self.target_instances = Some(instances);
        self
    }

    /// Forces the rollback even if pre-checks fail.
    #[must_use]
    pub const fn with_force(mut self) -> Self {
        self.force = true;
        self
    }
}

/// Result of a rollback operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackResult {
    /// Unique rollback ID.
    pub rollback_id: String,

    /// Service that was rolled back.
    pub service: String,

    /// Version rolled back from.
    pub from_version: String,

    /// Version rolled back to.
    pub to_version: String,

    /// Whether the rollback succeeded.
    pub success: bool,

    /// Number of instances rolled back.
    pub instances_rolled_back: usize,

    /// Number of instances that failed to rollback.
    pub instances_failed: usize,

    /// Time taken for rollback.
    pub duration: Duration,

    /// Reason for the rollback.
    pub reason: String,

    /// Whether this was an automatic rollback.
    pub is_automatic: bool,

    /// Timestamp of the rollback.
    pub timestamp: DateTime<Utc>,

    /// Error message if rollback failed.
    pub error: Option<String>,
}

impl RollbackResult {
    /// Creates a successful rollback result.
    #[must_use]
    pub fn success(
        rollback_id: &str,
        service: &str,
        from_version: &str,
        to_version: &str,
        instances_rolled_back: usize,
        duration: Duration,
        reason: &str,
        is_automatic: bool,
    ) -> Self {
        Self {
            rollback_id: rollback_id.to_string(),
            service: service.to_string(),
            from_version: from_version.to_string(),
            to_version: to_version.to_string(),
            success: true,
            instances_rolled_back,
            instances_failed: 0,
            duration,
            reason: reason.to_string(),
            is_automatic,
            timestamp: Utc::now(),
            error: None,
        }
    }

    /// Creates a failed rollback result.
    #[must_use]
    pub fn failure(
        rollback_id: &str,
        service: &str,
        from_version: &str,
        to_version: &str,
        instances_rolled_back: usize,
        instances_failed: usize,
        duration: Duration,
        reason: &str,
        is_automatic: bool,
        error: &str,
    ) -> Self {
        Self {
            rollback_id: rollback_id.to_string(),
            service: service.to_string(),
            from_version: from_version.to_string(),
            to_version: to_version.to_string(),
            success: false,
            instances_rolled_back,
            instances_failed,
            duration,
            reason: reason.to_string(),
            is_automatic,
            timestamp: Utc::now(),
            error: Some(error.to_string()),
        }
    }
}

/// Health failure tracking for auto-rollback decisions.
#[derive(Debug, Clone)]
struct FailureTracker {
    /// Recent failure timestamps.
    failures: Vec<Instant>,

    /// Last auto-rollback timestamp.
    last_auto_rollback: Option<Instant>,
}

impl FailureTracker {
    fn new() -> Self {
        Self {
            failures: Vec::new(),
            last_auto_rollback: None,
        }
    }

    fn record_failure(&mut self) {
        self.failures.push(Instant::now());
    }

    fn failures_in_window(&self, window: Duration) -> usize {
        let cutoff = Instant::now() - window;
        self.failures.iter().filter(|&&t| t > cutoff).count()
    }

    fn can_auto_rollback(&self, cooldown: Duration) -> bool {
        match self.last_auto_rollback {
            Some(last) => last.elapsed() > cooldown,
            None => true,
        }
    }

    fn mark_auto_rollback(&mut self) {
        self.last_auto_rollback = Some(Instant::now());
        self.failures.clear();
    }

    fn clear(&mut self) {
        self.failures.clear();
    }
}

/// Version history entry for a service.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionHistory {
    /// Version string.
    pub version: String,

    /// Deployment timestamp.
    pub deployed_at: DateTime<Utc>,

    /// Whether this version is known to be healthy.
    pub is_healthy: bool,

    /// Deployment ID that deployed this version.
    pub deployment_id: String,
}

/// Internal state for the rollback controller.
struct RollbackState {
    /// Version history per service.
    version_history: HashMap<String, Vec<VersionHistory>>,

    /// Current version per service.
    current_versions: HashMap<String, String>,

    /// Failure tracking per service.
    failure_trackers: HashMap<String, FailureTracker>,

    /// Rollback history.
    rollback_history: Vec<RollbackResult>,
}

impl RollbackState {
    fn new() -> Self {
        Self {
            version_history: HashMap::new(),
            current_versions: HashMap::new(),
            failure_trackers: HashMap::new(),
            rollback_history: Vec::new(),
        }
    }
}

/// Controller for managing rollbacks.
///
/// Handles both manual and automatic rollback triggers,
/// tracks version history, and enforces cooldown periods.
pub struct RollbackController {
    config: RollbackConfig,
    state: Arc<RwLock<RollbackState>>,
    audit_logger: Option<Arc<AuditLogger>>,
}

impl RollbackController {
    /// Creates a new rollback controller.
    pub fn new(config: RollbackConfig) -> Self {
        Self {
            config,
            state: Arc::new(RwLock::new(RollbackState::new())),
            audit_logger: None,
        }
    }

    /// Creates a new rollback controller with an audit logger.
    pub fn with_audit_logger(config: RollbackConfig, logger: Arc<AuditLogger>) -> Self {
        Self {
            config,
            state: Arc::new(RwLock::new(RollbackState::new())),
            audit_logger: Some(logger),
        }
    }

    /// Sets the audit logger for rollback event logging.
    pub fn set_audit_logger(&mut self, logger: Arc<AuditLogger>) {
        self.audit_logger = Some(logger);
    }

    /// Logs a rollback started event.
    fn log_rollback_started(&self, service: &str, from_version: &str, to_version: &str) {
        if let Some(logger) = &self.audit_logger {
            let event = DistributionEvent::rollback_started(service, from_version, to_version);
            if let Err(e) = logger.log(&event) {
                tracing::warn!(
                    error = %e,
                    service = %service,
                    "failed to log rollback started event"
                );
            }
        }
    }

    /// Logs a rollback completed event.
    fn log_rollback_completed(&self, service: &str, version: &str, success: bool) {
        if let Some(logger) = &self.audit_logger {
            let event = DistributionEvent::rollback_completed(service, version, success);
            if let Err(e) = logger.log(&event) {
                tracing::warn!(
                    error = %e,
                    service = %service,
                    "failed to log rollback completed event"
                );
            }
        }
    }

    /// Records a successful deployment for version tracking.
    pub fn record_deployment(
        &self,
        service: &str,
        version: &str,
        deployment_id: &str,
    ) {
        let mut state = self.state.write().unwrap();

        // Update current version
        state
            .current_versions
            .insert(service.to_string(), version.to_string());

        // Add to version history
        let history = state
            .version_history
            .entry(service.to_string())
            .or_default();

        history.push(VersionHistory {
            version: version.to_string(),
            deployed_at: Utc::now(),
            is_healthy: true, // Assume healthy until proven otherwise
            deployment_id: deployment_id.to_string(),
        });

        // Trim history if needed
        while history.len() > self.config.max_history_entries {
            history.remove(0);
        }

        // Clear failure tracker on successful deployment
        if let Some(tracker) = state.failure_trackers.get_mut(service) {
            tracker.clear();
        }
    }

    /// Records a health check failure for auto-rollback evaluation.
    pub fn record_health_failure(&self, service: &str) {
        let mut state = self.state.write().unwrap();
        let tracker = state
            .failure_trackers
            .entry(service.to_string())
            .or_insert_with(FailureTracker::new);
        tracker.record_failure();
    }

    /// Records a health check success, clearing failure tracking.
    pub fn record_health_success(&self, service: &str) {
        let mut state = self.state.write().unwrap();
        if let Some(tracker) = state.failure_trackers.get_mut(service) {
            tracker.clear();
        }
    }

    /// Checks if an auto-rollback should be triggered for a service.
    ///
    /// Returns `Some(target_version)` if auto-rollback should be triggered,
    /// `None` otherwise.
    pub fn should_auto_rollback(&self, service: &str) -> Option<String> {
        if !self.config.auto_rollback_enabled {
            return None;
        }

        let state = self.state.read().unwrap();

        // Get failure tracker
        let tracker = state.failure_trackers.get(service)?;

        // Check if we've exceeded the failure threshold
        let failure_count = tracker.failures_in_window(self.config.failure_window);
        if failure_count < self.config.failure_threshold as usize {
            return None;
        }

        // Check cooldown
        if !tracker.can_auto_rollback(self.config.cooldown_period) {
            tracing::debug!(
                service = %service,
                "auto-rollback skipped due to cooldown"
            );
            return None;
        }

        // Find previous healthy version
        self.get_previous_version(service)
    }

    /// Gets the previous version for a service (for rollback).
    pub fn get_previous_version(&self, service: &str) -> Option<String> {
        let state = self.state.read().unwrap();

        let history = state.version_history.get(service)?;
        let current = state.current_versions.get(service)?;

        // Find the most recent version that isn't the current one
        history
            .iter()
            .rev()
            .find(|h| h.version != *current && h.is_healthy)
            .map(|h| h.version.clone())
    }

    /// Gets the current version for a service.
    pub fn get_current_version(&self, service: &str) -> Option<String> {
        self.state.read().unwrap().current_versions.get(service).cloned()
    }

    /// Gets the version history for a service.
    pub fn get_version_history(&self, service: &str) -> Vec<VersionHistory> {
        self.state
            .read()
            .unwrap()
            .version_history
            .get(service)
            .cloned()
            .unwrap_or_default()
    }

    /// Gets the rollback history.
    pub fn get_rollback_history(&self) -> Vec<RollbackResult> {
        self.state.read().unwrap().rollback_history.clone()
    }

    /// Gets rollback history for a specific service.
    pub fn get_rollback_history_for_service(&self, service: &str) -> Vec<RollbackResult> {
        self.state
            .read()
            .unwrap()
            .rollback_history
            .iter()
            .filter(|r| r.service == service)
            .cloned()
            .collect()
    }

    /// Records a completed rollback.
    ///
    /// This method updates the internal state and logs the rollback completion
    /// to the audit logger if configured.
    pub fn record_rollback(&self, result: RollbackResult) {
        // Log rollback completion
        self.log_rollback_completed(&result.service, &result.to_version, result.success);

        let mut state = self.state.write().unwrap();

        // If successful, update current version
        if result.success {
            state
                .current_versions
                .insert(result.service.clone(), result.to_version.clone());

            // Mark auto-rollback in tracker if automatic
            if result.is_automatic {
                if let Some(tracker) = state.failure_trackers.get_mut(&result.service) {
                    tracker.mark_auto_rollback();
                }
            }
        }

        // Add to history
        state.rollback_history.push(result);

        // Trim history if needed
        while state.rollback_history.len() > self.config.max_history_entries * 10 {
            state.rollback_history.remove(0);
        }
    }

    /// Prepares and validates a rollback, logging the start event.
    ///
    /// Call this before executing the actual rollback to validate and log.
    /// Returns the current version being rolled back from.
    pub fn prepare_rollback(&self, trigger: &RollbackTrigger) -> Result<String> {
        // Validate first
        self.validate_rollback(trigger)?;

        // Get current version
        let current_version = self.get_current_version(&trigger.service)
            .unwrap_or_else(|| "unknown".to_string());

        // Log rollback started
        self.log_rollback_started(&trigger.service, &current_version, &trigger.target_version);

        tracing::info!(
            service = %trigger.service,
            from_version = %current_version,
            to_version = %trigger.target_version,
            reason = %trigger.reason,
            is_automatic = trigger.is_automatic,
            "rollback initiated"
        );

        Ok(current_version)
    }

    /// Validates that a rollback can be performed.
    pub fn validate_rollback(&self, trigger: &RollbackTrigger) -> Result<()> {
        let state = self.state.read().unwrap();

        // Check if service has version history
        if !state.version_history.contains_key(&trigger.service) {
            return Err(DistributorError::InvalidOperation {
                reason: format!(
                    "no version history for service '{}'",
                    trigger.service
                ),
            });
        }

        // Check if target version exists in history
        let history = state.version_history.get(&trigger.service).unwrap();
        let version_exists = history.iter().any(|h| h.version == trigger.target_version);

        if !version_exists && !trigger.force {
            return Err(DistributorError::InvalidOperation {
                reason: format!(
                    "target version '{}' not found in history for service '{}'",
                    trigger.target_version, trigger.service
                ),
            });
        }

        // Check if we're already at target version
        if let Some(current) = state.current_versions.get(&trigger.service) {
            if *current == trigger.target_version && !trigger.force {
                return Err(DistributorError::InvalidOperation {
                    reason: format!(
                        "service '{}' is already at version '{}'",
                        trigger.service, trigger.target_version
                    ),
                });
            }
        }

        // Check cooldown for auto-rollback
        if trigger.is_automatic {
            if let Some(tracker) = state.failure_trackers.get(&trigger.service) {
                if !tracker.can_auto_rollback(self.config.cooldown_period) {
                    return Err(DistributorError::InvalidOperation {
                        reason: "auto-rollback is in cooldown period".to_string(),
                    });
                }
            }
        }

        Ok(())
    }

    /// Checks if auto-rollback is enabled.
    #[must_use]
    pub const fn is_auto_rollback_enabled(&self) -> bool {
        self.config.auto_rollback_enabled
    }

    /// Gets the failure threshold.
    #[must_use]
    pub const fn failure_threshold(&self) -> u32 {
        self.config.failure_threshold
    }

    /// Gets the current configuration.
    #[must_use]
    pub const fn config(&self) -> &RollbackConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rollback_config_default() {
        let config = RollbackConfig::default();
        assert!(config.auto_rollback_enabled);
        assert_eq!(config.failure_threshold, 3);
        assert_eq!(config.failure_window, Duration::from_secs(60));
        assert_eq!(config.cooldown_period, Duration::from_secs(300));
    }

    #[test]
    fn test_rollback_config_builder() {
        let config = RollbackConfig::builder()
            .auto_rollback(false)
            .failure_threshold(5)
            .failure_window(Duration::from_secs(120))
            .cooldown_period(Duration::from_secs(600))
            .build();

        assert!(!config.auto_rollback_enabled);
        assert_eq!(config.failure_threshold, 5);
        assert_eq!(config.failure_window, Duration::from_secs(120));
        assert_eq!(config.cooldown_period, Duration::from_secs(600));
    }

    #[test]
    fn test_rollback_trigger_manual() {
        let trigger = RollbackTrigger::manual("users-service", "1.0.0", "Performance issue");

        assert_eq!(trigger.service, "users-service");
        assert_eq!(trigger.target_version, "1.0.0");
        assert_eq!(trigger.reason, "Performance issue");
        assert!(!trigger.is_automatic);
        assert!(!trigger.force);
    }

    #[test]
    fn test_rollback_trigger_automatic() {
        let trigger = RollbackTrigger::automatic("users-service", "1.0.0", "Health check failures");

        assert!(trigger.is_automatic);
    }

    #[test]
    fn test_rollback_trigger_with_force() {
        let trigger = RollbackTrigger::manual("users-service", "1.0.0", "Emergency")
            .with_force();

        assert!(trigger.force);
    }

    #[test]
    fn test_rollback_result_success() {
        let result = RollbackResult::success(
            "rollback-123",
            "users-service",
            "1.1.0",
            "1.0.0",
            3,
            Duration::from_secs(5),
            "Performance issue",
            false,
        );

        assert!(result.success);
        assert_eq!(result.instances_rolled_back, 3);
        assert_eq!(result.instances_failed, 0);
        assert!(result.error.is_none());
    }

    #[test]
    fn test_rollback_result_failure() {
        let result = RollbackResult::failure(
            "rollback-123",
            "users-service",
            "1.1.0",
            "1.0.0",
            1,
            2,
            Duration::from_secs(5),
            "Performance issue",
            false,
            "Connection refused",
        );

        assert!(!result.success);
        assert_eq!(result.instances_rolled_back, 1);
        assert_eq!(result.instances_failed, 2);
        assert!(result.error.is_some());
    }

    #[test]
    fn test_rollback_controller_record_deployment() {
        let controller = RollbackController::new(RollbackConfig::default());

        controller.record_deployment("users-service", "1.0.0", "deploy-1");
        controller.record_deployment("users-service", "1.1.0", "deploy-2");

        let current = controller.get_current_version("users-service");
        assert_eq!(current, Some("1.1.0".to_string()));

        let history = controller.get_version_history("users-service");
        assert_eq!(history.len(), 2);
    }

    #[test]
    fn test_rollback_controller_get_previous_version() {
        let controller = RollbackController::new(RollbackConfig::default());

        controller.record_deployment("users-service", "1.0.0", "deploy-1");
        controller.record_deployment("users-service", "1.1.0", "deploy-2");
        controller.record_deployment("users-service", "1.2.0", "deploy-3");

        let previous = controller.get_previous_version("users-service");
        assert_eq!(previous, Some("1.1.0".to_string()));
    }

    #[test]
    fn test_rollback_controller_no_previous_version() {
        let controller = RollbackController::new(RollbackConfig::default());

        controller.record_deployment("users-service", "1.0.0", "deploy-1");

        let previous = controller.get_previous_version("users-service");
        assert!(previous.is_none());
    }

    #[test]
    fn test_rollback_controller_health_failure_tracking() {
        let config = RollbackConfig::builder()
            .failure_threshold(3)
            .failure_window(Duration::from_secs(60))
            .build();
        let controller = RollbackController::new(config);

        controller.record_deployment("users-service", "1.0.0", "deploy-1");
        controller.record_deployment("users-service", "1.1.0", "deploy-2");

        // Record failures below threshold
        controller.record_health_failure("users-service");
        controller.record_health_failure("users-service");

        let should_rollback = controller.should_auto_rollback("users-service");
        assert!(should_rollback.is_none());

        // Third failure triggers auto-rollback
        controller.record_health_failure("users-service");

        let should_rollback = controller.should_auto_rollback("users-service");
        assert_eq!(should_rollback, Some("1.0.0".to_string()));
    }

    #[test]
    fn test_rollback_controller_auto_rollback_disabled() {
        let config = RollbackConfig::builder()
            .auto_rollback(false)
            .build();
        let controller = RollbackController::new(config);

        controller.record_deployment("users-service", "1.0.0", "deploy-1");
        controller.record_deployment("users-service", "1.1.0", "deploy-2");

        // Record many failures
        for _ in 0..10 {
            controller.record_health_failure("users-service");
        }

        let should_rollback = controller.should_auto_rollback("users-service");
        assert!(should_rollback.is_none());
    }

    #[test]
    fn test_rollback_controller_health_success_clears_failures() {
        let config = RollbackConfig::builder()
            .failure_threshold(3)
            .build();
        let controller = RollbackController::new(config);

        controller.record_deployment("users-service", "1.0.0", "deploy-1");
        controller.record_deployment("users-service", "1.1.0", "deploy-2");

        controller.record_health_failure("users-service");
        controller.record_health_failure("users-service");
        controller.record_health_success("users-service"); // Clears failures

        controller.record_health_failure("users-service");
        controller.record_health_failure("users-service");

        // Should not trigger because failures were cleared
        let should_rollback = controller.should_auto_rollback("users-service");
        assert!(should_rollback.is_none());
    }

    #[test]
    fn test_rollback_controller_validate_rollback() {
        let controller = RollbackController::new(RollbackConfig::default());

        controller.record_deployment("users-service", "1.0.0", "deploy-1");
        controller.record_deployment("users-service", "1.1.0", "deploy-2");

        // Valid rollback
        let trigger = RollbackTrigger::manual("users-service", "1.0.0", "Test");
        assert!(controller.validate_rollback(&trigger).is_ok());

        // Invalid: no history
        let trigger = RollbackTrigger::manual("unknown-service", "1.0.0", "Test");
        assert!(controller.validate_rollback(&trigger).is_err());

        // Invalid: version not in history
        let trigger = RollbackTrigger::manual("users-service", "0.5.0", "Test");
        assert!(controller.validate_rollback(&trigger).is_err());

        // Invalid: already at target version
        let trigger = RollbackTrigger::manual("users-service", "1.1.0", "Test");
        assert!(controller.validate_rollback(&trigger).is_err());

        // Force bypasses version check
        let trigger = RollbackTrigger::manual("users-service", "0.5.0", "Test").with_force();
        assert!(controller.validate_rollback(&trigger).is_ok());
    }

    #[test]
    fn test_rollback_controller_record_rollback() {
        let controller = RollbackController::new(RollbackConfig::default());

        controller.record_deployment("users-service", "1.0.0", "deploy-1");
        controller.record_deployment("users-service", "1.1.0", "deploy-2");

        let result = RollbackResult::success(
            "rollback-1",
            "users-service",
            "1.1.0",
            "1.0.0",
            3,
            Duration::from_secs(5),
            "Test",
            false,
        );

        controller.record_rollback(result);

        // Current version should be updated
        assert_eq!(
            controller.get_current_version("users-service"),
            Some("1.0.0".to_string())
        );

        // Rollback history should contain entry
        let history = controller.get_rollback_history();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].to_version, "1.0.0");
    }

    #[test]
    fn test_failure_tracker() {
        let mut tracker = FailureTracker::new();

        tracker.record_failure();
        tracker.record_failure();

        assert_eq!(tracker.failures_in_window(Duration::from_secs(60)), 2);
        assert!(tracker.can_auto_rollback(Duration::from_secs(300)));

        tracker.mark_auto_rollback();
        assert!(!tracker.can_auto_rollback(Duration::from_secs(300)));
        assert_eq!(tracker.failures_in_window(Duration::from_secs(60)), 0);
    }

    #[test]
    fn test_version_history_serialization() {
        let history = VersionHistory {
            version: "1.0.0".to_string(),
            deployed_at: Utc::now(),
            is_healthy: true,
            deployment_id: "deploy-1".to_string(),
        };

        let json = serde_json::to_string(&history).unwrap();
        assert!(json.contains("1.0.0"));
        assert!(json.contains("deploy-1"));

        let parsed: VersionHistory = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.version, "1.0.0");
    }

    #[test]
    fn test_rollback_controller_with_audit_logger() {
        use eunomia_audit::{InMemoryBackend, TracingBackend};

        let config = RollbackConfig::default();
        let backend = Arc::new(InMemoryBackend::new());
        let logger = Arc::new(
            AuditLogger::builder()
                .with_backend(backend.clone())
                .build(),
        );

        let controller = RollbackController::with_audit_logger(config, logger);

        // Record deployments
        controller.record_deployment("users-service", "1.0.0", "deploy-1");
        controller.record_deployment("users-service", "1.1.0", "deploy-2");

        // Prepare rollback (logs started event)
        let trigger = RollbackTrigger::manual("users-service", "1.0.0", "Test audit logging");
        let from_version = controller.prepare_rollback(&trigger).unwrap();
        assert_eq!(from_version, "1.1.0");

        // Record rollback (logs completed event)
        let result = RollbackResult::success(
            "rollback-1",
            "users-service",
            "1.1.0",
            "1.0.0",
            3,
            Duration::from_secs(5),
            "Test audit logging",
            false,
        );
        controller.record_rollback(result);

        // Verify audit events were logged
        let events = backend.events();
        assert_eq!(events.len(), 2);

        // First event should be rollback started
        assert!(events[0].contains("rollback_started"));
        assert!(events[0].contains("users-service"));

        // Second event should be rollback completed
        assert!(events[1].contains("rollback_completed"));
        assert!(events[1].contains("users-service"));
    }

    #[test]
    fn test_rollback_controller_set_audit_logger() {
        use eunomia_audit::InMemoryBackend;

        let config = RollbackConfig::default();
        let mut controller = RollbackController::new(config);

        // Initially no logger
        controller.record_deployment("svc", "1.0.0", "d-1");
        controller.record_deployment("svc", "2.0.0", "d-2");

        // Add logger later
        let backend = Arc::new(InMemoryBackend::new());
        let logger = Arc::new(
            AuditLogger::builder()
                .with_backend(backend.clone())
                .build(),
        );
        controller.set_audit_logger(logger);

        // Now events should be logged
        let result = RollbackResult::success(
            "rb-1",
            "svc",
            "2.0.0",
            "1.0.0",
            1,
            Duration::from_secs(1),
            "Test",
            false,
        );
        controller.record_rollback(result);

        let events = backend.events();
        assert_eq!(events.len(), 1);
        assert!(events[0].contains("rollback_completed"));
    }
}
