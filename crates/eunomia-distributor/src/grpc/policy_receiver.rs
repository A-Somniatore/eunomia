//! gRPC Policy Receiver service implementation.
//!
//! This service is called by Archimedes instances to receive policy updates
//! and report health status.

use std::sync::Arc;

use tonic::{Request, Response, Status};
use tracing::{debug, info, instrument};

use super::types::*;
use crate::{Distributor, HealthState};

/// Policy Receiver gRPC service implementation.
///
/// This service handles:
/// - Policy update requests from Archimedes instances
/// - Policy version queries
/// - Health check reporting
#[derive(Clone)]
pub struct PolicyReceiverService {
    distributor: Arc<Distributor>,
}

impl std::fmt::Debug for PolicyReceiverService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PolicyReceiverService").finish_non_exhaustive()
    }
}

impl PolicyReceiverService {
    /// Create a new Policy Receiver service.
    pub fn new(distributor: Arc<Distributor>) -> Self {
        Self { distributor }
    }

    /// Convert to a tonic service.
    pub fn into_service(self) -> PolicyReceiverServiceServer<Self> {
        PolicyReceiverServiceServer::new(self)
    }
}

/// Trait definition matching the protobuf service.
#[tonic::async_trait]
pub trait PolicyReceiver: Send + Sync + 'static {
    /// Update policy on a service.
    async fn update_policy(
        &self,
        request: Request<UpdatePolicyRequest>,
    ) -> Result<Response<UpdatePolicyResponse>, Status>;

    /// Get current policy for a service.
    async fn get_current_policy(
        &self,
        request: Request<GetCurrentPolicyRequest>,
    ) -> Result<Response<CurrentPolicyResponse>, Status>;

    /// Health check.
    async fn health_check(
        &self,
        request: Request<HealthCheckRequest>,
    ) -> Result<Response<HealthCheckResponse>, Status>;
}

#[tonic::async_trait]
impl PolicyReceiver for PolicyReceiverService {
    #[instrument(skip(self, request), fields(service = %request.get_ref().service))]
    async fn update_policy(
        &self,
        request: Request<UpdatePolicyRequest>,
    ) -> Result<Response<UpdatePolicyResponse>, Status> {
        let req = request.into_inner();
        info!(
            "UpdatePolicy request: service={}, version={}, bundle_size={}",
            req.service,
            req.version,
            req.bundle.len()
        );

        // Validate checksum
        let computed_checksum = sha256_hex(&req.bundle);
        if !req.checksum.is_empty() && computed_checksum != req.checksum {
            return Err(Status::invalid_argument(format!(
                "Checksum mismatch: expected {}, got {}",
                req.checksum, computed_checksum
            )));
        }

        // Get previous version from status
        let previous_version = self
            .distributor
            .get_status(&req.service)
            .await
            .ok()
            .and_then(|s| s.current_version)
            .unwrap_or_default();

        // For now, just report success - actual bundle storage would be implemented
        // when we have a registry component
        info!(
            "Policy update accepted: service={}, version={}",
            req.service, req.version
        );

        let response = UpdatePolicyResponse {
            success: true,
            previous_version,
            error_message: String::new(),
        };
        Ok(Response::new(response))
    }

    #[instrument(skip(self, request), fields(service = %request.get_ref().service))]
    async fn get_current_policy(
        &self,
        request: Request<GetCurrentPolicyRequest>,
    ) -> Result<Response<CurrentPolicyResponse>, Status> {
        let req = request.into_inner();
        debug!("GetCurrentPolicy request: service={}", req.service);

        let status = self.distributor.get_status(&req.service).await.map_err(|e| {
            Status::not_found(format!("No policy found for service {}: {}", req.service, e))
        })?;

        let version = status.current_version.ok_or_else(|| {
            Status::not_found(format!("No policy version found for service: {}", req.service))
        })?;

        let response = CurrentPolicyResponse {
            service: req.service,
            version,
            checksum: String::new(), // Would compute from actual bundle
            loaded_at: None,
        };

        Ok(Response::new(response))
    }

    #[instrument(skip(self, request))]
    async fn health_check(
        &self,
        request: Request<HealthCheckRequest>,
    ) -> Result<Response<HealthCheckResponse>, Status> {
        let req = request.into_inner();
        debug!("HealthCheck request: service={:?}", req.service);

        // Check if we can get status - that indicates the distributor is healthy
        let is_healthy = true; // Simple health check for now

        let status = if is_healthy {
            GrpcHealthState::Healthy
        } else {
            GrpcHealthState::Degraded
        };

        let mut service_statuses = std::collections::HashMap::new();

        // If a specific service is requested, get its status
        if !req.service.is_empty() {
            if let Ok(svc_status) = self.distributor.get_status(&req.service).await {
                let instances = self
                    .distributor
                    .list_instances(&req.service)
                    .await
                    .unwrap_or_default();

                let healthy_count = instances
                    .iter()
                    .filter(|i| i.status.to_health_state() == HealthState::Healthy)
                    .count();

                let service_health = if healthy_count == instances.len() && !instances.is_empty() {
                    GrpcHealthState::Healthy
                } else if healthy_count > 0 {
                    GrpcHealthState::Degraded
                } else {
                    GrpcHealthState::Unhealthy
                };

                service_statuses.insert(
                    req.service.clone(),
                    ServiceHealthStatus {
                        status: service_health,
                        policy_version: svc_status.current_version.unwrap_or_default(),
                        last_evaluation: None,
                        error_count: 0,
                    },
                );
            }
        }

        let response = HealthCheckResponse {
            status,
            message: if is_healthy {
                "Distributor is healthy".to_string()
            } else {
                "Distributor is degraded".to_string()
            },
            service_statuses,
        };

        Ok(Response::new(response))
    }
}

/// Compute SHA256 hash of data and return as hex string.
fn sha256_hex(data: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

/// gRPC server wrapper for PolicyReceiver service.
pub struct PolicyReceiverServiceServer<T: PolicyReceiver> {
    inner: T,
}

impl<T: PolicyReceiver> PolicyReceiverServiceServer<T> {
    /// Create a new server with the given service implementation.
    pub fn new(inner: T) -> Self {
        Self { inner }
    }
}

impl<T: PolicyReceiver> Clone for PolicyReceiverServiceServer<T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T: PolicyReceiver + Clone>
    tonic::codegen::Service<tonic::codegen::http::Request<tonic::body::BoxBody>>
    for PolicyReceiverServiceServer<T>
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
        Box::pin(async move {
            let response = tonic::codegen::http::Response::builder()
                .status(501)
                .body(tonic::body::empty_body())
                .unwrap();
            Ok(response)
        })
    }
}

impl<T: PolicyReceiver + Clone> tonic::server::NamedService for PolicyReceiverServiceServer<T> {
    const NAME: &'static str = "control_plane.PolicyReceiver";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sha256_hex() {
        let data = b"hello world";
        let hash = sha256_hex(data);
        assert_eq!(
            hash,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }

    #[test]
    fn test_sha256_empty() {
        let data = b"";
        let hash = sha256_hex(data);
        assert_eq!(
            hash,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }
}
