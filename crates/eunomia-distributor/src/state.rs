//! Deployment state tracking.
//!
//! This module tracks the state of ongoing and completed deployments.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;

use crate::error::{DistributorError, Result};
use crate::DeploymentResult;

/// State of a deployment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeploymentState {
    /// Deployment has not started.
    Pending,

    /// Deployment is in progress.
    InProgress,

    /// Deployment completed successfully.
    Completed,

    /// Deployment failed.
    Failed,

    /// Deployment was rolled back.
    RolledBack,

    /// Deployment was cancelled.
    Cancelled,
}

impl DeploymentState {
    /// Returns true if the deployment is terminal (no more changes expected).
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Completed | Self::Failed | Self::RolledBack | Self::Cancelled
        )
    }

    /// Returns a string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::InProgress => "in_progress",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::RolledBack => "rolled_back",
            Self::Cancelled => "cancelled",
        }
    }
}

impl std::fmt::Display for DeploymentState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Information about a tracked deployment.
#[derive(Debug, Clone)]
pub struct DeploymentInfo {
    /// Unique deployment ID.
    pub id: String,

    /// Service name.
    pub service: String,

    /// Target version.
    pub version: String,

    /// Current state.
    pub state: DeploymentState,

    /// Total instance count.
    pub total_instances: usize,

    /// Number of successful instances.
    pub successful: usize,

    /// Number of failed instances.
    pub failed: usize,

    /// Per-instance status.
    pub instance_status: HashMap<String, InstanceDeploymentStatus>,

    /// When the deployment started.
    pub started_at: Instant,

    /// When the deployment ended (if terminal).
    pub ended_at: Option<Instant>,

    /// Error message (if failed).
    pub error: Option<String>,
}

/// Status of a deployment for a single instance.
#[derive(Debug, Clone)]
pub struct InstanceDeploymentStatus {
    /// Instance ID.
    pub instance_id: String,

    /// Whether deployment was successful.
    pub success: bool,

    /// Error message if failed.
    pub error: Option<String>,

    /// When this instance was updated.
    pub updated_at: Instant,
}

/// Tracks the state of multiple deployments.
pub struct DeploymentTracker {
    /// Active and recent deployments.
    deployments: Arc<RwLock<HashMap<String, DeploymentInfo>>>,

    /// Service to current deployment mapping.
    service_deployments: Arc<RwLock<HashMap<String, String>>>,

    /// Maximum number of completed deployments to keep.
    max_history: usize,
}

impl DeploymentTracker {
    /// Creates a new deployment tracker.
    pub fn new() -> Self {
        Self {
            deployments: Arc::new(RwLock::new(HashMap::new())),
            service_deployments: Arc::new(RwLock::new(HashMap::new())),
            max_history: 100,
        }
    }

    /// Creates a tracker with custom history limit.
    pub fn with_history_limit(max_history: usize) -> Self {
        Self {
            deployments: Arc::new(RwLock::new(HashMap::new())),
            service_deployments: Arc::new(RwLock::new(HashMap::new())),
            max_history,
        }
    }

    /// Starts tracking a new deployment.
    pub async fn start_deployment(
        &self,
        deployment_id: &str,
        service: &str,
        version: &str,
        total_instances: usize,
    ) -> Result<()> {
        // Check for existing deployment
        let service_deployments = self.service_deployments.read().await;
        if let Some(existing_id) = service_deployments.get(service) {
            let deployments = self.deployments.read().await;
            if let Some(existing) = deployments.get(existing_id) {
                if !existing.state.is_terminal() {
                    return Err(DistributorError::DeploymentInProgress {
                        service: service.to_string(),
                        deployment_id: existing_id.clone(),
                    });
                }
            }
        }
        drop(service_deployments);

        // Create new deployment
        let info = DeploymentInfo {
            id: deployment_id.to_string(),
            service: service.to_string(),
            version: version.to_string(),
            state: DeploymentState::InProgress,
            total_instances,
            successful: 0,
            failed: 0,
            instance_status: HashMap::new(),
            started_at: Instant::now(),
            ended_at: None,
            error: None,
        };

        let mut deployments = self.deployments.write().await;
        deployments.insert(deployment_id.to_string(), info);

        let mut service_deployments = self.service_deployments.write().await;
        service_deployments.insert(service.to_string(), deployment_id.to_string());

        Ok(())
    }

    /// Updates the status of an instance in a deployment.
    pub async fn update_instance(
        &self,
        deployment_id: &str,
        instance_id: &str,
        success: bool,
    ) -> Result<()> {
        let mut deployments = self.deployments.write().await;
        let info = deployments.get_mut(deployment_id).ok_or_else(|| {
            DistributorError::DeploymentNotFound {
                deployment_id: deployment_id.to_string(),
            }
        })?;

        let status = InstanceDeploymentStatus {
            instance_id: instance_id.to_string(),
            success,
            error: None,
            updated_at: Instant::now(),
        };

        info.instance_status.insert(instance_id.to_string(), status);

        if success {
            info.successful += 1;
        } else {
            info.failed += 1;
        }

        Ok(())
    }

    /// Marks a deployment as completed.
    pub async fn complete_deployment(
        &self,
        deployment_id: &str,
        result: DeploymentResult,
    ) -> Result<()> {
        let mut deployments = self.deployments.write().await;
        let info = deployments.get_mut(deployment_id).ok_or_else(|| {
            DistributorError::DeploymentNotFound {
                deployment_id: deployment_id.to_string(),
            }
        })?;

        info.state = if result.failed == 0 {
            DeploymentState::Completed
        } else {
            DeploymentState::Failed
        };
        info.successful = result.successful;
        info.failed = result.failed;
        info.ended_at = Some(Instant::now());

        self.cleanup_old_deployments(&mut deployments);

        Ok(())
    }

    /// Marks a deployment as failed.
    pub async fn fail_deployment(&self, deployment_id: &str, error: String) -> Result<()> {
        let mut deployments = self.deployments.write().await;
        let info = deployments.get_mut(deployment_id).ok_or_else(|| {
            DistributorError::DeploymentNotFound {
                deployment_id: deployment_id.to_string(),
            }
        })?;

        info.state = DeploymentState::Failed;
        info.error = Some(error);
        info.ended_at = Some(Instant::now());

        Ok(())
    }

    /// Gets the status of a specific deployment.
    pub async fn get_deployment(&self, deployment_id: &str) -> Result<DeploymentInfo> {
        let deployments = self.deployments.read().await;
        deployments.get(deployment_id).cloned().ok_or_else(|| {
            DistributorError::DeploymentNotFound {
                deployment_id: deployment_id.to_string(),
            }
        })
    }

    /// Gets the current status for a service.
    pub async fn get_service_status(&self, service: &str) -> Result<crate::ServiceStatus> {
        let service_deployments = self.service_deployments.read().await;
        let deployment_id = service_deployments.get(service);

        let (state, current_version) = if let Some(id) = deployment_id {
            let deployments = self.deployments.read().await;
            deployments
                .get(id)
                .map_or((DeploymentState::Pending, None), |info| {
                    (info.state.clone(), Some(info.version.clone()))
                })
        } else {
            (DeploymentState::Pending, None)
        };

        Ok(crate::ServiceStatus {
            service: service.to_string(),
            current_version,
            previous_version: None, // Would track this in a more complete implementation
            state,
            instances: Vec::new(), // Would populate from discovery
        })
    }

    /// Lists all active (non-terminal) deployments.
    pub async fn list_active(&self) -> Vec<DeploymentInfo> {
        let deployments = self.deployments.read().await;
        deployments
            .values()
            .filter(|d| !d.state.is_terminal())
            .cloned()
            .collect()
    }

    /// Cancels a deployment.
    pub async fn cancel_deployment(&self, deployment_id: &str) -> Result<()> {
        let mut deployments = self.deployments.write().await;
        let info = deployments.get_mut(deployment_id).ok_or_else(|| {
            DistributorError::DeploymentNotFound {
                deployment_id: deployment_id.to_string(),
            }
        })?;

        if info.state.is_terminal() {
            return Err(DistributorError::StateError {
                reason: format!("deployment {deployment_id} is already terminal"),
            });
        }

        info.state = DeploymentState::Cancelled;
        info.ended_at = Some(Instant::now());

        Ok(())
    }

    fn cleanup_old_deployments(&self, deployments: &mut HashMap<String, DeploymentInfo>) {
        if deployments.len() <= self.max_history {
            return;
        }

        // Remove oldest completed deployments
        let mut completed: Vec<_> = deployments
            .iter()
            .filter(|(_, d)| d.state.is_terminal())
            .map(|(id, d)| (id.clone(), d.started_at))
            .collect();

        completed.sort_by(|a, b| a.1.cmp(&b.1));

        let to_remove = completed.len().saturating_sub(self.max_history);
        for (id, _) in completed.into_iter().take(to_remove) {
            deployments.remove(&id);
        }
    }
}

impl Default for DeploymentTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deployment_state_is_terminal() {
        assert!(!DeploymentState::Pending.is_terminal());
        assert!(!DeploymentState::InProgress.is_terminal());
        assert!(DeploymentState::Completed.is_terminal());
        assert!(DeploymentState::Failed.is_terminal());
        assert!(DeploymentState::RolledBack.is_terminal());
        assert!(DeploymentState::Cancelled.is_terminal());
    }

    #[test]
    fn test_deployment_state_display() {
        assert_eq!(DeploymentState::Pending.to_string(), "pending");
        assert_eq!(DeploymentState::InProgress.to_string(), "in_progress");
        assert_eq!(DeploymentState::Completed.to_string(), "completed");
        assert_eq!(DeploymentState::Failed.to_string(), "failed");
    }

    #[tokio::test]
    async fn test_tracker_start_deployment() {
        let tracker = DeploymentTracker::new();

        let result = tracker
            .start_deployment("deploy-1", "my-service", "1.0.0", 3)
            .await;
        assert!(result.is_ok());

        let info = tracker.get_deployment("deploy-1").await.unwrap();
        assert_eq!(info.service, "my-service");
        assert_eq!(info.version, "1.0.0");
        assert_eq!(info.total_instances, 3);
        assert_eq!(info.state, DeploymentState::InProgress);
    }

    #[tokio::test]
    async fn test_tracker_prevents_concurrent_deployments() {
        let tracker = DeploymentTracker::new();

        // Start first deployment
        tracker
            .start_deployment("deploy-1", "my-service", "1.0.0", 3)
            .await
            .unwrap();

        // Try to start second deployment for same service
        let result = tracker
            .start_deployment("deploy-2", "my-service", "2.0.0", 3)
            .await;

        assert!(result.is_err());
        if let Err(DistributorError::DeploymentInProgress { service, .. }) = result {
            assert_eq!(service, "my-service");
        } else {
            panic!("Expected DeploymentInProgress error");
        }
    }

    #[tokio::test]
    async fn test_tracker_update_instance() {
        let tracker = DeploymentTracker::new();

        tracker
            .start_deployment("deploy-1", "my-service", "1.0.0", 2)
            .await
            .unwrap();

        tracker
            .update_instance("deploy-1", "inst-1", true)
            .await
            .unwrap();

        let info = tracker.get_deployment("deploy-1").await.unwrap();
        assert_eq!(info.successful, 1);
        assert_eq!(info.failed, 0);

        tracker
            .update_instance("deploy-1", "inst-2", false)
            .await
            .unwrap();

        let info = tracker.get_deployment("deploy-1").await.unwrap();
        assert_eq!(info.successful, 1);
        assert_eq!(info.failed, 1);
    }

    #[tokio::test]
    async fn test_tracker_complete_deployment() {
        let tracker = DeploymentTracker::new();

        tracker
            .start_deployment("deploy-1", "my-service", "1.0.0", 2)
            .await
            .unwrap();

        let result = crate::DeploymentResult {
            deployment_id: "deploy-1".to_string(),
            successful: 2,
            failed: 0,
            skipped: 0,
            instance_results: Vec::new(),
        };

        tracker
            .complete_deployment("deploy-1", result)
            .await
            .unwrap();

        let info = tracker.get_deployment("deploy-1").await.unwrap();
        assert_eq!(info.state, DeploymentState::Completed);
        assert!(info.ended_at.is_some());
    }

    #[tokio::test]
    async fn test_tracker_fail_deployment() {
        let tracker = DeploymentTracker::new();

        tracker
            .start_deployment("deploy-1", "my-service", "1.0.0", 2)
            .await
            .unwrap();

        tracker
            .fail_deployment("deploy-1", "connection error".to_string())
            .await
            .unwrap();

        let info = tracker.get_deployment("deploy-1").await.unwrap();
        assert_eq!(info.state, DeploymentState::Failed);
        assert_eq!(info.error, Some("connection error".to_string()));
    }

    #[tokio::test]
    async fn test_tracker_cancel_deployment() {
        let tracker = DeploymentTracker::new();

        tracker
            .start_deployment("deploy-1", "my-service", "1.0.0", 2)
            .await
            .unwrap();

        tracker.cancel_deployment("deploy-1").await.unwrap();

        let info = tracker.get_deployment("deploy-1").await.unwrap();
        assert_eq!(info.state, DeploymentState::Cancelled);
    }

    #[tokio::test]
    async fn test_tracker_list_active() {
        let tracker = DeploymentTracker::new();

        // Start two deployments
        tracker
            .start_deployment("deploy-1", "service-1", "1.0.0", 2)
            .await
            .unwrap();
        tracker
            .start_deployment("deploy-2", "service-2", "1.0.0", 2)
            .await
            .unwrap();

        // Complete one
        let result = crate::DeploymentResult {
            deployment_id: "deploy-1".to_string(),
            successful: 2,
            failed: 0,
            skipped: 0,
            instance_results: Vec::new(),
        };
        tracker
            .complete_deployment("deploy-1", result)
            .await
            .unwrap();

        // Only deploy-2 should be active
        let active = tracker.list_active().await;
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].id, "deploy-2");
    }

    #[tokio::test]
    async fn test_tracker_get_service_status() {
        let tracker = DeploymentTracker::new();

        tracker
            .start_deployment("deploy-1", "my-service", "1.0.0", 2)
            .await
            .unwrap();

        let status = tracker.get_service_status("my-service").await.unwrap();
        assert_eq!(status.service, "my-service");
        assert_eq!(status.current_version, Some("1.0.0".to_string()));
        assert_eq!(status.state, DeploymentState::InProgress);
    }

    #[tokio::test]
    async fn test_tracker_allows_deployment_after_previous_completes() {
        let tracker = DeploymentTracker::new();

        // First deployment
        tracker
            .start_deployment("deploy-1", "my-service", "1.0.0", 2)
            .await
            .unwrap();

        // Complete it
        let result = crate::DeploymentResult {
            deployment_id: "deploy-1".to_string(),
            successful: 2,
            failed: 0,
            skipped: 0,
            instance_results: Vec::new(),
        };
        tracker
            .complete_deployment("deploy-1", result)
            .await
            .unwrap();

        // Second deployment should now work
        let result = tracker
            .start_deployment("deploy-2", "my-service", "2.0.0", 2)
            .await;
        assert!(result.is_ok());
    }
}
