//! gRPC Integration Tests for Eunomia Distributor
//!
//! These tests verify the gRPC service logic works correctly.
//! Note: Full protocol-level tests require tonic codegen which is pending.

use std::sync::Arc;
use std::time::Duration;

use tonic::Request;

use eunomia_distributor::grpc::types::{
    DeployPolicyRequest, GetPolicyStatusRequest, GrpcDeploymentStrategy, GrpcHealthState,
    GrpcStrategyType, HealthCheckRequest, ListInstancesRequest, RollbackPolicyRequest,
    UpdatePolicyRequest,
};
use eunomia_distributor::grpc::{ControlPlane, ControlPlaneService, GrpcServerConfig};
use eunomia_distributor::{Distributor, DistributorConfig};

/// Create a test distributor with static discovery.
async fn create_test_distributor(endpoints: Vec<String>) -> Arc<Distributor> {
    let config = DistributorConfig::builder()
        .static_endpoints(endpoints)
        .build();
    Arc::new(Distributor::new(config).await.unwrap())
}

// =============================================================================
// Control Plane Service Logic Tests
// =============================================================================

#[tokio::test]
async fn test_control_plane_deploy_policy_no_instances() {
    // Create distributor with no endpoints
    let distributor = create_test_distributor(vec![]).await;
    let service = ControlPlaneService::new(distributor);

    // Try to deploy - should fail because there are no instances
    let request = DeployPolicyRequest {
        service: "test-service".to_string(),
        version: "v1.0.0".to_string(),
        strategy: None,
        target_instances: vec![],
        reason: "Test deployment".to_string(),
    };

    let response = service.deploy_policy(Request::new(request)).await;

    // Should fail (no instances to deploy to)
    assert!(response.is_err(), "Deploy should fail with no instances");
    let status = response.unwrap_err();
    assert!(status.message().contains("no instances found"));
}

#[tokio::test]
async fn test_control_plane_deploy_with_canary_strategy() {
    let distributor = create_test_distributor(vec![]).await;
    let service = ControlPlaneService::new(distributor);

    let request = DeployPolicyRequest {
        service: "users-service".to_string(),
        version: "v2.0.0".to_string(),
        strategy: Some(GrpcDeploymentStrategy {
            strategy_type: GrpcStrategyType::Canary,
            canary_percentage: 10,
            rolling_batch_size: 1,
            batch_delay_seconds: 30,
            auto_rollback: true,
            max_failures: 2,
        }),
        target_instances: vec![],
        reason: "Canary deployment".to_string(),
    };

    let response = service.deploy_policy(Request::new(request)).await;
    // Should fail because no instances are available
    assert!(response.is_err());
}

#[tokio::test]
async fn test_control_plane_deploy_with_rolling_strategy() {
    let distributor = create_test_distributor(vec![]).await;
    let service = ControlPlaneService::new(distributor);

    let request = DeployPolicyRequest {
        service: "orders-service".to_string(),
        version: "v1.5.0".to_string(),
        strategy: Some(GrpcDeploymentStrategy {
            strategy_type: GrpcStrategyType::Rolling,
            canary_percentage: 0,
            rolling_batch_size: 5,
            batch_delay_seconds: 60,
            auto_rollback: true,
            max_failures: 3,
        }),
        target_instances: vec![],
        reason: "Rolling deployment".to_string(),
    };

    let response = service.deploy_policy(Request::new(request)).await;
    // Should fail because no instances are available
    assert!(response.is_err());
}

#[tokio::test]
async fn test_control_plane_get_status() {
    let distributor = create_test_distributor(vec![]).await;
    let service = ControlPlaneService::new(distributor);

    let request = GetPolicyStatusRequest {
        service: "test-service".to_string(),
    };

    let response = service.get_policy_status(Request::new(request)).await;

    // Should return status (even if no current version)
    assert!(response.is_ok());
}

#[tokio::test]
async fn test_control_plane_list_instances_empty() {
    let distributor = create_test_distributor(vec![]).await;
    let service = ControlPlaneService::new(distributor);

    let request = ListInstancesRequest {
        service_filter: String::new(),
        health_filter: None,
    };

    let response = service.list_instances(Request::new(request)).await;

    assert!(response.is_ok());
    let resp = response.unwrap().into_inner();
    assert!(resp.instances.is_empty());
}

#[tokio::test]
async fn test_control_plane_list_instances_healthy_only() {
    let distributor = create_test_distributor(vec![]).await;
    let service = ControlPlaneService::new(distributor);

    let request = ListInstancesRequest {
        service_filter: "users-service".to_string(),
        health_filter: Some(GrpcHealthState::Healthy),
    };

    let response = service.list_instances(Request::new(request)).await;
    assert!(response.is_ok());
}

#[tokio::test]
async fn test_control_plane_rollback() {
    let distributor = create_test_distributor(vec![]).await;
    let service = ControlPlaneService::new(distributor);

    let request = RollbackPolicyRequest {
        service: "test-service".to_string(),
        target_version: "v1.0.0".to_string(),
        target_instances: vec![],
        reason: "Rollback due to errors".to_string(),
    };

    let response = service.rollback_policy(Request::new(request)).await;
    // Should fail because no instances are available
    assert!(response.is_err());
}

// =============================================================================
// Policy Receiver Service Logic Tests
// =============================================================================

#[tokio::test]
async fn test_policy_receiver_update() {
    use eunomia_distributor::grpc::{PolicyReceiver, PolicyReceiverService};

    let distributor = create_test_distributor(vec![]).await;
    let service = PolicyReceiverService::new(distributor);

    let bundle_data = vec![0xDE, 0xAD, 0xBE, 0xEF];
    let request = UpdatePolicyRequest {
        service: "users-service".to_string(),
        version: "v2.0.0".to_string(),
        bundle: bundle_data.clone(),
        checksum: String::new(), // No checksum validation
        force: false,
    };

    let response = service.update_policy(Request::new(request)).await;

    assert!(response.is_ok());
    let resp = response.unwrap().into_inner();
    assert!(resp.success);
}

// =============================================================================
// Hot-Reload Scenario Tests
// =============================================================================

/// Test that updating to a new version works correctly (hot-reload scenario).
#[tokio::test]
async fn test_hot_reload_update_version() {
    use eunomia_distributor::grpc::{PolicyReceiver, PolicyReceiverService};

    let distributor = create_test_distributor(vec![]).await;
    let service = PolicyReceiverService::new(distributor);

    // First deployment
    let bundle_v1 = b"bundle v1.0.0 content";
    let request_v1 = UpdatePolicyRequest {
        service: "hot-reload-service".to_string(),
        version: "v1.0.0".to_string(),
        bundle: bundle_v1.to_vec(),
        checksum: String::new(),
        force: false,
    };

    let response = service.update_policy(Request::new(request_v1)).await;
    assert!(response.is_ok());
    let resp_v1 = response.unwrap().into_inner();
    assert!(resp_v1.success);

    // Second deployment (hot-reload)
    let bundle_v2 = b"bundle v2.0.0 content with new rules";
    let request_v2 = UpdatePolicyRequest {
        service: "hot-reload-service".to_string(),
        version: "v2.0.0".to_string(),
        bundle: bundle_v2.to_vec(),
        checksum: String::new(),
        force: false,
    };

    let response = service.update_policy(Request::new(request_v2)).await;
    assert!(response.is_ok());
    let resp_v2 = response.unwrap().into_inner();
    assert!(resp_v2.success);
    // Previous version should be tracked (empty in this test setup)
}

/// Test multiple rapid updates (stress test hot-reload).
#[tokio::test]
async fn test_hot_reload_rapid_updates() {
    use eunomia_distributor::grpc::{PolicyReceiver, PolicyReceiverService};

    let distributor = create_test_distributor(vec![]).await;
    let service = PolicyReceiverService::new(distributor);

    // Perform 10 rapid updates
    for i in 1..=10 {
        let bundle = format!("bundle version {}", i).into_bytes();
        let request = UpdatePolicyRequest {
            service: "rapid-update-service".to_string(),
            version: format!("v1.0.{}", i),
            bundle,
            checksum: String::new(),
            force: false,
        };

        let response = service.update_policy(Request::new(request)).await;
        assert!(
            response.is_ok(),
            "Update {} should succeed",
            i
        );
        let resp = response.unwrap().into_inner();
        assert!(resp.success, "Update {} should report success", i);
    }
}

/// Test force update (bypass version check).
#[tokio::test]
async fn test_hot_reload_force_update() {
    use eunomia_distributor::grpc::{PolicyReceiver, PolicyReceiverService};

    let distributor = create_test_distributor(vec![]).await;
    let service = PolicyReceiverService::new(distributor);

    // Deploy v2.0.0
    let request_v2 = UpdatePolicyRequest {
        service: "force-update-service".to_string(),
        version: "v2.0.0".to_string(),
        bundle: b"v2 bundle".to_vec(),
        checksum: String::new(),
        force: false,
    };

    let response = service.update_policy(Request::new(request_v2)).await;
    assert!(response.is_ok());

    // Force downgrade to v1.0.0 (normally might be blocked)
    let request_v1_force = UpdatePolicyRequest {
        service: "force-update-service".to_string(),
        version: "v1.0.0".to_string(),
        bundle: b"v1 bundle - emergency rollback".to_vec(),
        checksum: String::new(),
        force: true, // Force flag set
    };

    let response = service.update_policy(Request::new(request_v1_force)).await;
    assert!(response.is_ok());
    assert!(response.unwrap().into_inner().success);
}

/// Test update with valid checksum (secure hot-reload).
#[tokio::test]
async fn test_hot_reload_with_valid_checksum() {
    use eunomia_distributor::grpc::{PolicyReceiver, PolicyReceiverService};
    use sha2::{Digest, Sha256};

    let distributor = create_test_distributor(vec![]).await;
    let service = PolicyReceiverService::new(distributor);

    let bundle_data = b"secure policy bundle content";
    let checksum = hex::encode(Sha256::digest(bundle_data));

    let request = UpdatePolicyRequest {
        service: "secure-service".to_string(),
        version: "v3.0.0".to_string(),
        bundle: bundle_data.to_vec(),
        checksum,
        force: false,
    };

    let response = service.update_policy(Request::new(request)).await;
    assert!(response.is_ok());
    assert!(response.unwrap().into_inner().success);
}

/// Test hot-reload to same version (idempotent update).
#[tokio::test]
async fn test_hot_reload_same_version() {
    use eunomia_distributor::grpc::{PolicyReceiver, PolicyReceiverService};

    let distributor = create_test_distributor(vec![]).await;
    let service = PolicyReceiverService::new(distributor);

    let bundle = b"idempotent bundle";

    // First update
    let request1 = UpdatePolicyRequest {
        service: "idempotent-service".to_string(),
        version: "v1.0.0".to_string(),
        bundle: bundle.to_vec(),
        checksum: String::new(),
        force: false,
    };

    let response = service.update_policy(Request::new(request1)).await;
    assert!(response.is_ok());

    // Same version update (should still succeed - idempotent)
    let request2 = UpdatePolicyRequest {
        service: "idempotent-service".to_string(),
        version: "v1.0.0".to_string(),
        bundle: bundle.to_vec(),
        checksum: String::new(),
        force: false,
    };

    let response = service.update_policy(Request::new(request2)).await;
    assert!(response.is_ok());
    assert!(response.unwrap().into_inner().success);
}

#[tokio::test]
async fn test_policy_receiver_update_checksum_validation() {
    use eunomia_distributor::grpc::{PolicyReceiver, PolicyReceiverService};

    let distributor = create_test_distributor(vec![]).await;
    let service = PolicyReceiverService::new(distributor);

    let bundle_data = b"test bundle data";
    let request = UpdatePolicyRequest {
        service: "users-service".to_string(),
        version: "v1.0.0".to_string(),
        bundle: bundle_data.to_vec(),
        checksum: "invalid_checksum".to_string(), // Wrong checksum
        force: false,
    };

    let response = service.update_policy(Request::new(request)).await;

    // Should fail due to checksum mismatch
    assert!(response.is_err());
    let status = response.unwrap_err();
    assert!(status.message().contains("Checksum mismatch"));
}

#[tokio::test]
async fn test_policy_receiver_health_check() {
    use eunomia_distributor::grpc::{PolicyReceiver, PolicyReceiverService};

    let distributor = create_test_distributor(vec![]).await;
    let service = PolicyReceiverService::new(distributor);

    let request = HealthCheckRequest {
        service: String::new(), // Empty = check all
    };

    let response = service.health_check(Request::new(request)).await;

    assert!(response.is_ok());
    let resp = response.unwrap().into_inner();
    assert_eq!(resp.status, GrpcHealthState::Healthy);
}

#[tokio::test]
async fn test_policy_receiver_health_check_specific_service() {
    use eunomia_distributor::grpc::{PolicyReceiver, PolicyReceiverService};

    let distributor = create_test_distributor(vec![]).await;
    let service = PolicyReceiverService::new(distributor);

    let request = HealthCheckRequest {
        service: "users-service".to_string(),
    };

    let response = service.health_check(Request::new(request)).await;
    assert!(response.is_ok());
}

// =============================================================================
// Server Configuration Tests
// =============================================================================

#[test]
fn test_grpc_server_config_defaults() {
    let config = GrpcServerConfig::default();

    assert_eq!(config.bind_address.port(), 9090);
    assert!(config.tcp_nodelay);
    assert!(config.enable_reflection);
    assert_eq!(config.max_recv_message_size, Some(4 * 1024 * 1024));
    assert_eq!(config.max_send_message_size, Some(4 * 1024 * 1024));
}

#[test]
fn test_grpc_server_config_builder() {
    let config = GrpcServerConfig::new("0.0.0.0:8080".parse().unwrap())
        .with_tcp_keepalive(Duration::from_secs(120))
        .with_max_concurrent_streams(1000)
        .with_max_recv_message_size(16 * 1024 * 1024)
        .with_reflection(false);

    assert_eq!(config.bind_address.port(), 8080);
    assert_eq!(config.tcp_keepalive, Some(Duration::from_secs(120)));
    assert_eq!(config.max_concurrent_streams, Some(1000));
    assert_eq!(config.max_recv_message_size, Some(16 * 1024 * 1024));
    assert!(!config.enable_reflection);
}

#[test]
fn test_grpc_server_config_disable_keepalive() {
    let config = GrpcServerConfig::default().without_tcp_keepalive();
    assert!(config.tcp_keepalive.is_none());
}

// =============================================================================
// gRPC Type Conversion Tests
// =============================================================================

#[test]
fn test_strategy_type_conversion() {
    use eunomia_distributor::grpc::types::GrpcStrategyType;
    use eunomia_distributor::StrategyType;

    assert_eq!(
        GrpcStrategyType::from(StrategyType::Immediate),
        GrpcStrategyType::Immediate
    );
    assert_eq!(
        GrpcStrategyType::from(StrategyType::Canary),
        GrpcStrategyType::Canary
    );
    assert_eq!(
        GrpcStrategyType::from(StrategyType::Rolling),
        GrpcStrategyType::Rolling
    );
}

#[test]
fn test_deployment_state_conversion() {
    use eunomia_distributor::grpc::types::GrpcDeploymentState;
    use eunomia_distributor::DeploymentState;

    assert_eq!(
        GrpcDeploymentState::from(DeploymentState::Pending),
        GrpcDeploymentState::Pending
    );
    assert_eq!(
        GrpcDeploymentState::from(DeploymentState::InProgress),
        GrpcDeploymentState::InProgress
    );
    assert_eq!(
        GrpcDeploymentState::from(DeploymentState::Completed),
        GrpcDeploymentState::Completed
    );
    assert_eq!(
        GrpcDeploymentState::from(DeploymentState::Failed),
        GrpcDeploymentState::Failed
    );
    assert_eq!(
        GrpcDeploymentState::from(DeploymentState::RolledBack),
        GrpcDeploymentState::RolledBack
    );
    assert_eq!(
        GrpcDeploymentState::from(DeploymentState::Cancelled),
        GrpcDeploymentState::Cancelled
    );
}

#[test]
fn test_health_state_conversion() {
    use eunomia_distributor::grpc::types::GrpcHealthState;
    use eunomia_distributor::HealthState;

    assert_eq!(
        GrpcHealthState::from(HealthState::Unknown),
        GrpcHealthState::Unknown
    );
    assert_eq!(
        GrpcHealthState::from(HealthState::Healthy),
        GrpcHealthState::Healthy
    );
    assert_eq!(
        GrpcHealthState::from(HealthState::Unhealthy),
        GrpcHealthState::Unhealthy
    );
    assert_eq!(
        GrpcHealthState::from(HealthState::Degraded),
        GrpcHealthState::Degraded
    );
}
