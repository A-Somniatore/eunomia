//! Deployment event bus for real-time event streaming.
//!
//! This module provides a publish-subscribe system for deployment events,
//! allowing clients to watch deployment progress in real-time.
//!
//! # Example
//!
//! ```rust,ignore
//! use eunomia_distributor::events::{EventBus, DeploymentEventData};
//!
//! let bus = EventBus::new(100);
//!
//! // Subscribe to events
//! let mut rx = bus.subscribe();
//!
//! // Publish an event
//! bus.publish(DeploymentEventData::started("deploy-1", "users-service", "1.0.0"));
//!
//! // Receive event
//! let event = rx.recv().await.unwrap();
//! ```

use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

/// Event type for deployment lifecycle events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventType {
    /// Deployment started.
    DeploymentStarted,
    /// Instance update started.
    InstanceUpdateStarted,
    /// Instance update completed successfully.
    InstanceUpdateCompleted,
    /// Instance update failed.
    InstanceUpdateFailed,
    /// Instance skipped (already at target version).
    InstanceSkipped,
    /// Batch completed (for rolling deployments).
    BatchCompleted,
    /// Canary validation started.
    CanaryValidationStarted,
    /// Canary validation passed.
    CanaryValidationPassed,
    /// Canary validation failed.
    CanaryValidationFailed,
    /// Deployment completed successfully.
    DeploymentCompleted,
    /// Deployment failed.
    DeploymentFailed,
    /// Rollback started.
    RollbackStarted,
    /// Rollback completed.
    RollbackCompleted,
}

impl std::fmt::Display for EventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            EventType::DeploymentStarted => "deployment_started",
            EventType::InstanceUpdateStarted => "instance_update_started",
            EventType::InstanceUpdateCompleted => "instance_update_completed",
            EventType::InstanceUpdateFailed => "instance_update_failed",
            EventType::InstanceSkipped => "instance_skipped",
            EventType::BatchCompleted => "batch_completed",
            EventType::CanaryValidationStarted => "canary_validation_started",
            EventType::CanaryValidationPassed => "canary_validation_passed",
            EventType::CanaryValidationFailed => "canary_validation_failed",
            EventType::DeploymentCompleted => "deployment_completed",
            EventType::DeploymentFailed => "deployment_failed",
            EventType::RollbackStarted => "rollback_started",
            EventType::RollbackCompleted => "rollback_completed",
        };
        write!(f, "{s}")
    }
}

/// Deployment event data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentEventData {
    /// Deployment ID.
    pub deployment_id: String,
    /// Event type.
    pub event_type: EventType,
    /// Event timestamp.
    pub timestamp: DateTime<Utc>,
    /// Service name.
    pub service: String,
    /// Target version.
    pub version: String,
    /// Instance ID (if instance-specific).
    pub instance_id: Option<String>,
    /// Human-readable message.
    pub message: String,
    /// Progress percentage (0-100).
    pub progress: Option<u8>,
    /// Additional metadata.
    pub metadata: Option<serde_json::Value>,
}

impl DeploymentEventData {
    /// Creates a new event.
    fn new(
        deployment_id: &str,
        event_type: EventType,
        service: &str,
        version: &str,
        message: &str,
    ) -> Self {
        Self {
            deployment_id: deployment_id.to_string(),
            event_type,
            timestamp: Utc::now(),
            service: service.to_string(),
            version: version.to_string(),
            instance_id: None,
            message: message.to_string(),
            progress: None,
            metadata: None,
        }
    }

    /// Creates a deployment started event.
    #[must_use]
    pub fn started(deployment_id: &str, service: &str, version: &str) -> Self {
        Self::new(
            deployment_id,
            EventType::DeploymentStarted,
            service,
            version,
            &format!("Deployment started for {service} version {version}"),
        )
    }

    /// Creates a deployment completed event.
    #[must_use]
    pub fn completed(deployment_id: &str, service: &str, version: &str) -> Self {
        let mut event = Self::new(
            deployment_id,
            EventType::DeploymentCompleted,
            service,
            version,
            &format!("Deployment completed for {service} version {version}"),
        );
        event.progress = Some(100);
        event
    }

    /// Creates a deployment failed event.
    #[must_use]
    pub fn failed(deployment_id: &str, service: &str, version: &str, error: &str) -> Self {
        Self::new(
            deployment_id,
            EventType::DeploymentFailed,
            service,
            version,
            &format!("Deployment failed: {error}"),
        )
    }

    /// Creates an instance update started event.
    #[must_use]
    pub fn instance_started(
        deployment_id: &str,
        service: &str,
        version: &str,
        instance_id: &str,
    ) -> Self {
        let mut event = Self::new(
            deployment_id,
            EventType::InstanceUpdateStarted,
            service,
            version,
            &format!("Updating instance {instance_id}"),
        );
        event.instance_id = Some(instance_id.to_string());
        event
    }

    /// Creates an instance update completed event.
    #[must_use]
    pub fn instance_completed(
        deployment_id: &str,
        service: &str,
        version: &str,
        instance_id: &str,
    ) -> Self {
        let mut event = Self::new(
            deployment_id,
            EventType::InstanceUpdateCompleted,
            service,
            version,
            &format!("Instance {instance_id} updated successfully"),
        );
        event.instance_id = Some(instance_id.to_string());
        event
    }

    /// Creates an instance update failed event.
    #[must_use]
    pub fn instance_failed(
        deployment_id: &str,
        service: &str,
        version: &str,
        instance_id: &str,
        error: &str,
    ) -> Self {
        let mut event = Self::new(
            deployment_id,
            EventType::InstanceUpdateFailed,
            service,
            version,
            &format!("Instance {instance_id} failed: {error}"),
        );
        event.instance_id = Some(instance_id.to_string());
        event
    }

    /// Creates a rollback started event.
    #[must_use]
    pub fn rollback_started(
        deployment_id: &str,
        service: &str,
        from_version: &str,
        to_version: &str,
    ) -> Self {
        let mut event = Self::new(
            deployment_id,
            EventType::RollbackStarted,
            service,
            to_version,
            &format!("Rolling back {service} from {from_version} to {to_version}"),
        );
        event.metadata = Some(serde_json::json!({
            "from_version": from_version,
        }));
        event
    }

    /// Creates a rollback completed event.
    #[must_use]
    pub fn rollback_completed(deployment_id: &str, service: &str, version: &str) -> Self {
        let mut event = Self::new(
            deployment_id,
            EventType::RollbackCompleted,
            service,
            version,
            &format!("Rollback completed, now at version {version}"),
        );
        event.progress = Some(100);
        event
    }

    /// Sets the instance ID.
    #[must_use]
    pub fn with_instance_id(mut self, instance_id: &str) -> Self {
        self.instance_id = Some(instance_id.to_string());
        self
    }

    /// Sets the progress percentage.
    #[must_use]
    pub fn with_progress(mut self, progress: u8) -> Self {
        self.progress = Some(progress.min(100));
        self
    }

    /// Sets metadata.
    #[must_use]
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

/// Event bus for publishing and subscribing to deployment events.
///
/// Uses a broadcast channel to allow multiple subscribers to receive
/// the same events. Events are cloned to each subscriber.
#[derive(Clone)]
pub struct EventBus {
    sender: broadcast::Sender<DeploymentEventData>,
}

impl EventBus {
    /// Creates a new event bus with the given capacity.
    ///
    /// The capacity determines how many events can be buffered
    /// before slow subscribers start missing events.
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    /// Creates a new event bus with default capacity (256).
    #[must_use]
    pub fn default_capacity() -> Self {
        Self::new(256)
    }

    /// Publishes an event to all subscribers.
    ///
    /// Returns the number of subscribers that received the event.
    /// If there are no subscribers, the event is dropped and 0 is returned.
    pub fn publish(&self, event: DeploymentEventData) -> usize {
        self.sender.send(event).unwrap_or(0)
    }

    /// Subscribes to receive events.
    ///
    /// Returns a receiver that will receive all future events.
    /// Events published before subscribing are not received.
    pub fn subscribe(&self) -> EventSubscriber {
        EventSubscriber {
            receiver: self.sender.subscribe(),
        }
    }

    /// Returns the number of active subscribers.
    #[must_use]
    pub fn subscriber_count(&self) -> usize {
        self.sender.receiver_count()
    }
}

impl std::fmt::Debug for EventBus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EventBus")
            .field("subscriber_count", &self.subscriber_count())
            .finish()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::default_capacity()
    }
}

/// Subscriber for receiving deployment events.
pub struct EventSubscriber {
    receiver: broadcast::Receiver<DeploymentEventData>,
}

impl EventSubscriber {
    /// Receives the next event.
    ///
    /// Returns `None` if the bus is closed.
    /// Skips lagged events if the subscriber is too slow.
    pub async fn recv(&mut self) -> Option<DeploymentEventData> {
        loop {
            match self.receiver.recv().await {
                Ok(event) => return Some(event),
                Err(broadcast::error::RecvError::Lagged(count)) => {
                    tracing::warn!("Event subscriber lagged, skipped {count} events");
                    continue; // Try to receive the next event
                }
                Err(broadcast::error::RecvError::Closed) => return None,
            }
        }
    }

    /// Tries to receive an event without blocking.
    ///
    /// Returns `None` if no event is available or the bus is closed.
    pub fn try_recv(&mut self) -> Option<DeploymentEventData> {
        loop {
            match self.receiver.try_recv() {
                Ok(event) => return Some(event),
                Err(broadcast::error::TryRecvError::Lagged(count)) => {
                    tracing::warn!("Event subscriber lagged, skipped {count} events");
                    continue;
                }
                Err(broadcast::error::TryRecvError::Empty) => return None,
                Err(broadcast::error::TryRecvError::Closed) => return None,
            }
        }
    }

    /// Filters events for a specific deployment.
    ///
    /// Returns a stream that only yields events for the specified deployment ID.
    pub fn filter_deployment(self, deployment_id: String) -> FilteredSubscriber {
        FilteredSubscriber {
            inner: self,
            deployment_id: Some(deployment_id),
            service: None,
        }
    }

    /// Filters events for a specific service.
    ///
    /// Returns a stream that only yields events for the specified service.
    pub fn filter_service(self, service: String) -> FilteredSubscriber {
        FilteredSubscriber {
            inner: self,
            deployment_id: None,
            service: Some(service),
        }
    }
}

/// Filtered event subscriber.
pub struct FilteredSubscriber {
    inner: EventSubscriber,
    deployment_id: Option<String>,
    service: Option<String>,
}

impl FilteredSubscriber {
    /// Receives the next matching event.
    pub async fn recv(&mut self) -> Option<DeploymentEventData> {
        loop {
            let event = self.inner.recv().await?;

            // Check deployment ID filter
            if let Some(ref id) = self.deployment_id {
                if &event.deployment_id != id {
                    continue;
                }
            }

            // Check service filter
            if let Some(ref svc) = self.service {
                if &event.service != svc {
                    continue;
                }
            }

            return Some(event);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_type_display() {
        assert_eq!(
            EventType::DeploymentStarted.to_string(),
            "deployment_started"
        );
        assert_eq!(
            EventType::RollbackCompleted.to_string(),
            "rollback_completed"
        );
    }

    #[test]
    fn test_deployment_event_data_started() {
        let event = DeploymentEventData::started("deploy-1", "users", "1.0.0");
        assert_eq!(event.deployment_id, "deploy-1");
        assert_eq!(event.service, "users");
        assert_eq!(event.version, "1.0.0");
        assert_eq!(event.event_type, EventType::DeploymentStarted);
        assert!(event.message.contains("started"));
    }

    #[test]
    fn test_deployment_event_data_completed() {
        let event = DeploymentEventData::completed("deploy-1", "users", "1.0.0");
        assert_eq!(event.event_type, EventType::DeploymentCompleted);
        assert_eq!(event.progress, Some(100));
    }

    #[test]
    fn test_deployment_event_data_instance_failed() {
        let event = DeploymentEventData::instance_failed(
            "deploy-1",
            "users",
            "1.0.0",
            "inst-1",
            "Connection timeout",
        );
        assert_eq!(event.event_type, EventType::InstanceUpdateFailed);
        assert_eq!(event.instance_id, Some("inst-1".to_string()));
        assert!(event.message.contains("timeout"));
    }

    #[test]
    fn test_deployment_event_data_with_metadata() {
        let event = DeploymentEventData::started("d-1", "svc", "1.0")
            .with_progress(50)
            .with_metadata(serde_json::json!({"batch": 2}));
        assert_eq!(event.progress, Some(50));
        assert!(event.metadata.is_some());
    }

    #[tokio::test]
    async fn test_event_bus_publish_subscribe() {
        let bus = EventBus::new(10);
        let mut sub = bus.subscribe();

        let event = DeploymentEventData::started("d-1", "users", "1.0.0");
        let count = bus.publish(event.clone());
        assert_eq!(count, 1);

        let received = sub.recv().await.unwrap();
        assert_eq!(received.deployment_id, "d-1");
    }

    #[tokio::test]
    async fn test_event_bus_multiple_subscribers() {
        let bus = EventBus::new(10);
        let mut sub1 = bus.subscribe();
        let mut sub2 = bus.subscribe();

        let event = DeploymentEventData::started("d-1", "users", "1.0.0");
        let count = bus.publish(event);
        assert_eq!(count, 2);

        let r1 = sub1.recv().await.unwrap();
        let r2 = sub2.recv().await.unwrap();
        assert_eq!(r1.deployment_id, r2.deployment_id);
    }

    #[test]
    fn test_event_bus_no_subscribers() {
        let bus = EventBus::new(10);
        let event = DeploymentEventData::started("d-1", "users", "1.0.0");
        let count = bus.publish(event);
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_filtered_subscriber_by_deployment() {
        let bus = EventBus::new(10);
        let sub = bus.subscribe();
        let mut filtered = sub.filter_deployment("deploy-2".to_string());

        // Publish events for different deployments
        tokio::spawn({
            let bus = bus.clone();
            async move {
                bus.publish(DeploymentEventData::started("deploy-1", "svc", "1.0"));
                bus.publish(DeploymentEventData::started("deploy-2", "svc", "1.0"));
                bus.publish(DeploymentEventData::completed("deploy-2", "svc", "1.0"));
            }
        });

        // Should only receive deploy-2 events
        let e1 = filtered.recv().await.unwrap();
        assert_eq!(e1.deployment_id, "deploy-2");
        assert_eq!(e1.event_type, EventType::DeploymentStarted);

        let e2 = filtered.recv().await.unwrap();
        assert_eq!(e2.deployment_id, "deploy-2");
        assert_eq!(e2.event_type, EventType::DeploymentCompleted);
    }

    #[test]
    fn test_event_bus_subscriber_count() {
        let bus = EventBus::new(10);
        assert_eq!(bus.subscriber_count(), 0);

        let _sub1 = bus.subscribe();
        assert_eq!(bus.subscriber_count(), 1);

        let _sub2 = bus.subscribe();
        assert_eq!(bus.subscriber_count(), 2);
    }

    #[test]
    fn test_rollback_event_data() {
        let event = DeploymentEventData::rollback_started("rb-1", "users", "2.0.0", "1.0.0");
        assert_eq!(event.event_type, EventType::RollbackStarted);
        assert_eq!(event.version, "1.0.0"); // target version
        assert!(event.metadata.is_some());
        let meta = event.metadata.unwrap();
        assert_eq!(meta["from_version"], "2.0.0");
    }
}
