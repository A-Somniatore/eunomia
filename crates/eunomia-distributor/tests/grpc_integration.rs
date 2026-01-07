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

// =============================================================================
// Bundle Push Tests (PolicyPusher Integration)
// =============================================================================

/// Test pushing a policy bundle to a healthy instance.
#[tokio::test]
async fn test_bundle_push_to_healthy_instance() {
    use eunomia_distributor::pusher::{PolicyPusher, PushConfig};
    use eunomia_distributor::instance::{Instance, InstanceMetadata, InstanceStatus};

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
    use eunomia_distributor::pusher::{PolicyPusher, PushConfig};
    use eunomia_distributor::instance::{Instance, InstanceMetadata, InstanceStatus};

    let pusher = PolicyPusher::new(PushConfig::default());

    // Create multiple healthy instances
    let instances: Vec<Instance> = (1..=5)
        .map(|i| {
            let mut inst = Instance::new(
                format!("archimedes-{}", i),
                format!("192.168.1.{}:9091", i),
            )
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
    use eunomia_distributor::pusher::{PolicyPusher, PushConfig};
    use eunomia_distributor::instance::{Instance, InstanceStatus};

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
    use eunomia_distributor::pusher::{PolicyPusher, PushConfig};
    use eunomia_distributor::instance::{Instance, InstanceMetadata, InstanceStatus};

    let config = PushConfig::builder()
        .compression(true)
        .build();
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
    use eunomia_distributor::pusher::{PolicyPusher, PushConfig};
    use eunomia_distributor::instance::{Instance, InstanceMetadata, InstanceStatus};

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
    use eunomia_distributor::pusher::{PolicyPusher, PushConfig};
    use eunomia_distributor::instance::{Instance, InstanceMetadata, InstanceStatus};
    use eunomia_distributor::health::HealthState;

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
    use eunomia_distributor::pusher::{PolicyPusher, PushConfig};
    use eunomia_distributor::instance::{Instance, InstanceMetadata, InstanceStatus};

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
    use eunomia_distributor::pusher::{PolicyPusher, PushConfig};
    use eunomia_distributor::instance::{Instance, InstanceStatus};

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
    use eunomia_distributor::pusher::{PolicyPusher, PushConfig};
    use eunomia_distributor::instance::{Instance, InstanceStatus};

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
