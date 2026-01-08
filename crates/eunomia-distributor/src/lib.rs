// Clippy lint configuration for this crate.
// These pedantic lints require significant refactoring that doesn't provide value.
#![allow(clippy::must_use_candidate)]
#![allow(clippy::missing_const_for_fn)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::significant_drop_tightening)]
#![allow(clippy::unused_async)]
#![allow(clippy::return_self_not_must_use)]
#![allow(clippy::use_self)]
#![allow(clippy::option_if_let_else)]
#![allow(clippy::manual_unwrap_or_default)]
#![allow(clippy::needless_continue)]
#![allow(clippy::match_same_arms)]
#![allow(clippy::redundant_clone)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::unchecked_time_subtraction)]
#![allow(unused_results)]
#![allow(unused_imports)]

//! Eunomia Policy Distributor
//!
//! This crate provides the policy distribution functionality for Eunomia,
//! enabling push-based deployment of policy bundles to Archimedes instances.
//!
//! # Overview
//!
//! The distributor handles:
//! - **Push Distribution**: Actively pushes policy updates to Archimedes instances
//! - **Instance Discovery**: Discovers and tracks Archimedes instances (K8s, DNS, static)
//! - **Health Monitoring**: Monitors instance health and policy status
//! - **Deployment Strategies**: Supports immediate, canary, and rolling deployments
//! - **Rollback**: Automatic and manual rollback capabilities
//!
//! # Architecture
//!
//! ```text
//!                    ┌─────────────────────┐
//!                    │  Eunomia Control    │
//!                    │       Plane         │
//!                    └──────────┬──────────┘
//!                               │ gRPC
//!        ┌──────────────────────┼──────────────────────┐
//!        ▼                      ▼                      ▼
//! ┌─────────────┐       ┌─────────────┐       ┌─────────────┐
//! │ Archimedes  │       │ Archimedes  │       │ Archimedes  │
//! │ Instance 1  │       │ Instance 2  │       │ Instance N  │
//! └─────────────┘       └─────────────┘       └─────────────┘
//! ```
//!
//! # Example
//!
//! ```rust,ignore
//! use eunomia_distributor::{Distributor, DistributorConfig, DeploymentStrategy};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = DistributorConfig::default();
//!     let distributor = Distributor::new(config).await?;
//!     
//!     // Deploy a policy with canary strategy
//!     let result = distributor.deploy(
//!         "users-service",
//!         "1.2.0",
//!         DeploymentStrategy::canary(10, Duration::from_secs(300)),
//!     ).await?;
//!     
//!     println!("Deployed to {} instances", result.successful);
//!     Ok(())
//! }
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod config;
pub mod discovery;
pub mod error;
pub mod events;
pub mod grpc;
pub mod health;
pub mod instance;
pub mod pusher;
pub mod rollback;
pub mod scheduler;
pub mod state;
pub mod strategy;

use eunomia_metrics::MetricsRegistry;
use std::time::Instant;

// Re-export main types at crate root
pub use config::DistributorConfig;
pub use discovery::{Discovery, DiscoverySource, DnsDiscovery, StaticDiscovery};
pub use error::{DistributorError, Result};
pub use events::{DeploymentEventData, EventBus, EventSubscriber, EventType, FilteredSubscriber};
pub use health::{HealthCheck, HealthConfig, HealthState};
pub use instance::{Instance, InstanceId, InstanceMetadata, InstanceStatus};
pub use pusher::{PolicyPusher, PushConfig, PushResult};
pub use rollback::{
    RollbackConfig, RollbackController, RollbackResult, RollbackTrigger, VersionHistory,
};
pub use scheduler::{DeploymentScheduler, SchedulerConfig};
pub use state::{DeploymentState, DeploymentTracker};
pub use strategy::{DeploymentStrategy, StrategyType};

/// The main distributor service.
///
/// Coordinates policy distribution across multiple Archimedes instances
/// using configurable deployment strategies.
pub struct Distributor {
    #[allow(dead_code)]
    config: DistributorConfig,
    discovery: Box<dyn Discovery>,
    pusher: PolicyPusher,
    #[allow(dead_code)]
    scheduler: DeploymentScheduler,
    state: DeploymentTracker,
}

impl Distributor {
    /// Creates a new distributor with the given configuration.
    pub async fn new(config: DistributorConfig) -> Result<Self> {
        let discovery = config.create_discovery()?;
        let pusher = PolicyPusher::new(config.push_config.clone());
        let scheduler = DeploymentScheduler::new(config.scheduler_config.clone());
        let state = DeploymentTracker::new();

        Ok(Self {
            config,
            discovery,
            pusher,
            scheduler,
            state,
        })
    }

    /// Deploys a policy version to all discovered instances.
    ///
    /// # Arguments
    ///
    /// * `service` - Target service name
    /// * `version` - Policy version to deploy
    /// * `strategy` - Deployment strategy (immediate, canary, rolling)
    ///
    /// # Returns
    ///
    /// A deployment result containing success/failure counts and details.
    pub async fn deploy(
        &self,
        service: &str,
        version: &str,
        strategy: DeploymentStrategy,
    ) -> Result<DeploymentResult> {
        let start = Instant::now();
        let deployment_id = uuid::Uuid::now_v7().to_string();
        let strategy_type = strategy.strategy_type();

        tracing::info!(
            deployment_id = %deployment_id,
            service = %service,
            version = %version,
            strategy = ?strategy_type,
            "starting policy deployment"
        );

        // Record deployment start metric
        MetricsRegistry::global()
            .distributor()
            .record_deployment(service, &strategy_type.to_string());

        // Discover target instances
        let instances = self.discovery.discover(service).await?;
        if instances.is_empty() {
            return Err(DistributorError::NoInstancesFound {
                service: service.to_string(),
            });
        }

        tracing::info!(
            deployment_id = %deployment_id,
            instance_count = instances.len(),
            "discovered target instances"
        );

        // Track deployment state
        self.state
            .start_deployment(&deployment_id, service, version, instances.len())
            .await?;

        // Execute deployment based on strategy
        let result = match strategy_type {
            StrategyType::Immediate => {
                self.deploy_immediate(&deployment_id, service, version, &instances)
                    .await
            }
            StrategyType::Canary => {
                self.deploy_canary(&deployment_id, service, version, &instances, &strategy)
                    .await
            }
            StrategyType::Rolling => {
                self.deploy_rolling(&deployment_id, service, version, &instances, &strategy)
                    .await
            }
        };

        let duration_ms = start.elapsed().as_millis() as u64;

        // Update final state and record metrics
        match &result {
            Ok(r) => {
                self.state
                    .complete_deployment(&deployment_id, r.clone())
                    .await?;
                MetricsRegistry::global().distributor().record_push(
                    service,
                    version,
                    true,
                    duration_ms,
                );
            }
            Err(e) => {
                self.state
                    .fail_deployment(&deployment_id, e.to_string())
                    .await?;
                MetricsRegistry::global().distributor().record_push(
                    service,
                    version,
                    false,
                    duration_ms,
                );
            }
        }

        result
    }

    /// Rolls back a service to a previous policy version.
    pub async fn rollback(&self, service: &str, target_version: &str) -> Result<DeploymentResult> {
        let start = Instant::now();

        // Use immediate strategy for rollbacks
        let result = self
            .deploy(service, target_version, DeploymentStrategy::immediate())
            .await;

        let duration_ms = start.elapsed().as_millis() as u64;

        // Record rollback metric
        MetricsRegistry::global().distributor().record_rollback(
            service,
            result.is_ok(),
            duration_ms,
        );

        result
    }

    /// Gets the current deployment status for a service.
    pub async fn get_status(&self, service: &str) -> Result<ServiceStatus> {
        self.state.get_service_status(service).await
    }

    /// Lists all known instances for a service.
    pub async fn list_instances(&self, service: &str) -> Result<Vec<Instance>> {
        self.discovery.discover(service).await
    }

    // Private deployment methods

    async fn deploy_immediate(
        &self,
        deployment_id: &str,
        service: &str,
        version: &str,
        instances: &[Instance],
    ) -> Result<DeploymentResult> {
        let mut results = Vec::new();

        // Push to all instances in parallel
        let futures: Vec<_> = instances
            .iter()
            .map(|instance| self.pusher.push(instance, service, version))
            .collect();

        let push_results = futures::future::join_all(futures).await;

        for (instance, push_result) in instances.iter().zip(push_results) {
            results.push(InstanceResult {
                instance_id: instance.id.clone(),
                status: match push_result {
                    Ok(_) => InstanceResultStatus::Success,
                    Err(ref e) => InstanceResultStatus::Failed(e.to_string()),
                },
            });

            self.state
                .update_instance(deployment_id, &instance.id, push_result.is_ok())
                .await?;
        }

        Ok(DeploymentResult::from_results(deployment_id, results))
    }

    async fn deploy_canary(
        &self,
        deployment_id: &str,
        service: &str,
        version: &str,
        instances: &[Instance],
        strategy: &DeploymentStrategy,
    ) -> Result<DeploymentResult> {
        let canary_count = strategy.canary_count(instances.len());
        let (canary_instances, remaining) = instances.split_at(canary_count.min(instances.len()));

        tracing::info!(
            deployment_id = %deployment_id,
            canary_count = canary_instances.len(),
            remaining_count = remaining.len(),
            "deploying to canary instances first"
        );

        // Deploy to canary instances
        let canary_result = self
            .deploy_immediate(deployment_id, service, version, canary_instances)
            .await?;

        if !canary_result.is_fully_successful() {
            tracing::warn!(
                deployment_id = %deployment_id,
                failed_count = canary_result.failed,
                "canary deployment failed, aborting"
            );
            return Ok(canary_result);
        }

        // Wait for canary duration
        if let Some(duration) = strategy.canary_duration() {
            tracing::info!(
                deployment_id = %deployment_id,
                duration_secs = duration.as_secs(),
                "waiting for canary validation period"
            );
            tokio::time::sleep(duration).await;

            // Perform health checks
            for instance in canary_instances {
                let health = self.pusher.health_check(instance).await?;
                if health.state != HealthState::Healthy {
                    tracing::warn!(
                        deployment_id = %deployment_id,
                        instance_id = %instance.id,
                        "canary instance unhealthy, aborting"
                    );
                    return Ok(canary_result);
                }
            }
        }

        // Deploy to remaining instances
        let remaining_result = self
            .deploy_immediate(deployment_id, service, version, remaining)
            .await?;

        // Merge results
        Ok(canary_result.merge(remaining_result))
    }

    async fn deploy_rolling(
        &self,
        deployment_id: &str,
        service: &str,
        version: &str,
        instances: &[Instance],
        strategy: &DeploymentStrategy,
    ) -> Result<DeploymentResult> {
        let batch_size = strategy.batch_size().unwrap_or(1);
        let mut all_results = Vec::new();

        for (batch_num, batch) in instances.chunks(batch_size).enumerate() {
            tracing::info!(
                deployment_id = %deployment_id,
                batch = batch_num + 1,
                batch_size = batch.len(),
                "deploying batch"
            );

            let batch_result = self
                .deploy_immediate(deployment_id, service, version, batch)
                .await?;

            all_results.extend(batch_result.instance_results);

            // Check for failures
            if batch_result.failed > 0 {
                tracing::warn!(
                    deployment_id = %deployment_id,
                    batch = batch_num + 1,
                    "batch had failures, aborting rolling deployment"
                );
                break;
            }

            // Wait between batches
            if let Some(delay) = strategy.batch_delay() {
                tokio::time::sleep(delay).await;
            }
        }

        Ok(DeploymentResult::from_results(deployment_id, all_results))
    }
}

/// Result of a deployment operation.
#[derive(Debug, Clone)]
pub struct DeploymentResult {
    /// Unique deployment ID
    pub deployment_id: String,

    /// Number of successful instance updates
    pub successful: usize,

    /// Number of failed instance updates
    pub failed: usize,

    /// Number of skipped instances
    pub skipped: usize,

    /// Per-instance results
    pub instance_results: Vec<InstanceResult>,
}

impl DeploymentResult {
    /// Creates a result from individual instance results.
    pub fn from_results(deployment_id: &str, results: Vec<InstanceResult>) -> Self {
        let successful = results
            .iter()
            .filter(|r| matches!(r.status, InstanceResultStatus::Success))
            .count();
        let failed = results
            .iter()
            .filter(|r| matches!(r.status, InstanceResultStatus::Failed(_)))
            .count();
        let skipped = results
            .iter()
            .filter(|r| matches!(r.status, InstanceResultStatus::Skipped))
            .count();

        Self {
            deployment_id: deployment_id.to_string(),
            successful,
            failed,
            skipped,
            instance_results: results,
        }
    }

    /// Returns true if all instances were successfully updated.
    pub fn is_fully_successful(&self) -> bool {
        self.failed == 0 && self.skipped == 0
    }

    /// Merges two deployment results.
    pub fn merge(mut self, other: Self) -> Self {
        self.successful += other.successful;
        self.failed += other.failed;
        self.skipped += other.skipped;
        self.instance_results.extend(other.instance_results);
        self
    }
}

/// Result for a single instance.
#[derive(Debug, Clone)]
pub struct InstanceResult {
    /// Instance identifier
    pub instance_id: InstanceId,

    /// Result status
    pub status: InstanceResultStatus,
}

/// Status of an instance deployment.
#[derive(Debug, Clone)]
pub enum InstanceResultStatus {
    /// Successfully updated
    Success,

    /// Failed with error
    Failed(String),

    /// Skipped (e.g., already at target version)
    Skipped,
}

/// Status of a service's policy deployment.
#[derive(Debug, Clone)]
pub struct ServiceStatus {
    /// Service name
    pub service: String,

    /// Current deployed version
    pub current_version: Option<String>,

    /// Previous version (for rollback)
    pub previous_version: Option<String>,

    /// Current deployment state
    pub state: DeploymentState,

    /// Per-instance status
    pub instances: Vec<InstanceStatus>,
}
