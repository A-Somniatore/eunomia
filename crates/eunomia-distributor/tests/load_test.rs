//! Load Testing for Eunomia Distributor
//!
//! These tests verify the system can handle concurrent operations at scale:
//! - Concurrent bundle signing (10-100 parallel operations)
//! - Concurrent verifications (10-100 parallel operations)
//! - Simulated multi-instance push scenarios
//! - Throughput measurements
//!
//! Run with: `cargo test --test load_test --release`

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use eunomia_core::{Bundle, BundleSigner, BundleVerifier, SigningKeyPair};

// =============================================================================
// Test Constants
// =============================================================================

/// Sample policy for load testing
const SAMPLE_POLICY: &str = r#"
# METADATA
# title: Load Test Policy
# description: Policy for load testing
# scope: service
package load_test.authz

import future.keywords.if
import future.keywords.in

default allow := false

allow if {
    input.caller.type == "user"
    "admin" in input.caller.roles
}

allow if {
    input.caller.type == "spiffe"
    input.caller.service_name == "trusted-service"
}
"#;

// =============================================================================
// Helper Functions
// =============================================================================

/// Create a bundle for load testing
fn create_test_bundle() -> Bundle {
    Bundle::builder("load-test-service")
        .version("1.0.0")
        .add_policy("load_test.authz", SAMPLE_POLICY)
        .build()
}

/// Shared metrics structure for load tests
struct LoadTestMetrics {
    successful: AtomicUsize,
    failed: AtomicUsize,
    total_duration_micros: AtomicUsize,
}

impl LoadTestMetrics {
    fn new() -> Self {
        Self {
            successful: AtomicUsize::new(0),
            failed: AtomicUsize::new(0),
            total_duration_micros: AtomicUsize::new(0),
        }
    }

    fn record_success(&self, duration: Duration) {
        self.successful.fetch_add(1, Ordering::SeqCst);
        self.total_duration_micros
            .fetch_add(duration.as_micros() as usize, Ordering::SeqCst);
    }

    fn record_failure(&self) {
        self.failed.fetch_add(1, Ordering::SeqCst);
    }

    fn successful(&self) -> usize {
        self.successful.load(Ordering::SeqCst)
    }

    fn failed(&self) -> usize {
        self.failed.load(Ordering::SeqCst)
    }

    fn avg_duration_micros(&self) -> f64 {
        let total = self.total_duration_micros.load(Ordering::SeqCst);
        let count = self.successful();
        if count == 0 {
            0.0
        } else {
            total as f64 / count as f64
        }
    }
}

// =============================================================================
// Load Tests - Concurrent Signing
// =============================================================================

#[tokio::test]
async fn test_load_concurrent_signing_10() {
    run_concurrent_signing_test(10).await;
}

#[tokio::test]
async fn test_load_concurrent_signing_50() {
    run_concurrent_signing_test(50).await;
}

#[tokio::test]
async fn test_load_concurrent_signing_100() {
    run_concurrent_signing_test(100).await;
}

async fn run_concurrent_signing_test(concurrency: usize) {
    let bundle = create_test_bundle();
    let key_pair = SigningKeyPair::generate();
    let metrics = Arc::new(LoadTestMetrics::new());
    let mut handles = vec![];

    let start = Instant::now();

    for i in 0..concurrency {
        let bundle_clone = bundle.clone();
        let kp_clone = SigningKeyPair::from_seed(&key_pair.to_bytes()).unwrap();
        let metrics_clone = Arc::clone(&metrics);

        let handle = tokio::spawn(async move {
            let op_start = Instant::now();
            let signer = BundleSigner::from_key_pair(&kp_clone, format!("key-{}", i));
            let signed = signer.sign(&bundle_clone);
            let duration = op_start.elapsed();

            if signed.is_signed() {
                metrics_clone.record_success(duration);
            } else {
                metrics_clone.record_failure();
            }
        });

        handles.push(handle);
    }

    // Wait for all to complete
    for handle in handles {
        handle.await.unwrap();
    }

    let total_duration = start.elapsed();

    // Verify results
    assert_eq!(
        metrics.successful(),
        concurrency,
        "All {} signings should succeed",
        concurrency
    );
    assert_eq!(metrics.failed(), 0, "No signings should fail");

    // Calculate throughput
    let throughput = concurrency as f64 / total_duration.as_secs_f64();

    println!(
        "Concurrent signing ({} ops): total={:?}, avg={:.2}µs, throughput={:.2} ops/sec",
        concurrency,
        total_duration,
        metrics.avg_duration_micros(),
        throughput
    );

    // SLO: Should complete within reasonable time (< 5s for 100 concurrent ops)
    assert!(
        total_duration < Duration::from_secs(5),
        "Concurrent signing took too long: {:?}",
        total_duration
    );
}

// =============================================================================
// Load Tests - Concurrent Verification
// =============================================================================

#[tokio::test]
async fn test_load_concurrent_verification_10() {
    run_concurrent_verification_test(10).await;
}

#[tokio::test]
async fn test_load_concurrent_verification_50() {
    run_concurrent_verification_test(50).await;
}

#[tokio::test]
async fn test_load_concurrent_verification_100() {
    run_concurrent_verification_test(100).await;
}

async fn run_concurrent_verification_test(concurrency: usize) {
    let bundle = create_test_bundle();
    let key_pair = SigningKeyPair::generate();
    let signer = BundleSigner::from_key_pair(&key_pair, "load-test-key".to_string());
    let signed = Arc::new(signer.sign(&bundle));
    let verifying_key = key_pair.verifying_key();
    let metrics = Arc::new(LoadTestMetrics::new());
    let mut handles = vec![];

    let start = Instant::now();

    for _ in 0..concurrency {
        let signed_clone = Arc::clone(&signed);
        let vk = verifying_key.clone();
        let metrics_clone = Arc::clone(&metrics);

        let handle = tokio::spawn(async move {
            let op_start = Instant::now();
            let mut verifier = BundleVerifier::new();
            verifier.add_public_key("load-test-key", vk);
            let result = verifier.verify(&signed_clone);
            let duration = op_start.elapsed();

            if result.is_ok() {
                metrics_clone.record_success(duration);
            } else {
                metrics_clone.record_failure();
            }
        });

        handles.push(handle);
    }

    // Wait for all to complete
    for handle in handles {
        handle.await.unwrap();
    }

    let total_duration = start.elapsed();

    // Verify results
    assert_eq!(
        metrics.successful(),
        concurrency,
        "All {} verifications should succeed",
        concurrency
    );
    assert_eq!(metrics.failed(), 0, "No verifications should fail");

    // Calculate throughput
    let throughput = concurrency as f64 / total_duration.as_secs_f64();

    println!(
        "Concurrent verification ({} ops): total={:?}, avg={:.2}µs, throughput={:.2} ops/sec",
        concurrency,
        total_duration,
        metrics.avg_duration_micros(),
        throughput
    );

    // SLO: Should complete within reasonable time (< 3s for 100 concurrent ops)
    assert!(
        total_duration < Duration::from_secs(3),
        "Concurrent verification took too long: {:?}",
        total_duration
    );
}

// =============================================================================
// Load Tests - Throughput Measurement
// =============================================================================

#[tokio::test]
async fn test_load_signing_throughput() {
    let bundle = create_test_bundle();
    let key_pair = SigningKeyPair::generate();
    let signer = BundleSigner::from_key_pair(&key_pair, "throughput-key".to_string());

    let iterations = 1000;
    let start = Instant::now();

    for _ in 0..iterations {
        let _ = signer.sign(&bundle);
    }

    let duration = start.elapsed();
    let throughput = iterations as f64 / duration.as_secs_f64();

    println!(
        "Signing throughput: {} ops in {:?}, {:.2} ops/sec",
        iterations, duration, throughput
    );

    // SLO: Should achieve at least 500 ops/sec
    assert!(
        throughput > 500.0,
        "Signing throughput too low: {:.2} ops/sec",
        throughput
    );
}

#[tokio::test]
async fn test_load_verification_throughput() {
    let bundle = create_test_bundle();
    let key_pair = SigningKeyPair::generate();
    let signer = BundleSigner::from_key_pair(&key_pair, "throughput-key".to_string());
    let signed = signer.sign(&bundle);

    let mut verifier = BundleVerifier::new();
    verifier.add_public_key("throughput-key", key_pair.verifying_key());

    let iterations = 1000;
    let start = Instant::now();

    for _ in 0..iterations {
        let _ = verifier.verify(&signed);
    }

    let duration = start.elapsed();
    let throughput = iterations as f64 / duration.as_secs_f64();

    println!(
        "Verification throughput: {} ops in {:?}, {:.2} ops/sec",
        iterations, duration, throughput
    );

    // SLO: Should achieve at least 1000 ops/sec
    assert!(
        throughput > 1000.0,
        "Verification throughput too low: {:.2} ops/sec",
        throughput
    );
}

#[tokio::test]
async fn test_load_checksum_throughput() {
    let bundle = create_test_bundle();

    let iterations = 10000;
    let start = Instant::now();

    for _ in 0..iterations {
        let _ = bundle.compute_checksum();
    }

    let duration = start.elapsed();
    let throughput = iterations as f64 / duration.as_secs_f64();

    println!(
        "Checksum throughput: {} ops in {:?}, {:.2} ops/sec",
        iterations, duration, throughput
    );

    // SLO: Should achieve at least 5000 ops/sec
    assert!(
        throughput > 5000.0,
        "Checksum throughput too low: {:.2} ops/sec",
        throughput
    );
}

// =============================================================================
// Load Tests - Simulated Multi-Instance Push
// =============================================================================

#[tokio::test]
async fn test_load_simulated_push_10_instances() {
    run_simulated_push_test(10).await;
}

#[tokio::test]
async fn test_load_simulated_push_50_instances() {
    run_simulated_push_test(50).await;
}

#[tokio::test]
async fn test_load_simulated_push_100_instances() {
    run_simulated_push_test(100).await;
}

/// Simulates pushing a signed bundle to multiple instances
async fn run_simulated_push_test(instance_count: usize) {
    let bundle = create_test_bundle();
    let key_pair = SigningKeyPair::generate();
    let signer = BundleSigner::from_key_pair(&key_pair, "push-key".to_string());
    let signed_bundle = Arc::new(signer.sign(&bundle));

    let metrics = Arc::new(LoadTestMetrics::new());
    let mut handles = vec![];

    let start = Instant::now();

    for instance_id in 0..instance_count {
        let bundle_clone = Arc::clone(&signed_bundle);
        let metrics_clone = Arc::clone(&metrics);

        let handle = tokio::spawn(async move {
            let op_start = Instant::now();

            // Simulate push operation:
            // 1. Serialize bundle
            let bytes = bundle_clone.bundle.to_bytes();
            if bytes.is_err() {
                metrics_clone.record_failure();
                return;
            }
            let bytes = bytes.unwrap();

            // 2. Simulate network latency (1-5ms)
            let latency = Duration::from_micros(1000 + (instance_id as u64 % 4000));
            tokio::time::sleep(latency).await;

            // 3. Deserialize bundle (at "receiver")
            let restored = Bundle::from_bytes(&bytes);
            if restored.is_err() {
                metrics_clone.record_failure();
                return;
            }

            // 4. Compute checksum to verify integrity
            let _checksum = restored.unwrap().compute_checksum();

            let duration = op_start.elapsed();
            metrics_clone.record_success(duration);
        });

        handles.push(handle);
    }

    // Wait for all to complete
    for handle in handles {
        handle.await.unwrap();
    }

    let total_duration = start.elapsed();

    // Verify results
    assert_eq!(
        metrics.successful(),
        instance_count,
        "All {} pushes should succeed",
        instance_count
    );
    assert_eq!(metrics.failed(), 0, "No pushes should fail");

    // Calculate throughput
    let throughput = instance_count as f64 / total_duration.as_secs_f64();

    println!(
        "Simulated push ({} instances): total={:?}, avg={:.2}µs, throughput={:.2} instances/sec",
        instance_count,
        total_duration,
        metrics.avg_duration_micros(),
        throughput
    );

    // SLO: Should complete within reasonable time
    // With simulated latency, 100 instances should complete in < 10s
    assert!(
        total_duration < Duration::from_secs(10),
        "Simulated push took too long: {:?}",
        total_duration
    );
}

// =============================================================================
// Load Tests - Sustained Load
// =============================================================================

#[tokio::test]
async fn test_load_sustained_operations() {
    let bundle = create_test_bundle();
    let key_pair = SigningKeyPair::generate();
    let signer = BundleSigner::from_key_pair(&key_pair, "sustained-key".to_string());

    let mut verifier = BundleVerifier::new();
    verifier.add_public_key("sustained-key", key_pair.verifying_key());

    // Run sustained operations for 2 seconds
    let duration = Duration::from_secs(2);
    let start = Instant::now();
    let mut operations = 0;

    while start.elapsed() < duration {
        // Full workflow: sign, serialize, deserialize, verify
        let signed = signer.sign(&bundle);
        let bytes = signed.bundle.to_bytes().unwrap();
        let restored = Bundle::from_bytes(&bytes).unwrap();
        let re_signed = signer.sign(&restored);
        let _ = verifier.verify(&re_signed);

        operations += 1;
    }

    let actual_duration = start.elapsed();
    let throughput = operations as f64 / actual_duration.as_secs_f64();

    println!(
        "Sustained load: {} full workflows in {:?}, {:.2} ops/sec",
        operations, actual_duration, throughput
    );

    // SLO: Should maintain at least 100 full workflows/sec
    assert!(
        throughput > 100.0,
        "Sustained throughput too low: {:.2} ops/sec",
        throughput
    );
}

// =============================================================================
// Load Tests - Memory Stability
// =============================================================================

#[tokio::test]
async fn test_load_memory_stability() {
    // Run many operations to check for memory leaks
    let bundle = create_test_bundle();
    let key_pair = SigningKeyPair::generate();
    let signer = BundleSigner::from_key_pair(&key_pair, "memory-key".to_string());

    let mut verifier = BundleVerifier::new();
    verifier.add_public_key("memory-key", key_pair.verifying_key());

    let iterations = 5000;

    for _ in 0..iterations {
        let signed = signer.sign(&bundle);
        let bytes = signed.bundle.to_bytes().unwrap();
        let restored = Bundle::from_bytes(&bytes).unwrap();
        let _ = restored.compute_checksum();
        let re_signed = signer.sign(&restored);
        let _ = verifier.verify(&re_signed);
    }

    // If we get here without OOM, the test passes
    println!(
        "Memory stability test completed: {} iterations without issues",
        iterations
    );
}
