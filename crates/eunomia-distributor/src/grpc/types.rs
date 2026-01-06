//! gRPC type definitions for the Control Plane.
//!
//! These types mirror the protobuf definitions in `proto/control_plane.proto`
//! for use with manual tonic implementation.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Deployment strategy type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(i32)]
pub enum GrpcStrategyType {
    /// Unknown strategy (default).
    Unknown = 0,
    /// Deploy to all instances immediately.
    Immediate = 1,
    /// Deploy to a subset first, then full rollout.
    Canary = 2,
    /// Deploy in batches with delays between.
    Rolling = 3,
}

impl From<i32> for GrpcStrategyType {
    fn from(value: i32) -> Self {
        match value {
            1 => Self::Immediate,
            2 => Self::Canary,
            3 => Self::Rolling,
            _ => Self::Unknown,
        }
    }
}

impl From<crate::StrategyType> for GrpcStrategyType {
    fn from(value: crate::StrategyType) -> Self {
        match value {
            crate::StrategyType::Immediate => Self::Immediate,
            crate::StrategyType::Canary => Self::Canary,
            crate::StrategyType::Rolling => Self::Rolling,
        }
    }
}

/// Deployment state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(i32)]
pub enum GrpcDeploymentState {
    /// Unknown state (default).
    Unknown = 0,
    /// Deployment is pending.
    Pending = 1,
    /// Deployment is in progress.
    InProgress = 2,
    /// Deployment completed successfully.
    Completed = 3,
    /// Deployment failed.
    Failed = 4,
    /// Deployment was rolled back.
    RolledBack = 5,
    /// Deployment was cancelled.
    Cancelled = 6,
}

impl From<i32> for GrpcDeploymentState {
    fn from(value: i32) -> Self {
        match value {
            1 => Self::Pending,
            2 => Self::InProgress,
            3 => Self::Completed,
            4 => Self::Failed,
            5 => Self::RolledBack,
            6 => Self::Cancelled,
            _ => Self::Unknown,
        }
    }
}

impl From<crate::DeploymentState> for GrpcDeploymentState {
    fn from(value: crate::DeploymentState) -> Self {
        match value {
            crate::DeploymentState::Pending => Self::Pending,
            crate::DeploymentState::InProgress => Self::InProgress,
            crate::DeploymentState::Completed => Self::Completed,
            crate::DeploymentState::Failed => Self::Failed,
            crate::DeploymentState::RolledBack => Self::RolledBack,
            crate::DeploymentState::Cancelled => Self::Cancelled,
        }
    }
}

/// Instance health state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(i32)]
pub enum GrpcHealthState {
    /// Unknown health state.
    Unknown = 0,
    /// Instance is healthy.
    Healthy = 1,
    /// Instance is unhealthy.
    Unhealthy = 2,
    /// Instance is degraded.
    Degraded = 3,
}

impl From<crate::HealthState> for GrpcHealthState {
    fn from(value: crate::HealthState) -> Self {
        match value {
            crate::HealthState::Unknown => Self::Unknown,
            crate::HealthState::Healthy => Self::Healthy,
            crate::HealthState::Unhealthy => Self::Unhealthy,
            crate::HealthState::Degraded => Self::Degraded,
            crate::HealthState::Unreachable => Self::Unhealthy,
        }
    }
}

/// Deployment strategy configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrpcDeploymentStrategy {
    /// Strategy type.
    pub strategy_type: GrpcStrategyType,
    /// Canary percentage (0-100).
    pub canary_percentage: i32,
    /// Rolling batch size.
    pub rolling_batch_size: i32,
    /// Delay between batches in seconds.
    pub batch_delay_seconds: i64,
    /// Whether to auto-rollback on failure.
    pub auto_rollback: bool,
    /// Maximum failures before rollback.
    pub max_failures: i32,
}

impl Default for GrpcDeploymentStrategy {
    fn default() -> Self {
        Self {
            strategy_type: GrpcStrategyType::Immediate,
            canary_percentage: 0,
            rolling_batch_size: 1,
            batch_delay_seconds: 0,
            auto_rollback: true,
            max_failures: 0,
        }
    }
}

/// Deploy policy request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeployPolicyRequest {
    /// Target service name.
    pub service: String,
    /// Version to deploy.
    pub version: String,
    /// Deployment strategy.
    pub strategy: Option<GrpcDeploymentStrategy>,
    /// Specific instance IDs to target.
    pub target_instances: Vec<String>,
    /// Reason for deployment.
    pub reason: String,
}

/// Deploy policy response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeployPolicyResponse {
    /// Unique deployment ID.
    pub deployment_id: String,
    /// Current deployment state.
    pub state: GrpcDeploymentState,
    /// Summary of results.
    pub summary: Option<DeploymentSummary>,
    /// Per-instance results.
    pub instance_results: Vec<InstanceDeploymentResult>,
}

/// Deployment summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentSummary {
    /// Total instances targeted.
    pub total_instances: i32,
    /// Successfully updated instances.
    pub successful: i32,
    /// Failed instances.
    pub failed: i32,
    /// Skipped instances.
    pub skipped: i32,
    /// Deployment duration in milliseconds.
    pub duration_ms: i64,
}

/// Per-instance deployment result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceDeploymentResult {
    /// Instance ID.
    pub instance_id: String,
    /// Whether the deployment succeeded.
    pub success: bool,
    /// Error message (if failed).
    pub error_message: String,
    /// Previous policy version.
    pub previous_version: String,
    /// Duration for this instance in milliseconds.
    pub duration_ms: i64,
}

/// Rollback policy request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackPolicyRequest {
    /// Target service.
    pub service: String,
    /// Target version to rollback to.
    pub target_version: String,
    /// Specific instance IDs.
    pub target_instances: Vec<String>,
    /// Reason for rollback.
    pub reason: String,
}

/// Rollback policy response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackPolicyResponse {
    /// Deployment ID for the rollback.
    pub deployment_id: String,
    /// Current state.
    pub state: GrpcDeploymentState,
    /// Summary.
    pub summary: Option<DeploymentSummary>,
    /// Per-instance results.
    pub instance_results: Vec<InstanceDeploymentResult>,
}

/// Get policy status request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetPolicyStatusRequest {
    /// Service name.
    pub service: String,
}

/// Policy status response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyStatusResponse {
    /// Service name.
    pub service: String,
    /// Current active version.
    pub current_version: String,
    /// Deployment state.
    pub state: GrpcDeploymentState,
    /// Last deployment time.
    pub last_deployment_time: Option<DateTime<Utc>>,
    /// Per-instance status.
    pub instance_statuses: Vec<InstancePolicyStatus>,
}

/// Instance policy status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstancePolicyStatus {
    /// Instance ID.
    pub instance_id: String,
    /// Policy version on this instance.
    pub version: String,
    /// Health state.
    pub health: GrpcHealthState,
    /// Last update time.
    pub last_updated: Option<DateTime<Utc>>,
}

/// List instances request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListInstancesRequest {
    /// Filter by service (optional).
    pub service_filter: String,
    /// Filter by health state (optional).
    pub health_filter: Option<GrpcHealthState>,
}

/// List instances response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListInstancesResponse {
    /// List of instances.
    pub instances: Vec<InstanceInfo>,
}

/// Instance information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceInfo {
    /// Instance ID.
    pub instance_id: String,
    /// Instance endpoint (host:port).
    pub endpoint: String,
    /// Services this instance handles.
    pub services: Vec<String>,
    /// Health state.
    pub health: GrpcHealthState,
    /// Current policy version.
    pub policy_version: String,
    /// Instance metadata (labels).
    pub metadata: std::collections::HashMap<String, String>,
}

/// Get instance health request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetInstanceHealthRequest {
    /// Instance ID.
    pub instance_id: String,
}

/// Instance health response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceHealthResponse {
    /// Instance ID.
    pub instance_id: String,
    /// Health state.
    pub health: GrpcHealthState,
    /// Last check time.
    pub last_check: Option<DateTime<Utc>>,
    /// Error message if unhealthy.
    pub error_message: String,
}

/// Watch deployment request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchDeploymentRequest {
    /// Deployment ID to watch.
    pub deployment_id: String,
}

/// Deployment event (streamed).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentEvent {
    /// Deployment ID.
    pub deployment_id: String,
    /// Event type.
    pub event_type: DeploymentEventType,
    /// Event timestamp.
    pub timestamp: DateTime<Utc>,
    /// Instance ID (if instance-specific).
    pub instance_id: String,
    /// Event message.
    pub message: String,
}

/// Deployment event types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(i32)]
pub enum DeploymentEventType {
    /// Unknown event.
    Unknown = 0,
    /// Deployment started.
    Started = 1,
    /// Instance update started.
    InstanceStarted = 2,
    /// Instance update completed.
    InstanceCompleted = 3,
    /// Instance update failed.
    InstanceFailed = 4,
    /// Batch completed (rolling).
    BatchCompleted = 5,
    /// Canary validation started.
    CanaryValidationStarted = 6,
    /// Canary validation passed.
    CanaryValidationPassed = 7,
    /// Deployment completed.
    Completed = 8,
    /// Deployment failed.
    Failed = 9,
    /// Rollback started.
    RollbackStarted = 10,
}

// === Policy Receiver (Archimedes side) messages ===

/// Update policy request (sent to Archimedes).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePolicyRequest {
    /// Service name.
    pub service: String,
    /// New policy version.
    pub version: String,
    /// Policy bundle (serialized).
    pub bundle: Vec<u8>,
    /// Bundle checksum.
    pub checksum: String,
    /// Whether to force update (bypass version check).
    pub force: bool,
}

/// Update policy response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePolicyResponse {
    /// Whether the update succeeded.
    pub success: bool,
    /// Previous policy version.
    pub previous_version: String,
    /// Error message if failed.
    pub error_message: String,
}

/// Get current policy request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetCurrentPolicyRequest {
    /// Service name.
    pub service: String,
}

/// Current policy response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentPolicyResponse {
    /// Service name.
    pub service: String,
    /// Current version.
    pub version: String,
    /// Bundle checksum.
    pub checksum: String,
    /// Load time.
    pub loaded_at: Option<DateTime<Utc>>,
}

/// Health check request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckRequest {
    /// Service to check (empty = all).
    pub service: String,
}

/// Health check response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResponse {
    /// Overall health state.
    pub status: GrpcHealthState,
    /// Message.
    pub message: String,
    /// Per-service status.
    pub service_statuses: std::collections::HashMap<String, ServiceHealthStatus>,
}

/// Per-service health status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceHealthStatus {
    /// Health state.
    pub status: GrpcHealthState,
    /// Policy version.
    pub policy_version: String,
    /// Last evaluation time.
    pub last_evaluation: Option<DateTime<Utc>>,
    /// Error count in last minute.
    pub error_count: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strategy_type_conversion() {
        assert_eq!(GrpcStrategyType::from(1), GrpcStrategyType::Immediate);
        assert_eq!(GrpcStrategyType::from(2), GrpcStrategyType::Canary);
        assert_eq!(GrpcStrategyType::from(3), GrpcStrategyType::Rolling);
        assert_eq!(GrpcStrategyType::from(99), GrpcStrategyType::Unknown);
    }

    #[test]
    fn test_deployment_state_conversion() {
        assert_eq!(GrpcDeploymentState::from(1), GrpcDeploymentState::Pending);
        assert_eq!(GrpcDeploymentState::from(3), GrpcDeploymentState::Completed);
        assert_eq!(GrpcDeploymentState::from(4), GrpcDeploymentState::Failed);
        assert_eq!(GrpcDeploymentState::from(99), GrpcDeploymentState::Unknown);
    }

    #[test]
    fn test_default_strategy() {
        let strategy = GrpcDeploymentStrategy::default();
        assert_eq!(strategy.strategy_type, GrpcStrategyType::Immediate);
        assert!(strategy.auto_rollback);
    }

    #[test]
    fn test_deploy_request_serialization() {
        let request = DeployPolicyRequest {
            service: "users".to_string(),
            version: "1.0.0".to_string(),
            strategy: None,
            target_instances: vec![],
            reason: "test".to_string(),
        };
        
        let json = serde_json::to_string(&request).unwrap();
        let parsed: DeployPolicyRequest = serde_json::from_str(&json).unwrap();
        
        assert_eq!(parsed.service, "users");
        assert_eq!(parsed.version, "1.0.0");
    }
}
