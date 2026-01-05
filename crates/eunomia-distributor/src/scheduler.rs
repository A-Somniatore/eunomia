//! Deployment scheduler.
//!
//! This module provides scheduling capabilities for coordinating
//! policy deployments across multiple instances.

use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::strategy::DeploymentStrategy;

/// Configuration for the deployment scheduler.
#[derive(Debug, Clone)]
pub struct SchedulerConfig {
    /// Maximum concurrent deployments.
    pub max_concurrent: usize,

    /// Queue size limit.
    pub max_queue_size: usize,

    /// Enable prioritization of deployments.
    pub enable_priority: bool,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            max_concurrent: 5,
            max_queue_size: 100,
            enable_priority: true,
        }
    }
}

impl SchedulerConfig {
    /// Creates a configuration builder.
    pub fn builder() -> SchedulerConfigBuilder {
        SchedulerConfigBuilder::default()
    }
}

/// Builder for `SchedulerConfig`.
#[derive(Debug, Default)]
pub struct SchedulerConfigBuilder {
    max_concurrent: Option<usize>,
    max_queue_size: Option<usize>,
    enable_priority: Option<bool>,
}

impl SchedulerConfigBuilder {
    /// Sets the maximum concurrent deployments.
    pub fn max_concurrent(mut self, max: usize) -> Self {
        self.max_concurrent = Some(max);
        self
    }

    /// Sets the maximum queue size.
    pub fn max_queue_size(mut self, size: usize) -> Self {
        self.max_queue_size = Some(size);
        self
    }

    /// Enables or disables priority.
    pub fn enable_priority(mut self, enabled: bool) -> Self {
        self.enable_priority = Some(enabled);
        self
    }

    /// Builds the configuration.
    pub fn build(self) -> SchedulerConfig {
        let defaults = SchedulerConfig::default();
        SchedulerConfig {
            max_concurrent: self.max_concurrent.unwrap_or(defaults.max_concurrent),
            max_queue_size: self.max_queue_size.unwrap_or(defaults.max_queue_size),
            enable_priority: self.enable_priority.unwrap_or(defaults.enable_priority),
        }
    }
}

/// Priority level for deployments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum DeploymentPriority {
    /// Low priority - can wait.
    Low = 0,

    /// Normal priority - default.
    #[default]
    Normal = 1,

    /// High priority - should be processed soon.
    High = 2,

    /// Critical priority - process immediately.
    Critical = 3,
}

/// A scheduled deployment request.
#[derive(Debug, Clone)]
pub struct ScheduledDeployment {
    /// Unique deployment ID.
    pub id: String,

    /// Target service.
    pub service: String,

    /// Target version.
    pub version: String,

    /// Deployment strategy.
    pub strategy: DeploymentStrategy,

    /// Priority level.
    pub priority: DeploymentPriority,

    /// When this deployment was queued.
    pub queued_at: std::time::Instant,
}

impl ScheduledDeployment {
    /// Creates a new scheduled deployment.
    pub fn new(
        id: impl Into<String>,
        service: impl Into<String>,
        version: impl Into<String>,
        strategy: DeploymentStrategy,
    ) -> Self {
        Self {
            id: id.into(),
            service: service.into(),
            version: version.into(),
            strategy,
            priority: DeploymentPriority::default(),
            queued_at: std::time::Instant::now(),
        }
    }

    /// Sets the priority.
    pub fn with_priority(mut self, priority: DeploymentPriority) -> Self {
        self.priority = priority;
        self
    }
}

/// Scheduler for managing deployment order and concurrency.
pub struct DeploymentScheduler {
    config: SchedulerConfig,
    queue: Arc<RwLock<VecDeque<ScheduledDeployment>>>,
    active_count: Arc<RwLock<usize>>,
}

impl DeploymentScheduler {
    /// Creates a new scheduler.
    pub fn new(config: SchedulerConfig) -> Self {
        Self {
            config,
            queue: Arc::new(RwLock::new(VecDeque::new())),
            active_count: Arc::new(RwLock::new(0)),
        }
    }

    /// Enqueues a deployment.
    pub async fn enqueue(&self, deployment: ScheduledDeployment) -> Result<(), SchedulerError> {
        let mut queue = self.queue.write().await;

        if queue.len() >= self.config.max_queue_size {
            return Err(SchedulerError::QueueFull);
        }

        if self.config.enable_priority {
            // Insert based on priority
            let pos = queue
                .iter()
                .position(|d| d.priority < deployment.priority)
                .unwrap_or(queue.len());
            queue.insert(pos, deployment);
        } else {
            queue.push_back(deployment);
        }

        Ok(())
    }

    /// Dequeues the next deployment if capacity is available.
    pub async fn dequeue(&self) -> Option<ScheduledDeployment> {
        let active = *self.active_count.read().await;
        if active >= self.config.max_concurrent {
            return None;
        }

        let mut queue = self.queue.write().await;
        let deployment = queue.pop_front()?;

        let mut active = self.active_count.write().await;
        *active += 1;

        Some(deployment)
    }

    /// Marks a deployment as complete, freeing a slot.
    pub async fn complete(&self, _deployment_id: &str) {
        let mut active = self.active_count.write().await;
        *active = active.saturating_sub(1);
    }

    /// Returns the current queue length.
    pub async fn queue_length(&self) -> usize {
        self.queue.read().await.len()
    }

    /// Returns the number of active deployments.
    pub async fn active_count(&self) -> usize {
        *self.active_count.read().await
    }

    /// Returns true if capacity is available.
    pub async fn has_capacity(&self) -> bool {
        *self.active_count.read().await < self.config.max_concurrent
    }

    /// Clears the queue.
    pub async fn clear_queue(&self) {
        let mut queue = self.queue.write().await;
        queue.clear();
    }

    /// Removes a specific deployment from the queue.
    pub async fn remove(&self, deployment_id: &str) -> Option<ScheduledDeployment> {
        let mut queue = self.queue.write().await;
        let pos = queue.iter().position(|d| d.id == deployment_id)?;
        queue.remove(pos)
    }

    /// Lists all queued deployments.
    pub async fn list_queued(&self) -> Vec<ScheduledDeployment> {
        self.queue.read().await.iter().cloned().collect()
    }
}

/// Errors that can occur during scheduling.
#[derive(Debug, thiserror::Error)]
pub enum SchedulerError {
    /// Queue is full.
    #[error("deployment queue is full")]
    QueueFull,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scheduler_config_default() {
        let config = SchedulerConfig::default();
        assert_eq!(config.max_concurrent, 5);
        assert_eq!(config.max_queue_size, 100);
        assert!(config.enable_priority);
    }

    #[test]
    fn test_scheduler_config_builder() {
        let config = SchedulerConfig::builder()
            .max_concurrent(10)
            .max_queue_size(50)
            .enable_priority(false)
            .build();

        assert_eq!(config.max_concurrent, 10);
        assert_eq!(config.max_queue_size, 50);
        assert!(!config.enable_priority);
    }

    #[test]
    fn test_deployment_priority_ordering() {
        assert!(DeploymentPriority::Low < DeploymentPriority::Normal);
        assert!(DeploymentPriority::Normal < DeploymentPriority::High);
        assert!(DeploymentPriority::High < DeploymentPriority::Critical);
    }

    #[test]
    fn test_scheduled_deployment_creation() {
        let strategy = DeploymentStrategy::immediate();
        let deployment = ScheduledDeployment::new("deploy-1", "my-service", "1.0.0", strategy);

        assert_eq!(deployment.id, "deploy-1");
        assert_eq!(deployment.service, "my-service");
        assert_eq!(deployment.version, "1.0.0");
        assert_eq!(deployment.priority, DeploymentPriority::Normal);
    }

    #[test]
    fn test_scheduled_deployment_with_priority() {
        let strategy = DeploymentStrategy::immediate();
        let deployment = ScheduledDeployment::new("deploy-1", "my-service", "1.0.0", strategy)
            .with_priority(DeploymentPriority::Critical);

        assert_eq!(deployment.priority, DeploymentPriority::Critical);
    }

    #[tokio::test]
    async fn test_scheduler_enqueue_dequeue() {
        let config = SchedulerConfig::default();
        let scheduler = DeploymentScheduler::new(config);

        let strategy = DeploymentStrategy::immediate();
        let deployment = ScheduledDeployment::new("deploy-1", "my-service", "1.0.0", strategy);

        scheduler.enqueue(deployment).await.unwrap();
        assert_eq!(scheduler.queue_length().await, 1);

        let dequeued = scheduler.dequeue().await;
        assert!(dequeued.is_some());
        assert_eq!(dequeued.unwrap().id, "deploy-1");
        assert_eq!(scheduler.queue_length().await, 0);
    }

    #[tokio::test]
    async fn test_scheduler_respects_max_concurrent() {
        let config = SchedulerConfig {
            max_concurrent: 2,
            ..SchedulerConfig::default()
        };
        let scheduler = DeploymentScheduler::new(config);

        // Enqueue 3 deployments
        for i in 0..3 {
            let strategy = DeploymentStrategy::immediate();
            let deployment =
                ScheduledDeployment::new(format!("deploy-{}", i), "service", "1.0.0", strategy);
            scheduler.enqueue(deployment).await.unwrap();
        }

        // Should be able to dequeue 2
        assert!(scheduler.dequeue().await.is_some());
        assert!(scheduler.dequeue().await.is_some());

        // Third should be blocked
        assert!(scheduler.dequeue().await.is_none());

        // Complete one
        scheduler.complete("deploy-0").await;

        // Now should be able to dequeue
        assert!(scheduler.dequeue().await.is_some());
    }

    #[tokio::test]
    async fn test_scheduler_queue_limit() {
        let config = SchedulerConfig {
            max_queue_size: 2,
            ..SchedulerConfig::default()
        };
        let scheduler = DeploymentScheduler::new(config);

        // Fill queue
        for i in 0..2 {
            let strategy = DeploymentStrategy::immediate();
            let deployment =
                ScheduledDeployment::new(format!("deploy-{}", i), "service", "1.0.0", strategy);
            scheduler.enqueue(deployment).await.unwrap();
        }

        // Third should fail
        let strategy = DeploymentStrategy::immediate();
        let deployment = ScheduledDeployment::new("deploy-2", "service", "1.0.0", strategy);
        let result = scheduler.enqueue(deployment).await;
        assert!(matches!(result, Err(SchedulerError::QueueFull)));
    }

    #[tokio::test]
    async fn test_scheduler_priority_ordering() {
        let config = SchedulerConfig {
            enable_priority: true,
            ..SchedulerConfig::default()
        };
        let scheduler = DeploymentScheduler::new(config);

        // Enqueue in reverse priority order
        let priorities = [
            DeploymentPriority::Low,
            DeploymentPriority::Normal,
            DeploymentPriority::Critical,
        ];

        for (i, priority) in priorities.iter().enumerate() {
            let strategy = DeploymentStrategy::immediate();
            let deployment =
                ScheduledDeployment::new(format!("deploy-{}", i), "service", "1.0.0", strategy)
                    .with_priority(*priority);
            scheduler.enqueue(deployment).await.unwrap();
        }

        // Should dequeue in priority order (Critical first)
        let first = scheduler.dequeue().await.unwrap();
        assert_eq!(first.priority, DeploymentPriority::Critical);

        let second = scheduler.dequeue().await.unwrap();
        assert_eq!(second.priority, DeploymentPriority::Normal);

        let third = scheduler.dequeue().await.unwrap();
        assert_eq!(third.priority, DeploymentPriority::Low);
    }

    #[tokio::test]
    async fn test_scheduler_remove() {
        let config = SchedulerConfig::default();
        let scheduler = DeploymentScheduler::new(config);

        let strategy = DeploymentStrategy::immediate();
        let deployment = ScheduledDeployment::new("deploy-1", "service", "1.0.0", strategy);
        scheduler.enqueue(deployment).await.unwrap();

        let removed = scheduler.remove("deploy-1").await;
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().id, "deploy-1");
        assert_eq!(scheduler.queue_length().await, 0);
    }

    #[tokio::test]
    async fn test_scheduler_list_queued() {
        let config = SchedulerConfig {
            enable_priority: false,
            ..SchedulerConfig::default()
        };
        let scheduler = DeploymentScheduler::new(config);

        for i in 0..3 {
            let strategy = DeploymentStrategy::immediate();
            let deployment =
                ScheduledDeployment::new(format!("deploy-{}", i), "service", "1.0.0", strategy);
            scheduler.enqueue(deployment).await.unwrap();
        }

        let queued = scheduler.list_queued().await;
        assert_eq!(queued.len(), 3);
        assert_eq!(queued[0].id, "deploy-0");
        assert_eq!(queued[1].id, "deploy-1");
        assert_eq!(queued[2].id, "deploy-2");
    }

    #[tokio::test]
    async fn test_scheduler_clear_queue() {
        let config = SchedulerConfig::default();
        let scheduler = DeploymentScheduler::new(config);

        for i in 0..3 {
            let strategy = DeploymentStrategy::immediate();
            let deployment =
                ScheduledDeployment::new(format!("deploy-{}", i), "service", "1.0.0", strategy);
            scheduler.enqueue(deployment).await.unwrap();
        }

        assert_eq!(scheduler.queue_length().await, 3);
        scheduler.clear_queue().await;
        assert_eq!(scheduler.queue_length().await, 0);
    }

    #[tokio::test]
    async fn test_scheduler_has_capacity() {
        let config = SchedulerConfig {
            max_concurrent: 1,
            ..SchedulerConfig::default()
        };
        let scheduler = DeploymentScheduler::new(config);

        assert!(scheduler.has_capacity().await);

        let strategy = DeploymentStrategy::immediate();
        let deployment = ScheduledDeployment::new("deploy-1", "service", "1.0.0", strategy);
        scheduler.enqueue(deployment).await.unwrap();
        scheduler.dequeue().await;

        assert!(!scheduler.has_capacity().await);

        scheduler.complete("deploy-1").await;
        assert!(scheduler.has_capacity().await);
    }
}
