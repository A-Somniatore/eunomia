//! Deployment strategy types.
//!
//! This module provides different strategies for deploying policies
//! across Archimedes instances.

use std::time::Duration;

/// Deployment strategy configuration.
#[derive(Debug, Clone)]
pub struct DeploymentStrategy {
    /// Type of deployment strategy.
    strategy_type: StrategyType,

    /// Canary percentage (for canary deployments).
    canary_percentage: Option<u32>,

    /// Canary duration (how long to wait for canary validation).
    canary_duration: Option<Duration>,

    /// Batch size (for rolling deployments).
    batch_size: Option<usize>,

    /// Delay between batches (for rolling deployments).
    batch_delay: Option<Duration>,

    /// Maximum number of failures before aborting.
    max_failures: Option<u32>,

    /// Automatically rollback on failure.
    auto_rollback: bool,
}

impl DeploymentStrategy {
    /// Creates an immediate deployment strategy.
    ///
    /// Deploys to all instances simultaneously.
    pub fn immediate() -> Self {
        Self {
            strategy_type: StrategyType::Immediate,
            canary_percentage: None,
            canary_duration: None,
            batch_size: None,
            batch_delay: None,
            max_failures: None,
            auto_rollback: false,
        }
    }

    /// Creates a canary deployment strategy.
    ///
    /// # Arguments
    ///
    /// * `percentage` - Percentage of instances for canary (1-100)
    /// * `duration` - Duration to wait for canary validation
    pub fn canary(percentage: u32, duration: Duration) -> Self {
        Self {
            strategy_type: StrategyType::Canary,
            canary_percentage: Some(percentage.clamp(1, 100)),
            canary_duration: Some(duration),
            batch_size: None,
            batch_delay: None,
            max_failures: None,
            auto_rollback: true,
        }
    }

    /// Creates a rolling deployment strategy.
    ///
    /// # Arguments
    ///
    /// * `batch_size` - Number of instances per batch
    /// * `delay` - Delay between batches
    pub fn rolling(batch_size: usize, delay: Duration) -> Self {
        Self {
            strategy_type: StrategyType::Rolling,
            canary_percentage: None,
            canary_duration: None,
            batch_size: Some(batch_size.max(1)),
            batch_delay: Some(delay),
            max_failures: None,
            auto_rollback: true,
        }
    }

    /// Returns the strategy type.
    pub fn strategy_type(&self) -> StrategyType {
        self.strategy_type
    }

    /// Returns the canary count for a given total number of instances.
    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss
    )]
    pub fn canary_count(&self, total: usize) -> usize {
        self.canary_percentage.map_or(1, |pct| {
            ((total as f64) * (f64::from(pct) / 100.0)).ceil() as usize
        })
    }

    /// Returns the canary duration.
    pub fn canary_duration(&self) -> Option<Duration> {
        self.canary_duration
    }

    /// Returns the batch size.
    pub fn batch_size(&self) -> Option<usize> {
        self.batch_size
    }

    /// Returns the batch delay.
    pub fn batch_delay(&self) -> Option<Duration> {
        self.batch_delay
    }

    /// Returns the maximum number of failures before aborting.
    pub fn max_failures(&self) -> Option<u32> {
        self.max_failures
    }

    /// Returns whether auto-rollback is enabled.
    pub fn auto_rollback(&self) -> bool {
        self.auto_rollback
    }

    /// Sets the maximum number of failures.
    pub fn with_max_failures(mut self, max: u32) -> Self {
        self.max_failures = Some(max);
        self
    }

    /// Sets the auto-rollback behavior.
    pub fn with_auto_rollback(mut self, enabled: bool) -> Self {
        self.auto_rollback = enabled;
        self
    }
}

/// Type of deployment strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StrategyType {
    /// Deploy to all instances immediately.
    Immediate,

    /// Deploy to a subset first (canary), then to the rest.
    Canary,

    /// Deploy in batches with delays.
    Rolling,
}

impl std::fmt::Display for StrategyType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Immediate => write!(f, "immediate"),
            Self::Canary => write!(f, "canary"),
            Self::Rolling => write!(f, "rolling"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_immediate_strategy() {
        let strategy = DeploymentStrategy::immediate();
        assert_eq!(strategy.strategy_type(), StrategyType::Immediate);
        assert!(strategy.canary_duration().is_none());
        assert!(strategy.batch_size().is_none());
        assert!(!strategy.auto_rollback());
    }

    #[test]
    fn test_canary_strategy() {
        let strategy = DeploymentStrategy::canary(10, Duration::from_secs(300));
        assert_eq!(strategy.strategy_type(), StrategyType::Canary);
        assert_eq!(strategy.canary_percentage, Some(10));
        assert_eq!(strategy.canary_duration(), Some(Duration::from_secs(300)));
        assert!(strategy.auto_rollback());
    }

    #[test]
    fn test_canary_count() {
        let strategy = DeploymentStrategy::canary(10, Duration::from_secs(60));

        // 10% of 100 = 10
        assert_eq!(strategy.canary_count(100), 10);

        // 10% of 50 = 5
        assert_eq!(strategy.canary_count(50), 5);

        // 10% of 5 = 0.5, ceil to 1
        assert_eq!(strategy.canary_count(5), 1);

        // 10% of 1 = 0.1, ceil to 1
        assert_eq!(strategy.canary_count(1), 1);
    }

    #[test]
    fn test_canary_percentage_clamped() {
        let strategy_high = DeploymentStrategy::canary(150, Duration::from_secs(60));
        assert_eq!(strategy_high.canary_percentage, Some(100));

        let strategy_low = DeploymentStrategy::canary(0, Duration::from_secs(60));
        assert_eq!(strategy_low.canary_percentage, Some(1));
    }

    #[test]
    fn test_rolling_strategy() {
        let strategy = DeploymentStrategy::rolling(5, Duration::from_secs(10));
        assert_eq!(strategy.strategy_type(), StrategyType::Rolling);
        assert_eq!(strategy.batch_size(), Some(5));
        assert_eq!(strategy.batch_delay(), Some(Duration::from_secs(10)));
        assert!(strategy.auto_rollback());
    }

    #[test]
    fn test_rolling_batch_size_min() {
        let strategy = DeploymentStrategy::rolling(0, Duration::from_secs(10));
        assert_eq!(strategy.batch_size(), Some(1)); // Minimum is 1
    }

    #[test]
    fn test_with_max_failures() {
        let strategy = DeploymentStrategy::rolling(5, Duration::from_secs(10)).with_max_failures(3);
        assert_eq!(strategy.max_failures(), Some(3));
    }

    #[test]
    fn test_with_auto_rollback() {
        let strategy = DeploymentStrategy::immediate().with_auto_rollback(true);
        assert!(strategy.auto_rollback());

        let strategy =
            DeploymentStrategy::canary(10, Duration::from_secs(60)).with_auto_rollback(false);
        assert!(!strategy.auto_rollback());
    }

    #[test]
    fn test_strategy_type_display() {
        assert_eq!(StrategyType::Immediate.to_string(), "immediate");
        assert_eq!(StrategyType::Canary.to_string(), "canary");
        assert_eq!(StrategyType::Rolling.to_string(), "rolling");
    }

    #[test]
    fn test_strategy_type_equality() {
        assert_eq!(StrategyType::Immediate, StrategyType::Immediate);
        assert_ne!(StrategyType::Immediate, StrategyType::Canary);
    }
}
