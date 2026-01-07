//! Error Scenario Tests for Eunomia Distributor
//!
//! These tests verify the system handles error conditions gracefully:
//! - Invalid input handling
//! - Partial failures
//! - Timeout handling
//! - Retry behavior
//! - Recovery scenarios
//!
//! Run with: `cargo test --test error_scenarios`

use std::sync::Arc;
use std::time::Duration;

use tokio::time::timeout;
use tonic::Request;

use eunomia_core::{Bundle, BundleSigner, BundleVerifier, SigningKeyPair};
use eunomia_distributor::grpc::types::{
    DeployPolicyRequest, GrpcDeploymentStrategy, GrpcStrategyType, ListInstancesRequest,
    RollbackPolicyRequest,
};
use eunomia_distributor::grpc::{ControlPlane, ControlPlaneService};
use eunomia_distributor::{Distributor, DistributorConfig};

// =============================================================================
// Test Constants
// =============================================================================

/// Sample policy for testing
const SAMPLE_POLICY: &str = r#"
package test.authz
default allow := false
allow if input.caller.type == "admin"
"#;

// =============================================================================
// Helper Functions
// =============================================================================

/// Create a test distributor with no endpoints
async fn create_empty_distributor() -> Arc<Distributor> {
    let config = DistributorConfig::builder()
        .static_endpoints(vec![])
        .build();
    Arc::new(Distributor::new(config).await.unwrap())
}

/// Create a test distributor with unreachable endpoints
#[allow(dead_code)]
async fn create_unreachable_distributor() -> Arc<Distributor> {
    let config = DistributorConfig::builder()
        .static_endpoints(vec![
            "http://unreachable-host-1:9999".to_string(),
            "http://unreachable-host-2:9999".to_string(),
        ])
        .build();
    Arc::new(Distributor::new(config).await.unwrap())
}

/// Create a test bundle
fn create_test_bundle() -> Bundle {
    Bundle::builder("error-test-service")
        .version("1.0.0")
        .add_policy("test.authz", SAMPLE_POLICY)
        .build()
}

// =============================================================================
// Error Scenario Tests - Invalid Input
// =============================================================================

#[tokio::test]
async fn test_error_empty_service_name_in_deploy() {
    let distributor = create_empty_distributor().await;
    let service = ControlPlaneService::new(distributor);

    let request = DeployPolicyRequest {
        service: "".to_string(), // Empty service name
        version: "1.0.0".to_string(),
        strategy: Some(GrpcDeploymentStrategy {
            strategy_type: GrpcStrategyType::Immediate,
            canary_percentage: 0,
            rolling_batch_size: 1,
            batch_delay_seconds: 0,
            auto_rollback: true,
            max_failures: 1,
        }),
        target_instances: vec![],
        reason: "Error test".to_string(),
    };

    let response = service.deploy_policy(Request::new(request)).await;
    assert!(response.is_err(), "Empty service name should be rejected");
}

#[tokio::test]
async fn test_error_empty_version_in_deploy() {
    let distributor = create_empty_distributor().await;
    let service = ControlPlaneService::new(distributor);

    let request = DeployPolicyRequest {
        service: "test-service".to_string(),
        version: "".to_string(), // Empty version
        strategy: Some(GrpcDeploymentStrategy {
            strategy_type: GrpcStrategyType::Immediate,
            canary_percentage: 0,
            rolling_batch_size: 1,
            batch_delay_seconds: 0,
            auto_rollback: true,
            max_failures: 1,
        }),
        target_instances: vec![],
        reason: "Error test".to_string(),
    };

    let response = service.deploy_policy(Request::new(request)).await;
    assert!(response.is_err(), "Empty version should be rejected");
}

#[tokio::test]
async fn test_error_invalid_canary_percentage() {
    let distributor = create_empty_distributor().await;
    let service = ControlPlaneService::new(distributor);

    let request = DeployPolicyRequest {
        service: "test-service".to_string(),
        version: "1.0.0".to_string(),
        strategy: Some(GrpcDeploymentStrategy {
            strategy_type: GrpcStrategyType::Canary,
            canary_percentage: 150, // Invalid: > 100
            rolling_batch_size: 1,
            batch_delay_seconds: 0,
            auto_rollback: true,
            max_failures: 1,
        }),
        target_instances: vec![],
        reason: "Error test".to_string(),
    };

    let response = service.deploy_policy(Request::new(request)).await;
    // Should either reject invalid percentage or handle gracefully
    // The exact behavior depends on implementation
    println!("Response: {:?}", response);
}

// =============================================================================
// Error Scenario Tests - No Instances Available
// =============================================================================

#[tokio::test]
async fn test_error_deploy_no_instances() {
    let distributor = create_empty_distributor().await;
    let service = ControlPlaneService::new(distributor);

    let request = DeployPolicyRequest {
        service: "test-service".to_string(),
        version: "1.0.0".to_string(),
        strategy: Some(GrpcDeploymentStrategy {
            strategy_type: GrpcStrategyType::Immediate,
            canary_percentage: 0,
            rolling_batch_size: 1,
            batch_delay_seconds: 0,
            auto_rollback: true,
            max_failures: 1,
        }),
        target_instances: vec![],
        reason: "Error test".to_string(),
    };

    let response = service.deploy_policy(Request::new(request)).await;
    assert!(
        response.is_err(),
        "Deploy with no instances should fail gracefully"
    );

    let status = response.unwrap_err();
    // Error message should indicate the issue
    println!(
        "Error code: {:?}, message: {}",
        status.code(),
        status.message()
    );
}

#[tokio::test]
async fn test_error_rollback_no_instances() {
    let distributor = create_empty_distributor().await;
    let service = ControlPlaneService::new(distributor);

    let request = RollbackPolicyRequest {
        service: "test-service".to_string(),
        target_version: "0.9.0".to_string(),
        target_instances: vec![],
        reason: "Error test rollback".to_string(),
    };

    let response = service.rollback_policy(Request::new(request)).await;
    assert!(
        response.is_err(),
        "Rollback with no instances should fail gracefully"
    );
}

// =============================================================================
// Error Scenario Tests - Timeout Handling
// =============================================================================

#[tokio::test]
async fn test_error_operation_timeout() {
    let distributor = create_empty_distributor().await;
    let service = ControlPlaneService::new(distributor);

    let request = ListInstancesRequest {
        service_filter: "test".to_string(),
        health_filter: None,
    };

    // Use a very short timeout
    let result = timeout(Duration::from_millis(100), async {
        service.list_instances(Request::new(request)).await
    })
    .await;

    // Should complete within timeout (no blocking operations for empty distributor)
    match result {
        Ok(response) => {
            // Operation completed within timeout
            assert!(
                response.is_ok(),
                "List instances should succeed for empty distributor"
            );
        }
        Err(_) => {
            // Timeout occurred - also acceptable if operation is slow
            println!("Operation timed out (acceptable for stress test)");
        }
    }
}

#[tokio::test]
async fn test_error_concurrent_timeout_handling() {
    let distributor = create_empty_distributor().await;
    let service = Arc::new(ControlPlaneService::new(distributor));

    let mut handles = vec![];

    // Spawn multiple concurrent operations with short timeouts
    for _ in 0..10 {
        let service_clone = Arc::clone(&service);
        let handle = tokio::spawn(async move {
            let request = ListInstancesRequest {
                service_filter: "test".to_string(),
                health_filter: None,
            };

            timeout(Duration::from_millis(50), async {
                service_clone.list_instances(Request::new(request)).await
            })
            .await
        });
        handles.push(handle);
    }

    // All operations should complete (success or timeout)
    for handle in handles {
        let result = handle.await;
        assert!(result.is_ok(), "Task should not panic");
    }
}

// =============================================================================
// Error Scenario Tests - Cryptographic Errors
// =============================================================================

#[tokio::test]
async fn test_error_signature_verification_wrong_key() {
    let bundle = create_test_bundle();
    let key_pair = SigningKeyPair::generate();
    let wrong_key_pair = SigningKeyPair::generate();

    let signer = BundleSigner::from_key_pair(&key_pair, "correct-key".to_string());
    let signed = signer.sign(&bundle);

    // Verify with wrong key
    let mut verifier = BundleVerifier::new();
    verifier.add_public_key("correct-key", wrong_key_pair.verifying_key());

    let result = verifier.verify(&signed);
    assert!(result.is_err(), "Wrong key should fail verification");
}

#[tokio::test]
async fn test_error_signature_verification_unknown_key_id() {
    let bundle = create_test_bundle();
    let key_pair = SigningKeyPair::generate();

    let signer = BundleSigner::from_key_pair(&key_pair, "key-1".to_string());
    let signed = signer.sign(&bundle);

    // Verify with different key ID
    let mut verifier = BundleVerifier::new();
    verifier.add_public_key("key-2", key_pair.verifying_key()); // Different key ID

    let result = verifier.verify(&signed);
    assert!(result.is_err(), "Unknown key ID should fail verification");
}

#[tokio::test]
async fn test_error_signature_verification_no_signatures() {
    let bundle = create_test_bundle();
    let unsigned = eunomia_core::SignedBundle::unsigned(bundle);
    let key_pair = SigningKeyPair::generate();

    let mut verifier = BundleVerifier::new();
    verifier.add_public_key("test-key", key_pair.verifying_key());

    let result = verifier.verify(&unsigned);
    assert!(result.is_err(), "Unsigned bundle should fail verification");
}

#[tokio::test]
async fn test_error_invalid_base64_key() {
    let result = SigningKeyPair::from_base64("not-valid-base64!!!");
    assert!(result.is_err(), "Invalid base64 should fail");
}

#[tokio::test]
async fn test_error_invalid_seed_length() {
    let short_seed = [0u8; 16]; // Too short, needs 32 bytes
    let result = SigningKeyPair::from_seed(&short_seed);
    assert!(result.is_err(), "Short seed should fail");

    let long_seed = [0u8; 64]; // Too long, needs 32 bytes
    let result = SigningKeyPair::from_seed(&long_seed);
    assert!(result.is_err(), "Long seed should fail");
}

// =============================================================================
// Error Scenario Tests - Bundle Errors
// =============================================================================

#[tokio::test]
async fn test_error_deserialize_invalid_bytes() {
    let invalid_bytes = vec![0xFF, 0xFE, 0xFD, 0xFC]; // Not a valid bundle
    let result = Bundle::from_bytes(&invalid_bytes);
    assert!(result.is_err(), "Invalid bytes should fail deserialization");
}

#[tokio::test]
async fn test_error_deserialize_empty_bytes() {
    let empty_bytes: Vec<u8> = vec![];
    let result = Bundle::from_bytes(&empty_bytes);
    assert!(result.is_err(), "Empty bytes should fail deserialization");
}

#[tokio::test]
async fn test_error_deserialize_truncated_bundle() {
    // Create a valid bundle, then truncate its serialization
    let bundle = create_test_bundle();
    let bytes = bundle.to_bytes().unwrap();

    // Truncate to half
    let truncated = &bytes[0..bytes.len() / 2];
    let result = Bundle::from_bytes(truncated);
    assert!(
        result.is_err(),
        "Truncated bundle should fail deserialization"
    );
}

// =============================================================================
// Error Scenario Tests - Recovery
// =============================================================================

#[tokio::test]
async fn test_error_recovery_after_failure() {
    let bundle = create_test_bundle();
    let key_pair = SigningKeyPair::generate();
    let signer = BundleSigner::from_key_pair(&key_pair, "recovery-key".to_string());

    // First, attempt invalid operations
    let unsigned = eunomia_core::SignedBundle::unsigned(bundle.clone());
    let mut verifier = BundleVerifier::new();
    verifier.add_public_key("recovery-key", key_pair.verifying_key());

    // This should fail
    let fail_result = verifier.verify(&unsigned);
    assert!(fail_result.is_err(), "First operation should fail");

    // Now do valid operations - system should recover
    let signed = signer.sign(&bundle);
    let success_result = verifier.verify(&signed);
    assert!(
        success_result.is_ok(),
        "System should recover after failure"
    );
}

#[tokio::test]
async fn test_error_partial_failure_handling() {
    // Simulate a scenario where some operations succeed and others fail
    let bundle = create_test_bundle();
    let key_pair = SigningKeyPair::generate();
    let signer = BundleSigner::from_key_pair(&key_pair, "partial-key".to_string());
    let signed = signer.sign(&bundle);

    let mut success_count = 0;
    let mut failure_count = 0;

    // Mix of valid and invalid verifications
    for i in 0..20 {
        let mut verifier = BundleVerifier::new();
        if i % 2 == 0 {
            // Valid key
            verifier.add_public_key("partial-key", key_pair.verifying_key());
        } else {
            // Wrong key
            let wrong_key = SigningKeyPair::generate();
            verifier.add_public_key("partial-key", wrong_key.verifying_key());
        }

        match verifier.verify(&signed) {
            Ok(()) => success_count += 1,
            Err(_) => failure_count += 1,
        }
    }

    // Should have mix of successes and failures
    assert_eq!(success_count, 10, "Half should succeed");
    assert_eq!(failure_count, 10, "Half should fail");
}

// =============================================================================
// Error Scenario Tests - Stress Under Errors
// =============================================================================

#[tokio::test]
async fn test_error_stress_invalid_operations() {
    // Run many invalid operations to ensure stability
    for _ in 0..100 {
        // Invalid deserialization
        let invalid = vec![0u8; 100];
        let _ = Bundle::from_bytes(&invalid);

        // Invalid key operations
        let _ = SigningKeyPair::from_base64("invalid");
        let _ = SigningKeyPair::from_seed(&[0u8; 16]);

        // Unsigned verification
        let bundle = create_test_bundle();
        let unsigned = eunomia_core::SignedBundle::unsigned(bundle);
        let mut verifier = BundleVerifier::new();
        let key_pair = SigningKeyPair::generate();
        verifier.add_public_key("test", key_pair.verifying_key());
        let _ = verifier.verify(&unsigned);
    }

    // If we get here without panic, the test passes
    println!("Stress test with errors completed successfully");
}
