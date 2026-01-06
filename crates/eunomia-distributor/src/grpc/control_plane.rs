//! gRPC Control Plane service implementation.
//!
//! This service handles policy deployment, rollback, and monitoring
//! operations for the control plane.

use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use futures::Stream;
use tonic::{Request, Response, Status};
use tracing::{debug, info, instrument, warn};

use super::types::*;
use crate::{DeploymentState, DeploymentStrategy, Distributor, HealthState};

/// Control Plane gRPC service implementation.
#[derive(Clone)]
pub struct ControlPlaneService {
    distributor: Arc<Distributor>,
}

impl std::fmt::Debug for ControlPlaneService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ControlPlaneService").finish_non_exhaustive()
    }
}

impl ControlPlaneService {
    /// Create a new Control Plane service.
    pub fn new(distributor: Arc<Distributor>) -> Self {
        Self { distributor }
    }

    /// Convert to a tonic service.
    pub fn into_service(self) -> ControlPlaneServiceServer<Self> {
        ControlPlaneServiceServer::new(self)
    }
}

/// Manual tonic service implementation.
///
/// This matches the protobuf service definition in `proto/control_plane.proto`.
#[tonic::async_trait]
impl ControlPlane for ControlPlaneService {
    #[instrument(skip(self, request), fields(service = %request.get_ref().service))]
    async fn deploy_policy(
        &self,
        request: Request<DeployPolicyRequest>,
    ) -> Result<Response<DeployPolicyResponse>, Status> {
        let req = request.into_inner();
        info!(
            "DeployPolicy request: service={}, version={}",
            req.service, req.version
        );

        // Convert gRPC strategy to internal strategy
        let strategy = req.strategy.map_or_else(DeploymentStrategy::immediate, |s| {
            match s.strategy_type {
                GrpcStrategyType::Canary => DeploymentStrategy::canary(
                    s.canary_percentage as u32,
                    Duration::from_secs(s.batch_delay_seconds as u64),
                )
                .with_max_failures(s.max_failures as u32)
                .with_auto_rollback(s.auto_rollback),
                GrpcStrategyType::Rolling => DeploymentStrategy::rolling(
                    s.rolling_batch_size as usize,
                    Duration::from_secs(s.batch_delay_seconds as u64),
                )
                .with_max_failures(s.max_failures as u32)
                .with_auto_rollback(s.auto_rollback),
                _ => DeploymentStrategy::immediate(),
            }
        });

        // Execute deployment
        let result = self
            .distributor
            .deploy(&req.service, &req.version, strategy)
            .await;

        match result {
            Ok(status) => {
                let response = DeployPolicyResponse {
                    deployment_id: status.deployment_id,
                    state: GrpcDeploymentState::Completed,
                    summary: Some(DeploymentSummary {
                        total_instances: (status.successful + status.failed + status.skipped)
                            as i32,
                        successful: status.successful as i32,
                        failed: status.failed as i32,
                        skipped: status.skipped as i32,
                        duration_ms: 0, // TODO: Track duration
                    }),
                    instance_results: status
                        .instance_results
                        .into_iter()
                        .map(|r| InstanceDeploymentResult {
                            instance_id: r.instance_id.to_string(),
                            success: matches!(
                                r.status,
                                crate::InstanceResultStatus::Success
                            ),
                            error_message: match r.status {
                                crate::InstanceResultStatus::Failed(e) => e,
                                _ => String::new(),
                            },
                            previous_version: String::new(), // TODO: Track this
                            duration_ms: 0,
                        })
                        .collect(),
                };
                Ok(Response::new(response))
            }
            Err(e) => {
                warn!("Deploy policy failed: {}", e);
                Err(Status::internal(format!("Deployment failed: {}", e)))
            }
        }
    }

    #[instrument(skip(self, request), fields(service = %request.get_ref().service))]
    async fn rollback_policy(
        &self,
        request: Request<RollbackPolicyRequest>,
    ) -> Result<Response<RollbackPolicyResponse>, Status> {
        let req = request.into_inner();
        info!(
            "RollbackPolicy request: service={}, target_version={}",
            req.service, req.target_version
        );

        let result = self
            .distributor
            .rollback(&req.service, &req.target_version)
            .await;

        match result {
            Ok(status) => {
                let response = RollbackPolicyResponse {
                    deployment_id: status.deployment_id,
                    state: GrpcDeploymentState::Completed,
                    summary: Some(DeploymentSummary {
                        total_instances: (status.successful + status.failed + status.skipped)
                            as i32,
                        successful: status.successful as i32,
                        failed: status.failed as i32,
                        skipped: status.skipped as i32,
                        duration_ms: 0,
                    }),
                    instance_results: status
                        .instance_results
                        .into_iter()
                        .map(|r| InstanceDeploymentResult {
                            instance_id: r.instance_id.to_string(),
                            success: matches!(
                                r.status,
                                crate::InstanceResultStatus::Success
                            ),
                            error_message: match r.status {
                                crate::InstanceResultStatus::Failed(e) => e,
                                _ => String::new(),
                            },
                            previous_version: String::new(),
                            duration_ms: 0,
                        })
                        .collect(),
                };
                Ok(Response::new(response))
            }
            Err(e) => {
                warn!("Rollback policy failed: {}", e);
                Err(Status::internal(format!("Rollback failed: {}", e)))
            }
        }
    }

    #[instrument(skip(self, request), fields(service = %request.get_ref().service))]
    async fn get_policy_status(
        &self,
        request: Request<GetPolicyStatusRequest>,
    ) -> Result<Response<PolicyStatusResponse>, Status> {
        let req = request.into_inner();
        debug!("GetPolicyStatus request: service={}", req.service);

        let status = self
            .distributor
            .get_status(&req.service)
            .await
            .map_err(|e| Status::internal(format!("Failed to get status: {}", e)))?;

        let state = match status.state {
            DeploymentState::Pending => GrpcDeploymentState::Pending,
            DeploymentState::InProgress => GrpcDeploymentState::InProgress,
            DeploymentState::Completed => GrpcDeploymentState::Completed,
            DeploymentState::Failed => GrpcDeploymentState::Failed,
            DeploymentState::RolledBack => GrpcDeploymentState::RolledBack,
            DeploymentState::Cancelled => GrpcDeploymentState::Cancelled,
        };

        // Get instances for the service to populate status
        let instances = self
            .distributor
            .list_instances(&req.service)
            .await
            .unwrap_or_default();

        let instance_statuses: Vec<InstancePolicyStatus> = instances
            .iter()
            .map(|inst| {
                let health = match inst.status.to_health_state() {
                    HealthState::Unknown => GrpcHealthState::Unknown,
                    HealthState::Healthy => GrpcHealthState::Healthy,
                    HealthState::Unhealthy => GrpcHealthState::Unhealthy,
                    HealthState::Degraded => GrpcHealthState::Degraded,
                    HealthState::Unreachable => GrpcHealthState::Unhealthy,
                };
                InstancePolicyStatus {
                    instance_id: inst.id.to_string(),
                    version: inst.status.policy_version().unwrap_or_default().to_string(),
                    health,
                    last_updated: None,
                }
            })
            .collect();

        let response = PolicyStatusResponse {
            service: req.service,
            current_version: status.current_version.unwrap_or_default(),
            state,
            last_deployment_time: None,
            instance_statuses,
        };

        Ok(Response::new(response))
    }

    #[instrument(skip(self, request))]
    async fn list_instances(
        &self,
        request: Request<ListInstancesRequest>,
    ) -> Result<Response<ListInstancesResponse>, Status> {
        let req = request.into_inner();
        debug!(
            "ListInstances request: service_filter={:?}",
            req.service_filter
        );

        let instances = if req.service_filter.is_empty() {
            // For now, empty filter returns empty list
            vec![]
        } else {
            self.distributor
                .list_instances(&req.service_filter)
                .await
                .map_err(|e| Status::internal(format!("Failed to list instances: {}", e)))?
        };

        // Filter by health if specified
        let instances: Vec<_> = if let Some(health_filter) = req.health_filter {
            let target_health = match health_filter {
                GrpcHealthState::Healthy => HealthState::Healthy,
                GrpcHealthState::Unhealthy => HealthState::Unhealthy,
                GrpcHealthState::Degraded => HealthState::Degraded,
                GrpcHealthState::Unknown => HealthState::Unknown,
            };
            instances
                .into_iter()
                .filter(|i| i.status.to_health_state() == target_health)
                .collect()
        } else {
            instances
        };

        let response = ListInstancesResponse {
            instances: instances
                .iter()
                .map(|inst| {
                    let health = match inst.status.to_health_state() {
                        HealthState::Unknown => GrpcHealthState::Unknown,
                        HealthState::Healthy => GrpcHealthState::Healthy,
                        HealthState::Unhealthy => GrpcHealthState::Unhealthy,
                        HealthState::Degraded => GrpcHealthState::Degraded,
                        HealthState::Unreachable => GrpcHealthState::Unhealthy,
                    };
                    InstanceInfo {
                        instance_id: inst.id.to_string(),
                        endpoint: format!("{}:{}", inst.endpoint.host, inst.endpoint.port),
                        services: inst.metadata.service.clone().into_iter().collect(),
                        health,
                        policy_version: inst.status.policy_version().unwrap_or_default().to_string(),
                        metadata: inst.metadata.labels.clone(),
                    }
                })
                .collect(),
        };

        Ok(Response::new(response))
    }

    #[instrument(skip(self, request), fields(instance_id = %request.get_ref().instance_id))]
    async fn get_instance_health(
        &self,
        request: Request<GetInstanceHealthRequest>,
    ) -> Result<Response<InstanceHealthResponse>, Status> {
        let req = request.into_inner();
        debug!("GetInstanceHealth request: instance_id={}", req.instance_id);

        // For now, we don't have a direct get_instance method
        Err(Status::unimplemented("get_instance_health not yet implemented"))
    }

    type WatchDeploymentStream =
        Pin<Box<dyn Stream<Item = Result<DeploymentEvent, Status>> + Send>>;

    #[instrument(skip(self, request), fields(deployment_id = %request.get_ref().deployment_id))]
    async fn watch_deployment(
        &self,
        request: Request<WatchDeploymentRequest>,
    ) -> Result<Response<Self::WatchDeploymentStream>, Status> {
        let req = request.into_inner();
        info!("WatchDeployment request: deployment_id={}", req.deployment_id);

        // For now, return an empty stream
        // TODO: Implement deployment event streaming
        let stream = futures::stream::empty();

        Ok(Response::new(Box::pin(stream)))
    }
}

/// Trait definition matching the protobuf service.
#[tonic::async_trait]
pub trait ControlPlane: Send + Sync + 'static {
    /// Deploy a policy to instances.
    async fn deploy_policy(
        &self,
        request: Request<DeployPolicyRequest>,
    ) -> Result<Response<DeployPolicyResponse>, Status>;

    /// Rollback a policy to a previous version.
    async fn rollback_policy(
        &self,
        request: Request<RollbackPolicyRequest>,
    ) -> Result<Response<RollbackPolicyResponse>, Status>;

    /// Get the current policy status for a service.
    async fn get_policy_status(
        &self,
        request: Request<GetPolicyStatusRequest>,
    ) -> Result<Response<PolicyStatusResponse>, Status>;

    /// List registered instances.
    async fn list_instances(
        &self,
        request: Request<ListInstancesRequest>,
    ) -> Result<Response<ListInstancesResponse>, Status>;

    /// Get health of a specific instance.
    async fn get_instance_health(
        &self,
        request: Request<GetInstanceHealthRequest>,
    ) -> Result<Response<InstanceHealthResponse>, Status>;

    /// Stream type for deployment events.
    type WatchDeploymentStream: Stream<Item = Result<DeploymentEvent, Status>> + Send;

    /// Watch deployment progress.
    async fn watch_deployment(
        &self,
        request: Request<WatchDeploymentRequest>,
    ) -> Result<Response<Self::WatchDeploymentStream>, Status>;
}

/// gRPC server wrapper for ControlPlane service.
pub struct ControlPlaneServiceServer<T: ControlPlane> {
    inner: T,
}

impl<T: ControlPlane> ControlPlaneServiceServer<T> {
    /// Create a new server with the given service implementation.
    pub fn new(inner: T) -> Self {
        Self { inner }
    }
}

impl<T: ControlPlane> Clone for ControlPlaneServiceServer<T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T: ControlPlane + Clone> tonic::codegen::Service<tonic::codegen::http::Request<tonic::body::BoxBody>>
    for ControlPlaneServiceServer<T>
{
    type Response = tonic::codegen::http::Response<tonic::body::BoxBody>;
    type Error = std::convert::Infallible;
    type Future = std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>,
    >;

    fn poll_ready(
        &mut self,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        std::task::Poll::Ready(Ok(()))
    }

    fn call(&mut self, _req: tonic::codegen::http::Request<tonic::body::BoxBody>) -> Self::Future {
        // For now, return unimplemented
        // The actual implementation would route to the appropriate method
        Box::pin(async move {
            let response = tonic::codegen::http::Response::builder()
                .status(501)
                .body(tonic::body::empty_body())
                .unwrap();
            Ok(response)
        })
    }
}

impl<T: ControlPlane + Clone> tonic::server::NamedService for ControlPlaneServiceServer<T> {
    const NAME: &'static str = "control_plane.ControlPlane";
}

#[cfg(test)]
mod tests {
    use super::*;

    // Integration tests would go here
    // They require setting up a Distributor with a registry
}
