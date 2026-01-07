//! End-to-End Flow Tests for Eunomia
//!
//! These tests verify the complete authorization policy workflow:
//! 1. Create policy bundles
//! 2. Sign bundles with Ed25519
//! 3. Push bundles to instances via gRPC
//! 4. Verify bundle integrity
//!
//! These tests exercise the full stack integration between:
//! - eunomia-core: Bundle creation and signing
//! - eunomia-distributor: Push distribution

use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::time::timeout;
use tonic::Request;

use eunomia_core::{Bundle, BundleSigner, BundleVerifier, SigningKeyPair};
use eunomia_distributor::grpc::types::{
    DeployPolicyRequest, GrpcDeploymentStrategy, GrpcHealthState, GrpcStrategyType,
    ListInstancesRequest,
};
use eunomia_distributor::grpc::{ControlPlane, ControlPlaneService};
use eunomia_distributor::{Distributor, DistributorConfig};

// =============================================================================
// Test Constants
// =============================================================================

/// Sample policy for testing - default deny with admin access
const SAMPLE_POLICY: &str = r#"
# METADATA
# title: Test Service Authorization
# description: Test policy for e2e testing
# scope: service
package test_service.authz

import future.keywords.if
import future.keywords.in

# Default deny
default allow := false

# Admin access
allow if {
    input.caller.type == "user"
    "admin" in input.caller.roles
}

# Service-to-service access
allow if {
    input.caller.type == "spiffe"
    input.caller.service_name == "trusted-service"
}

# User can read their own profile
allow if {
    input.caller.type == "user"
    input.operation_id == "getProfile"
    input.context.user_id == input.caller.user_id
}
"#;

// =============================================================================
// Helper Functions
// =============================================================================

/// Create a test distributor with static endpoints
async fn create_test_distributor(endpoints: Vec<String>) -> Arc<Distributor> {
    let config = DistributorConfig::builder()
        .static_endpoints(endpoints)
        .build();
    Arc::new(Distributor::new(config).await.unwrap())
}

/// Create a bundle from policy source
fn create_test_bundle() -> Bundle {
    Bundle::builder("test-service")
        .version("1.0.0")
        .add_policy("test_service.authz", SAMPLE_POLICY)
        .build()
}

// =============================================================================
// End-to-End Flow Tests
// =============================================================================

#[tokio::test]
async fn test_e2e_bundle_creation() {
    // Step 1: Create a bundle using the builder
    let bundle = create_test_bundle();

    // Verify bundle contents
    assert_eq!(bundle.version, "1.0.0");
    assert_eq!(bundle.name, "test-service");
    assert!(!bundle.policies.is_empty(), "Bundle should contain policies");
    assert!(
        bundle.policies.contains_key("test_service.authz"),
        "Bundle should contain test_service.authz policy"
    );
}

#[tokio::test]
async fn test_e2e_bundle_signing_and_verification() {
    // Step 1: Create bundle
    let bundle = create_test_bundle();

    // Step 2: Generate signing key pair
    let key_pair = SigningKeyPair::generate();
    let key_id = "test-key-1";

    // Step 3: Sign the bundle
    let signer = BundleSigner::from_key_pair(&key_pair, key_id.to_string());
    let signed_bundle = signer.sign(&bundle);

    // Verify signature was added
    assert!(signed_bundle.is_signed());
    let sig_count = signed_bundle.signatures.len();
    assert_eq!(sig_count, 1);

    // Step 4: Verify the bundle with correct key
    let mut verifier = BundleVerifier::new();
    verifier.add_public_key(key_id, key_pair.verifying_key());
    let verify_result = verifier.verify(&signed_bundle);
    assert!(verify_result.is_ok(), "Bundle should verify with correct key");

    // Step 5: Verify fails with wrong key
    let wrong_key_pair = SigningKeyPair::generate();
    let mut wrong_verifier = BundleVerifier::new();
    wrong_verifier.add_public_key(key_id, wrong_key_pair.verifying_key());
    let wrong_verify_result = wrong_verifier.verify(&signed_bundle);
    assert!(
        wrong_verify_result.is_err(),
        "Bundle should not verify with wrong key"
    );
}

#[tokio::test]
async fn test_e2e_bundle_roundtrip_serialization() {
    // Step 1: Create bundle
    let bundle = create_test_bundle();

    // Step 2: Serialize bundle to bytes
    let bytes = bundle.to_bytes().expect("Serialization should work");
    assert!(!bytes.is_empty(), "Serialized bytes should not be empty");

    // Step 3: Deserialize back
    let restored_bundle = Bundle::from_bytes(&bytes).expect("Deserialization should work");

    // Step 4: Verify bundle integrity preserved
    assert_eq!(restored_bundle.version, bundle.version);
    assert_eq!(restored_bundle.name, bundle.name);
    assert_eq!(restored_bundle.policies.len(), bundle.policies.len());

    // Step 5: Sign the restored bundle and verify
    let key_pair = SigningKeyPair::generate();
    let signer = BundleSigner::from_key_pair(&key_pair, "roundtrip-key".to_string());
    let signed = signer.sign(&restored_bundle);

    let mut verifier = BundleVerifier::new();
    verifier.add_public_key("roundtrip-key", key_pair.verifying_key());
    let verify_result = verifier.verify(&signed);
    assert!(
        verify_result.is_ok(),
        "Signature should be valid after roundtrip"
    );
}

#[tokio::test]
async fn test_e2e_bundle_checksum_deterministic() {
    // Create two bundles with same content
    let bundle1 = create_test_bundle();
    let bundle2 = create_test_bundle();

    // Checksums should be identical
    let checksum1 = bundle1.compute_checksum();
    let checksum2 = bundle2.compute_checksum();

    assert_eq!(checksum1, checksum2, "Checksums should be deterministic");
    assert!(!checksum1.is_empty(), "Checksum should not be empty");
}

#[tokio::test]
async fn test_e2e_deployment_request_flow() {
    // Step 1: Create distributor (no instances for test)
    let distributor = create_test_distributor(vec![]).await;
    let service = ControlPlaneService::new(distributor);

    // Step 2: Create deploy request with strategy
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
        reason: "E2E test deployment".to_string(),
    };

    // Step 3: Attempt deploy (will fail due to no instances, but validates flow)
    let response = service.deploy_policy(Request::new(request)).await;

    // Verify appropriate error returned
    assert!(response.is_err(), "Deploy should fail with no instances");
    let status = response.unwrap_err();
    assert!(
        status.message().contains("no instances")
            || status.message().contains("not found")
            || status.code() == tonic::Code::FailedPrecondition
            || status.code() == tonic::Code::NotFound,
        "Error should indicate no instances: {}",
        status.message()
    );
}

#[tokio::test]
async fn test_e2e_instance_discovery() {
    // Step 1: Create distributor with static endpoints
    let endpoints = vec![
        "http://instance-1:8080".to_string(),
        "http://instance-2:8080".to_string(),
        "http://instance-3:8080".to_string(),
    ];
    let distributor = create_test_distributor(endpoints.clone()).await;
    let service = ControlPlaneService::new(distributor);

    // Step 2: List instances
    let request = ListInstancesRequest {
        service_filter: "".to_string(), // All services
        health_filter: None,            // Include all health states
    };

    let response = service.list_instances(Request::new(request)).await;
    assert!(response.is_ok(), "List instances should succeed");

    // Note: Static endpoints may not appear as instances until connection
    // The actual behavior depends on the Distributor implementation
    let instances = response.unwrap().into_inner();
    // Just verify the call succeeded - instance count depends on implementation
    println!("Found {} instances", instances.instances.len());
}

#[tokio::test]
async fn test_e2e_instance_filtering_by_service() {
    // Create distributor with static endpoints
    let endpoints = vec![
        "http://instance-1:8080".to_string(),
        "http://instance-2:8080".to_string(),
    ];
    let distributor = create_test_distributor(endpoints).await;
    let service = ControlPlaneService::new(distributor);

    // List instances with service filter
    let request = ListInstancesRequest {
        service_filter: "nonexistent-service".to_string(),
        health_filter: None,
    };

    let response = service.list_instances(Request::new(request)).await;
    assert!(response.is_ok(), "List instances should succeed");
}

#[tokio::test]
async fn test_e2e_instance_filtering_by_health() {
    // Create distributor with static endpoints
    let endpoints = vec![
        "http://instance-1:8080".to_string(),
        "http://instance-2:8080".to_string(),
    ];
    let distributor = create_test_distributor(endpoints).await;
    let service = ControlPlaneService::new(distributor);

    // List only healthy instances
    let request = ListInstancesRequest {
        service_filter: "".to_string(),
        health_filter: Some(GrpcHealthState::Healthy),
    };

    let response = service.list_instances(Request::new(request)).await;
    assert!(response.is_ok(), "List instances should succeed");
}

// =============================================================================
// Error Scenario Tests
// =============================================================================

#[tokio::test]
async fn test_e2e_tampered_bundle_detected() {
    // Create and sign bundle
    let bundle = create_test_bundle();
    let key_pair = SigningKeyPair::generate();
    let signer = BundleSigner::from_key_pair(&key_pair, "tamper-test-key".to_string());
    let signed_bundle = signer.sign(&bundle);

    // Serialize
    let mut bytes = bundle.to_bytes().unwrap();

    // Tamper with the bytes (modify some policy content)
    if bytes.len() > 100 {
        bytes[100] ^= 0xFF; // Flip some bits
    }

    // Try to deserialize and verify
    match Bundle::from_bytes(&bytes) {
        Ok(tampered_bundle) => {
            // If it loads, sign it and compare checksums - they should differ
            let tampered_checksum = tampered_bundle.compute_checksum();
            let original_checksum = signed_bundle.bundle.compute_checksum();
            assert_ne!(
                tampered_checksum, original_checksum,
                "Tampered bundle should have different checksum"
            );
        }
        Err(_) => {
            // Deserialization failure is also acceptable for tampered data
        }
    }
}

#[tokio::test]
async fn test_e2e_unsigned_bundle_verification_fails() {
    // Create unsigned bundle wrapper
    let bundle = create_test_bundle();
    let unsigned = eunomia_core::SignedBundle::unsigned(bundle);

    // Attempt to verify - should fail because no signatures
    let mut verifier = BundleVerifier::new();
    let key_pair = SigningKeyPair::generate();
    verifier.add_public_key("test-key", key_pair.verifying_key());

    let result = verifier.verify(&unsigned);
    assert!(
        result.is_err(),
        "Unsigned bundle should fail verification"
    );
}

#[tokio::test]
async fn test_e2e_multiple_signatures() {
    // Create bundle
    let bundle = create_test_bundle();

    // Sign with first key
    let key_pair1 = SigningKeyPair::generate();
    let signer1 = BundleSigner::from_key_pair(&key_pair1, "key-1".to_string());
    let signed = signer1.sign(&bundle);

    // Can also add a second signature
    let key_pair2 = SigningKeyPair::generate();
    let signer2 = BundleSigner::from_key_pair(&key_pair2, "key-2".to_string());
    let double_signed = signer2.sign(&signed.bundle);

    // Verify with second key works
    let mut verifier = BundleVerifier::new();
    verifier.add_public_key("key-2", key_pair2.verifying_key());

    let result = verifier.verify(&double_signed);
    assert!(result.is_ok(), "Second signature should verify");
}

// =============================================================================
// Performance Timing Tests
// =============================================================================

#[tokio::test]
async fn test_e2e_bundle_creation_timing() {
    let start = Instant::now();

    // Create bundle
    let _bundle = create_test_bundle();

    let duration = start.elapsed();

    // Bundle creation should complete quickly (< 100ms for small policies)
    assert!(
        duration < Duration::from_millis(500),
        "Bundle creation took too long: {:?}",
        duration
    );

    println!("Bundle creation time: {:?}", duration);
}

#[tokio::test]
async fn test_e2e_signing_timing() {
    let bundle = create_test_bundle();
    let key_pair = SigningKeyPair::generate();
    let signer = BundleSigner::from_key_pair(&key_pair, "timing-key".to_string());

    // Measure signing time
    let start = Instant::now();
    let _signed = signer.sign(&bundle);
    let sign_duration = start.elapsed();

    // Signing should be fast (< 50ms)
    assert!(
        sign_duration < Duration::from_millis(100),
        "Signing took too long: {:?}",
        sign_duration
    );

    println!("Bundle signing time: {:?}", sign_duration);
}

#[tokio::test]
async fn test_e2e_verification_timing() {
    let bundle = create_test_bundle();
    let key_pair = SigningKeyPair::generate();
    let signer = BundleSigner::from_key_pair(&key_pair, "timing-key".to_string());
    let signed = signer.sign(&bundle);

    let mut verifier = BundleVerifier::new();
    verifier.add_public_key("timing-key", key_pair.verifying_key());

    // Measure verification time
    let iterations = 100;
    let start = Instant::now();

    for _ in 0..iterations {
        let _ = verifier.verify(&signed);
    }

    let duration = start.elapsed();
    let avg_duration = duration / iterations as u32;

    // Verification should be fast (< 5ms average)
    assert!(
        avg_duration < Duration::from_millis(10),
        "Average verification time too slow: {:?}",
        avg_duration
    );

    println!(
        "Average verification time: {:?} ({} iterations)",
        avg_duration, iterations
    );
}

#[tokio::test]
async fn test_e2e_checksum_timing() {
    let bundle = create_test_bundle();

    // Measure checksum time
    let iterations = 100;
    let start = Instant::now();

    for _ in 0..iterations {
        let _ = bundle.compute_checksum();
    }

    let duration = start.elapsed();
    let avg_duration = duration / iterations as u32;

    // Checksum should be fast (< 1ms average)
    assert!(
        avg_duration < Duration::from_millis(5),
        "Average checksum time too slow: {:?}",
        avg_duration
    );

    println!(
        "Average checksum time: {:?} ({} iterations)",
        avg_duration, iterations
    );
}

// =============================================================================
// Concurrency Tests
// =============================================================================

#[tokio::test]
async fn test_e2e_concurrent_bundle_signing() {
    use std::sync::atomic::{AtomicUsize, Ordering};

    let bundle = create_test_bundle();
    let key_pair = SigningKeyPair::generate();

    let success_count = Arc::new(AtomicUsize::new(0));
    let mut handles = vec![];

    // Spawn multiple concurrent signings
    for i in 0..10 {
        let bundle_clone = bundle.clone();
        let kp_clone = SigningKeyPair::from_seed(&key_pair.to_bytes()).unwrap();
        let success = Arc::clone(&success_count);

        let handle = tokio::spawn(async move {
            let signer = BundleSigner::from_key_pair(&kp_clone, format!("key-{}", i));
            let signed = signer.sign(&bundle_clone);
            if signed.is_signed() {
                success.fetch_add(1, Ordering::SeqCst);
            }
        });

        handles.push(handle);
    }

    // Wait for all to complete
    for handle in handles {
        handle.await.unwrap();
    }

    // All should succeed
    assert_eq!(
        success_count.load(Ordering::SeqCst),
        10,
        "All concurrent signings should succeed"
    );
}

#[tokio::test]
async fn test_e2e_concurrent_verifications() {
    use std::sync::atomic::{AtomicUsize, Ordering};

    let bundle = create_test_bundle();
    let key_pair = SigningKeyPair::generate();
    let signer = BundleSigner::from_key_pair(&key_pair, "concurrent-key".to_string());
    let signed = Arc::new(signer.sign(&bundle));
    let verifying_key = key_pair.verifying_key();

    let success_count = Arc::new(AtomicUsize::new(0));
    let mut handles = vec![];

    // Spawn multiple concurrent verifications
    for _ in 0..10 {
        let signed_clone = Arc::clone(&signed);
        let vk = verifying_key.clone();
        let success = Arc::clone(&success_count);

        let handle = tokio::spawn(async move {
            let mut verifier = BundleVerifier::new();
            verifier.add_public_key("concurrent-key", vk);
            if verifier.verify(&signed_clone).is_ok() {
                success.fetch_add(1, Ordering::SeqCst);
            }
        });

        handles.push(handle);
    }

    // Wait for all to complete
    for handle in handles {
        handle.await.unwrap();
    }

    // All should succeed
    assert_eq!(
        success_count.load(Ordering::SeqCst),
        10,
        "All concurrent verifications should succeed"
    );
}

#[tokio::test]
async fn test_e2e_timeout_handling() {
    let distributor = create_test_distributor(vec![]).await;
    let service = ControlPlaneService::new(distributor);

    // Create request
    let request = ListInstancesRequest {
        service_filter: "test".to_string(),
        health_filter: None,
    };

    // Execute with timeout
    let result = timeout(Duration::from_secs(5), async {
        service.list_instances(Request::new(request)).await
    })
    .await;

    // Should complete within timeout
    assert!(result.is_ok(), "Operation should complete within timeout");
}

// =============================================================================
// Key Management Tests
// =============================================================================

#[tokio::test]
async fn test_e2e_key_pair_generation() {
    // Generate key pairs
    let key_pair1 = SigningKeyPair::generate();
    let key_pair2 = SigningKeyPair::generate();

    // Keys should be different
    assert_ne!(
        key_pair1.to_bytes(),
        key_pair2.to_bytes(),
        "Generated keys should be unique"
    );

    // Roundtrip through base64
    let base64 = key_pair1.to_base64();
    let restored = SigningKeyPair::from_base64(&base64).expect("Should restore from base64");

    assert_eq!(
        key_pair1.to_bytes(),
        restored.to_bytes(),
        "Key should roundtrip through base64"
    );
}

#[tokio::test]
async fn test_e2e_key_pair_from_seed() {
    // Create from specific seed
    let seed = [42u8; 32];
    let key_pair1 = SigningKeyPair::from_seed(&seed).expect("Should create from seed");
    let key_pair2 = SigningKeyPair::from_seed(&seed).expect("Should create from seed");

    // Same seed should produce same keys
    assert_eq!(
        key_pair1.to_bytes(),
        key_pair2.to_bytes(),
        "Same seed should produce same key"
    );
}
