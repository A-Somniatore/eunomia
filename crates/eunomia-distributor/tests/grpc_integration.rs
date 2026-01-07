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
        assert!(response.is_ok(), "Update {} should succeed", i);
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

// =============================================================================
// Bundle Push Tests (PolicyPusher Integration)
// =============================================================================

/// Test pushing a policy bundle to a healthy instance.
#[tokio::test]
async fn test_bundle_push_to_healthy_instance() {
    use eunomia_distributor::instance::{Instance, InstanceMetadata, InstanceStatus};
    use eunomia_distributor::pusher::{PolicyPusher, PushConfig};

    let pusher = PolicyPusher::new(PushConfig::default());

    // Create a healthy instance
    let mut instance = Instance::new("archimedes-1", "localhost:9091")
        .with_metadata(InstanceMetadata::for_service("payment-service"));
    instance.update_status(InstanceStatus::Healthy {
        policy_version: Some("v1.0.0".to_string()),
        last_check: std::time::Instant::now(),
    });

    // Push new version
    let result = pusher.push(&instance, "payment-service", "v2.0.0").await;

    assert!(result.is_ok());
    let push_result = result.unwrap();
    assert!(push_result.success);
    assert_eq!(push_result.version, "v2.0.0");
    assert_eq!(push_result.instance_id, "archimedes-1");
    assert_eq!(push_result.attempts, 1);
    assert!(push_result.error.is_none());
}

/// Test pushing to multiple instances in parallel.
#[tokio::test]
async fn test_bundle_push_to_multiple_instances() {
    use eunomia_distributor::instance::{Instance, InstanceMetadata, InstanceStatus};
    use eunomia_distributor::pusher::{PolicyPusher, PushConfig};

    let pusher = PolicyPusher::new(PushConfig::default());

    // Create multiple healthy instances
    let instances: Vec<Instance> = (1..=5)
        .map(|i| {
            let mut inst =
                Instance::new(format!("archimedes-{}", i), format!("192.168.1.{}:9091", i))
                    .with_metadata(InstanceMetadata::for_service("users-service"));
            inst.update_status(InstanceStatus::Healthy {
                policy_version: Some("v1.0.0".to_string()),
                last_check: std::time::Instant::now(),
            });
            inst
        })
        .collect();

    // Push to all instances concurrently
    let futures: Vec<_> = instances
        .iter()
        .map(|inst| pusher.push(inst, "users-service", "v2.0.0"))
        .collect();

    let results = futures::future::join_all(futures).await;

    // All pushes should succeed
    for (i, result) in results.iter().enumerate() {
        assert!(result.is_ok(), "Push to instance {} should succeed", i + 1);
        let push_result = result.as_ref().unwrap();
        assert!(
            push_result.success,
            "Instance {} push should report success",
            i + 1
        );
        assert_eq!(push_result.version, "v2.0.0");
    }
}

/// Test pushing to unreachable instance triggers retry logic.
#[tokio::test]
async fn test_bundle_push_retry_on_unreachable() {
    use eunomia_distributor::instance::{Instance, InstanceStatus};
    use eunomia_distributor::pusher::{PolicyPusher, PushConfig};

    // Configure for quick retries
    let config = PushConfig::builder()
        .max_retries(3)
        .retry_delay(Duration::from_millis(10))
        .build();
    let pusher = PolicyPusher::new(config);

    // Create an unreachable instance
    let mut instance = Instance::new("archimedes-down", "192.168.1.99:9091");
    instance.update_status(InstanceStatus::Unreachable {
        last_error: "Connection refused".to_string(),
        since: std::time::Instant::now(),
        failure_count: 5,
    });

    // Push should fail after retries
    let result = pusher.push(&instance, "test-service", "v1.0.0").await;

    assert!(result.is_ok()); // Returns Ok with failure status
    let push_result = result.unwrap();
    assert!(!push_result.success);
    assert!(push_result.error.is_some());
    let error_msg = push_result.error.as_ref().unwrap();
    assert!(error_msg.contains("unreachable") || error_msg.contains("Unreachable"));
}

/// Test push with compression enabled.
#[tokio::test]
async fn test_bundle_push_with_compression() {
    use eunomia_distributor::instance::{Instance, InstanceMetadata, InstanceStatus};
    use eunomia_distributor::pusher::{PolicyPusher, PushConfig};

    let config = PushConfig::builder().compression(true).build();
    let pusher = PolicyPusher::new(config);

    let mut instance = Instance::new("archimedes-1", "localhost:9091")
        .with_metadata(InstanceMetadata::for_service("orders-service"));
    instance.update_status(InstanceStatus::Healthy {
        policy_version: Some("v1.0.0".to_string()),
        last_check: std::time::Instant::now(),
    });

    let result = pusher.push(&instance, "orders-service", "v1.1.0").await;

    assert!(result.is_ok());
    assert!(result.unwrap().success);
}

/// Test push with custom timeouts.
#[tokio::test]
async fn test_bundle_push_custom_timeouts() {
    use eunomia_distributor::instance::{Instance, InstanceMetadata, InstanceStatus};
    use eunomia_distributor::pusher::{PolicyPusher, PushConfig};

    let config = PushConfig::builder()
        .connect_timeout(Duration::from_secs(5))
        .request_timeout(Duration::from_secs(15))
        .build();
    let pusher = PolicyPusher::new(config);

    let mut instance = Instance::new("archimedes-1", "localhost:9091")
        .with_metadata(InstanceMetadata::for_service("inventory-service"));
    instance.update_status(InstanceStatus::Healthy {
        policy_version: Some("v2.0.0".to_string()),
        last_check: std::time::Instant::now(),
    });

    let result = pusher.push(&instance, "inventory-service", "v2.1.0").await;

    assert!(result.is_ok());
    let push_result = result.unwrap();
    assert!(push_result.success);
    assert!(push_result.duration < Duration::from_secs(5)); // Should complete quickly
}

/// Test health check before push.
#[tokio::test]
async fn test_bundle_push_health_check_workflow() {
    use eunomia_distributor::health::HealthState;
    use eunomia_distributor::instance::{Instance, InstanceMetadata, InstanceStatus};
    use eunomia_distributor::pusher::{PolicyPusher, PushConfig};

    let pusher = PolicyPusher::new(PushConfig::default());

    let mut instance = Instance::new("archimedes-1", "localhost:9091")
        .with_metadata(InstanceMetadata::for_service("auth-service"));
    instance.update_status(InstanceStatus::Healthy {
        policy_version: Some("v1.0.0".to_string()),
        last_check: std::time::Instant::now(),
    });

    // First, check health
    let health = pusher.health_check(&instance).await;
    assert!(health.is_ok());
    let health_check = health.unwrap();
    assert_eq!(health_check.state, HealthState::Healthy);
    assert_eq!(health_check.policy_version, Some("v1.0.0".to_string()));

    // Then push if healthy
    if health_check.state == HealthState::Healthy {
        let result = pusher.push(&instance, "auth-service", "v1.1.0").await;
        assert!(result.is_ok());
        assert!(result.unwrap().success);
    }
}

/// Test push to instance with no prior policy version.
#[tokio::test]
async fn test_bundle_push_initial_deployment() {
    use eunomia_distributor::instance::{Instance, InstanceMetadata, InstanceStatus};
    use eunomia_distributor::pusher::{PolicyPusher, PushConfig};

    let pusher = PolicyPusher::new(PushConfig::default());

    // Instance with no prior policy version (fresh deployment)
    let mut instance = Instance::new("archimedes-new", "localhost:9092")
        .with_metadata(InstanceMetadata::for_service("new-service"));
    instance.update_status(InstanceStatus::Healthy {
        policy_version: None, // No prior version
        last_check: std::time::Instant::now(),
    });

    let result = pusher.push(&instance, "new-service", "v1.0.0").await;

    assert!(result.is_ok());
    let push_result = result.unwrap();
    assert!(push_result.success);
    assert_eq!(push_result.version, "v1.0.0");
}

/// Test push metrics (duration tracking).
#[tokio::test]
async fn test_bundle_push_tracks_duration() {
    use eunomia_distributor::instance::{Instance, InstanceStatus};
    use eunomia_distributor::pusher::{PolicyPusher, PushConfig};

    let pusher = PolicyPusher::new(PushConfig::default());

    let mut instance = Instance::new("archimedes-1", "localhost:9091");
    instance.update_status(InstanceStatus::Healthy {
        policy_version: Some("v1.0.0".to_string()),
        last_check: std::time::Instant::now(),
    });

    let result = pusher.push(&instance, "metrics-service", "v1.0.0").await;

    assert!(result.is_ok());
    let push_result = result.unwrap();
    assert!(push_result.duration > Duration::ZERO);
    assert!(push_result.duration < Duration::from_secs(10)); // Reasonable upper bound
}

/// Test push to unhealthy instance.
#[tokio::test]
async fn test_bundle_push_to_unhealthy_instance() {
    use eunomia_distributor::instance::{Instance, InstanceStatus};
    use eunomia_distributor::pusher::{PolicyPusher, PushConfig};

    let pusher = PolicyPusher::new(PushConfig::default());

    let mut instance = Instance::new("archimedes-sick", "localhost:9091");
    instance.update_status(InstanceStatus::Unhealthy {
        reason: "High memory usage".to_string(),
        since: std::time::Instant::now(),
    });

    // Push should still be attempted to unhealthy (but not unreachable) instances
    let result = pusher.push(&instance, "test-service", "v1.0.0").await;

    assert!(result.is_ok());
    // Unhealthy instances might still accept pushes (they're reachable)
    let push_result = result.unwrap();
    assert!(push_result.success);
}

// =============================================================================
// Rollback Controller Integration Tests
// =============================================================================

/// Test RollbackController deployment recording and version history.
#[tokio::test]
async fn test_rollback_controller_version_history() {
    use eunomia_distributor::rollback::{RollbackConfig, RollbackController};

    let config = RollbackConfig::default();
    let controller = RollbackController::new(config);

    // Record multiple deployments
    controller.record_deployment("orders-service", "1.0.0", "deploy-1");
    controller.record_deployment("orders-service", "1.1.0", "deploy-2");
    controller.record_deployment("orders-service", "1.2.0", "deploy-3");

    // Verify version history
    let history = controller.get_version_history("orders-service");
    assert_eq!(history.len(), 3);
    assert_eq!(history[0].version, "1.0.0");
    assert_eq!(history[1].version, "1.1.0");
    assert_eq!(history[2].version, "1.2.0");

    // Verify current version
    let current = controller.get_current_version("orders-service");
    assert_eq!(current, Some("1.2.0".to_string()));

    // Verify previous version for rollback
    let previous = controller.get_previous_version("orders-service");
    assert_eq!(previous, Some("1.1.0".to_string()));
}

/// Test RollbackController auto-rollback trigger.
#[tokio::test]
async fn test_rollback_controller_auto_rollback_trigger() {
    use eunomia_distributor::rollback::{RollbackConfig, RollbackController};

    let config = RollbackConfig::builder()
        .auto_rollback(true)
        .failure_threshold(3)
        .failure_window(Duration::from_secs(60))
        .build();

    let controller = RollbackController::new(config);

    // Record deployments
    controller.record_deployment("api-service", "1.0.0", "deploy-1");
    controller.record_deployment("api-service", "2.0.0", "deploy-2");

    // Record health failures
    controller.record_health_failure("api-service");
    controller.record_health_failure("api-service");

    // Not yet at threshold
    assert!(controller.should_auto_rollback("api-service").is_none());

    // Record third failure - should trigger
    controller.record_health_failure("api-service");

    // Now should recommend rollback
    let target = controller.should_auto_rollback("api-service");
    assert!(target.is_some());
    assert_eq!(target.unwrap(), "1.0.0");
}

/// Test RollbackController validation.
#[tokio::test]
async fn test_rollback_controller_validation() {
    use eunomia_distributor::rollback::{RollbackConfig, RollbackController, RollbackTrigger};

    let config = RollbackConfig::default();
    let controller = RollbackController::new(config);

    // Record deployment history
    controller.record_deployment("payments-service", "1.0.0", "d-1");
    controller.record_deployment("payments-service", "2.0.0", "d-2");

    // Valid rollback
    let trigger = RollbackTrigger::manual("payments-service", "1.0.0", "Bug found");
    let result = controller.validate_rollback(&trigger);
    assert!(result.is_ok());

    // Invalid rollback - version not in history
    let trigger = RollbackTrigger::manual("payments-service", "0.5.0", "Bad version");
    let result = controller.validate_rollback(&trigger);
    assert!(result.is_err());

    // Invalid rollback - service doesn't exist
    let trigger = RollbackTrigger::manual("unknown-service", "1.0.0", "Unknown");
    let result = controller.validate_rollback(&trigger);
    assert!(result.is_err());
}

/// Test RollbackController with force flag bypasses validation.
#[tokio::test]
async fn test_rollback_controller_force_rollback() {
    use eunomia_distributor::rollback::{RollbackConfig, RollbackController, RollbackTrigger};

    let config = RollbackConfig::default();
    let controller = RollbackController::new(config);

    // Record deployment history
    controller.record_deployment("auth-service", "1.0.0", "d-1");

    // Try to rollback to non-existent version without force
    let trigger = RollbackTrigger::manual("auth-service", "0.9.0", "Emergency");
    let result = controller.validate_rollback(&trigger);
    assert!(result.is_err());

    // With force flag, should pass validation
    let trigger = RollbackTrigger::forced("auth-service", "0.9.0", "Emergency");
    let result = controller.validate_rollback(&trigger);
    assert!(result.is_ok());
}

/// Test RollbackController cooldown prevents rapid rollbacks.
#[tokio::test]
async fn test_rollback_controller_cooldown() {
    use eunomia_distributor::rollback::{RollbackConfig, RollbackController, RollbackResult};

    let config = RollbackConfig::builder()
        .auto_rollback(true)
        .failure_threshold(1)
        .cooldown_period(Duration::from_secs(300)) // 5 minute cooldown
        .build();

    let controller = RollbackController::new(config);

    // Record deployments
    controller.record_deployment("cache-service", "1.0.0", "d-1");
    controller.record_deployment("cache-service", "2.0.0", "d-2");

    // Record failure
    controller.record_health_failure("cache-service");

    // First auto-rollback should trigger
    let target = controller.should_auto_rollback("cache-service");
    assert!(target.is_some());

    // Simulate rollback completion
    let result = RollbackResult::success(
        "rb-1",
        "cache-service",
        "2.0.0",
        "1.0.0",
        1,
        Duration::from_secs(1),
        "Auto-rollback",
        true,
    );
    controller.record_rollback(result);

    // Record another failure immediately
    controller.record_health_failure("cache-service");

    // Should NOT trigger due to cooldown
    let target = controller.should_auto_rollback("cache-service");
    assert!(target.is_none());
}

/// Test RollbackController with audit logging.
#[tokio::test]
async fn test_rollback_controller_audit_integration() {
    use eunomia_audit::{AuditLogger, InMemoryBackend};
    use eunomia_distributor::rollback::{
        RollbackConfig, RollbackController, RollbackResult, RollbackTrigger,
    };

    let config = RollbackConfig::default();
    let backend = Arc::new(InMemoryBackend::new());
    let logger = Arc::new(AuditLogger::builder().with_backend(backend.clone()).build());

    let controller = RollbackController::with_audit_logger(config, logger);

    // Record deployments
    controller.record_deployment("inventory-service", "1.0.0", "d-1");
    controller.record_deployment("inventory-service", "2.0.0", "d-2");

    // Prepare and execute rollback
    let trigger = RollbackTrigger::manual("inventory-service", "1.0.0", "Performance issue");
    let from_version = controller.prepare_rollback(&trigger).unwrap();
    assert_eq!(from_version, "2.0.0");

    // Record completion
    let result = RollbackResult::success(
        "rb-audit-1",
        "inventory-service",
        "2.0.0",
        "1.0.0",
        5,
        Duration::from_secs(3),
        "Performance issue",
        false,
    );
    controller.record_rollback(result);

    // Verify audit events
    let events = backend.events();
    assert!(events.len() >= 2); // At least started and completed
    assert!(events.iter().any(|e| e.contains("rollback_started")));
    assert!(events.iter().any(|e| e.contains("rollback_completed")));
}

/// Test EventBus integration with ControlPlaneService.
#[tokio::test]
async fn test_event_bus_control_plane_integration() {
    use eunomia_distributor::events::{DeploymentEventData, EventBus};

    let distributor = create_test_distributor(vec![]).await;
    let event_bus = Arc::new(EventBus::new(100));
    let _service = ControlPlaneService::with_event_bus(distributor, event_bus.clone());

    // Subscribe before publishing
    let mut subscriber = event_bus.subscribe();

    // Publish a test event
    let event = DeploymentEventData::started("deploy-test", "test-svc", "1.0.0");
    let receivers = event_bus.publish(event.clone());
    assert_eq!(receivers, 1);

    // Receive and verify
    let received = subscriber.recv().await.unwrap();
    assert_eq!(received.deployment_id, "deploy-test");
    assert_eq!(received.service, "test-svc");
}

/// Test filtered event subscription.
#[tokio::test]
async fn test_event_bus_filtered_subscription() {
    use eunomia_distributor::events::{DeploymentEventData, EventBus};

    let event_bus = EventBus::new(100);

    // Create filtered subscriber for specific deployment
    let subscriber = event_bus.subscribe();
    let mut filtered = subscriber.filter_deployment("target-deploy".to_string());

    // Spawn publisher that sends multiple events
    let bus_clone = event_bus.clone();
    tokio::spawn(async move {
        // Publish events for different deployments
        bus_clone.publish(DeploymentEventData::started("other-deploy", "svc1", "1.0"));
        bus_clone.publish(DeploymentEventData::started("target-deploy", "svc2", "2.0"));
        bus_clone.publish(DeploymentEventData::completed(
            "target-deploy",
            "svc2",
            "2.0",
        ));
        bus_clone.publish(DeploymentEventData::started(
            "another-deploy",
            "svc3",
            "3.0",
        ));
    });

    // Should only receive target-deploy events
    let e1 = filtered.recv().await.unwrap();
    assert_eq!(e1.deployment_id, "target-deploy");
    assert_eq!(e1.service, "svc2");

    let e2 = filtered.recv().await.unwrap();
    assert_eq!(e2.deployment_id, "target-deploy");
}

/// Test RollbackResult tracking.
#[tokio::test]
async fn test_rollback_result_tracking() {
    use eunomia_distributor::rollback::{RollbackConfig, RollbackController, RollbackResult};

    let config = RollbackConfig::builder().max_history_entries(5).build();

    let controller = RollbackController::new(config);

    // Record deployments
    controller.record_deployment("tracking-svc", "1.0.0", "d-1");
    controller.record_deployment("tracking-svc", "2.0.0", "d-2");

    // Record multiple rollbacks
    for i in 0..3 {
        let result = RollbackResult::success(
            &format!("rb-{i}"),
            "tracking-svc",
            "2.0.0",
            "1.0.0",
            3,
            Duration::from_secs(2),
            &format!("Rollback #{i}"),
            false,
        );
        controller.record_rollback(result);

        // Re-deploy between rollbacks
        controller.record_deployment("tracking-svc", "2.0.0", &format!("d-{}", i + 3));
    }

    // Verify rollback history
    let history = controller.get_rollback_history();
    assert_eq!(history.len(), 3);

    // Verify service-specific history
    let svc_history = controller.get_rollback_history_for_service("tracking-svc");
    assert_eq!(svc_history.len(), 3);
}
