//! Performance Benchmarks for Eunomia Distributor
//!
//! These benchmarks measure the performance of critical distribution operations:
//! - Bundle creation and signing
//! - Bundle verification
//! - Checksum computation
//! - Serialization/deserialization
//!
//! Run with: `cargo bench -p eunomia-distributor`
//!
//! ## Latency SLO Targets
//!
//! | Operation             | p50    | p95    | p99    |
//! |-----------------------|--------|--------|--------|
//! | Bundle Creation       | < 1ms  | < 5ms  | < 10ms |
//! | Bundle Signing        | < 1ms  | < 2ms  | < 5ms  |
//! | Bundle Verification   | < 1ms  | < 2ms  | < 5ms  |
//! | Checksum Computation  | < 500µs| < 1ms  | < 2ms  |
//! | Serialization         | < 1ms  | < 2ms  | < 5ms  |
//! | Deserialization       | < 1ms  | < 2ms  | < 5ms  |

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use eunomia_core::{Bundle, BundleSigner, BundleVerifier, SigningKeyPair};

// =============================================================================
// Test Data
// =============================================================================

/// Sample policy for benchmarking - a realistic authorization policy
const SAMPLE_POLICY: &str = r#"
# METADATA
# title: Users Service Authorization
# description: Authorization policy for users service
# scope: service
package users_service.authz

import future.keywords.if
import future.keywords.in

# Default deny
default allow := false

# Admin access - full permissions
allow if {
    input.caller.type == "user"
    "admin" in input.caller.roles
}

# Service account access - limited to read operations
allow if {
    input.caller.type == "spiffe"
    input.caller.service_name in allowed_services
    input.operation_id in read_operations
}

# User self-service access
allow if {
    input.caller.type == "user"
    input.operation_id in self_service_operations
    input.context.target_user_id == input.caller.user_id
}

# Manager access to subordinates
allow if {
    input.caller.type == "user"
    "manager" in input.caller.roles
    input.context.target_user_id in input.caller.subordinates
    input.operation_id in manager_operations
}

# Define allowed operations
read_operations := {"getUser", "listUsers", "getUserProfile"}
self_service_operations := {"getProfile", "updateProfile", "changePassword"}
manager_operations := {"getUser", "updateUser", "listUsers", "approveRequest"}
allowed_services := {"orders-service", "billing-service", "notification-service"}
"#;

/// Larger policy for stress testing
const LARGE_POLICY: &str = r#"
# METADATA
# title: Complex Multi-Service Authorization
# description: Complex authorization with many rules
# scope: service
package complex_service.authz

import future.keywords.if
import future.keywords.in

default allow := false

# Rule 1: Admin full access
allow if admin_access

# Rule 2: Service mesh access
allow if service_mesh_access

# Rule 3: User role-based access
allow if user_role_access

# Rule 4: Resource owner access
allow if resource_owner_access

# Rule 5: Time-based access
allow if time_based_access

# Helper rules
admin_access if {
    input.caller.type == "user"
    "admin" in input.caller.roles
}

service_mesh_access if {
    input.caller.type == "spiffe"
    trusted_service(input.caller.service_name)
    allowed_operation(input.operation_id, input.caller.service_name)
}

user_role_access if {
    input.caller.type == "user"
    required_role := operation_roles[input.operation_id]
    required_role in input.caller.roles
}

resource_owner_access if {
    input.caller.type == "user"
    input.context.resource_owner == input.caller.user_id
    input.operation_id in owner_operations
}

time_based_access if {
    input.caller.type == "user"
    current_hour := time.clock(time.now_ns())[0]
    current_hour >= 9
    current_hour < 17
    input.operation_id in business_hours_operations
}

# Data definitions
trusted_service(service) if service in {
    "orders-service",
    "billing-service",
    "inventory-service",
    "notification-service",
    "analytics-service",
    "search-service",
    "recommendation-service",
    "payment-service",
    "shipping-service",
    "customer-service"
}

allowed_operation(op, service) if {
    service_permissions[service][_] == op
}

operation_roles := {
    "createOrder": "customer",
    "viewOrder": "customer",
    "cancelOrder": "customer",
    "processOrder": "operator",
    "shipOrder": "operator",
    "refundOrder": "supervisor",
    "auditOrders": "auditor",
    "configureSystem": "admin"
}

service_permissions := {
    "orders-service": ["getUser", "getUserCredits"],
    "billing-service": ["getUser", "chargeUser"],
    "inventory-service": ["getUser"],
    "notification-service": ["getUser", "getUserPreferences"],
    "analytics-service": ["getUser", "getUserActivity"],
    "search-service": ["getUser"],
    "recommendation-service": ["getUser", "getUserHistory"],
    "payment-service": ["getUser", "getUserPaymentMethods"],
    "shipping-service": ["getUser", "getUserAddress"],
    "customer-service": ["getUser", "updateUser"]
}

owner_operations := {"viewResource", "editResource", "deleteResource"}
business_hours_operations := {"submitTimesheet", "requestPTO", "viewSchedule"}
"#;

// =============================================================================
// Helper Functions
// =============================================================================

/// Create a small bundle for benchmarking
fn create_small_bundle() -> Bundle {
    Bundle::builder("bench-service")
        .version("1.0.0")
        .add_policy("users_service.authz", SAMPLE_POLICY)
        .build()
}

/// Create a larger bundle for stress testing
fn create_large_bundle() -> Bundle {
    Bundle::builder("bench-service")
        .version("1.0.0")
        .add_policy("users_service.authz", SAMPLE_POLICY)
        .add_policy("complex_service.authz", LARGE_POLICY)
        .build()
}

/// Create bundles of different sizes for scaling tests
fn create_bundle_with_policies(policy_count: usize) -> Bundle {
    let mut builder = Bundle::builder("bench-service").version("1.0.0");

    for i in 0..policy_count {
        let policy_name = format!("policy_{}.authz", i);
        builder = builder.add_policy(policy_name, SAMPLE_POLICY);
    }

    builder.build()
}

// =============================================================================
// Benchmarks
// =============================================================================

fn bench_bundle_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("bundle_creation");

    group.bench_function("small_bundle", |b| {
        b.iter(|| black_box(create_small_bundle()))
    });

    group.bench_function("large_bundle", |b| {
        b.iter(|| black_box(create_large_bundle()))
    });

    // Benchmark different bundle sizes
    for policy_count in [1, 5, 10, 20].iter() {
        group.bench_with_input(
            BenchmarkId::new("policies", policy_count),
            policy_count,
            |b, &count| b.iter(|| black_box(create_bundle_with_policies(count))),
        );
    }

    group.finish();
}

fn bench_bundle_signing(c: &mut Criterion) {
    let mut group = c.benchmark_group("bundle_signing");

    let small_bundle = create_small_bundle();
    let large_bundle = create_large_bundle();
    let key_pair = SigningKeyPair::generate();
    let signer = BundleSigner::from_key_pair(&key_pair, "bench-key".to_string());

    group.bench_function("small_bundle", |b| {
        b.iter(|| black_box(signer.sign(&small_bundle)))
    });

    group.bench_function("large_bundle", |b| {
        b.iter(|| black_box(signer.sign(&large_bundle)))
    });

    // Benchmark signing different bundle sizes
    for policy_count in [1, 5, 10, 20].iter() {
        let bundle = create_bundle_with_policies(*policy_count);
        group.bench_with_input(
            BenchmarkId::new("policies", policy_count),
            &bundle,
            |b, bundle| b.iter(|| black_box(signer.sign(bundle))),
        );
    }

    group.finish();
}

fn bench_bundle_verification(c: &mut Criterion) {
    let mut group = c.benchmark_group("bundle_verification");

    let small_bundle = create_small_bundle();
    let large_bundle = create_large_bundle();
    let key_pair = SigningKeyPair::generate();
    let signer = BundleSigner::from_key_pair(&key_pair, "bench-key".to_string());

    let small_signed = signer.sign(&small_bundle);
    let large_signed = signer.sign(&large_bundle);

    let mut verifier = BundleVerifier::new();
    verifier.add_public_key("bench-key", key_pair.verifying_key());

    group.bench_function("small_bundle", |b| {
        b.iter(|| black_box(verifier.verify(&small_signed)))
    });

    group.bench_function("large_bundle", |b| {
        b.iter(|| black_box(verifier.verify(&large_signed)))
    });

    group.finish();
}

fn bench_checksum_computation(c: &mut Criterion) {
    let mut group = c.benchmark_group("checksum_computation");

    let small_bundle = create_small_bundle();
    let large_bundle = create_large_bundle();

    group.bench_function("small_bundle", |b| {
        b.iter(|| black_box(small_bundle.compute_checksum()))
    });

    group.bench_function("large_bundle", |b| {
        b.iter(|| black_box(large_bundle.compute_checksum()))
    });

    // Benchmark different bundle sizes
    for policy_count in [1, 5, 10, 20, 50].iter() {
        let bundle = create_bundle_with_policies(*policy_count);
        group.bench_with_input(
            BenchmarkId::new("policies", policy_count),
            &bundle,
            |b, bundle| b.iter(|| black_box(bundle.compute_checksum())),
        );
    }

    group.finish();
}

fn bench_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("serialization");

    let small_bundle = create_small_bundle();
    let large_bundle = create_large_bundle();

    group.bench_function("small_bundle_to_bytes", |b| {
        b.iter(|| black_box(small_bundle.to_bytes()))
    });

    group.bench_function("large_bundle_to_bytes", |b| {
        b.iter(|| black_box(large_bundle.to_bytes()))
    });

    // Benchmark deserialization
    let small_bytes = small_bundle.to_bytes().unwrap();
    let large_bytes = large_bundle.to_bytes().unwrap();

    group.bench_function("small_bundle_from_bytes", |b| {
        b.iter(|| black_box(Bundle::from_bytes(&small_bytes)))
    });

    group.bench_function("large_bundle_from_bytes", |b| {
        b.iter(|| black_box(Bundle::from_bytes(&large_bytes)))
    });

    group.finish();
}

fn bench_key_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("key_operations");

    group.bench_function("key_pair_generation", |b| {
        b.iter(|| black_box(SigningKeyPair::generate()))
    });

    let key_pair = SigningKeyPair::generate();
    let base64_key = key_pair.to_base64();

    group.bench_function("key_from_base64", |b| {
        b.iter(|| black_box(SigningKeyPair::from_base64(&base64_key)))
    });

    let seed = key_pair.to_bytes();
    group.bench_function("key_from_seed", |b| {
        b.iter(|| black_box(SigningKeyPair::from_seed(&seed)))
    });

    group.finish();
}

fn bench_end_to_end(c: &mut Criterion) {
    let mut group = c.benchmark_group("end_to_end");

    group.bench_function("full_workflow_small", |b| {
        b.iter(|| {
            // Create bundle
            let bundle = create_small_bundle();

            // Sign
            let key_pair = SigningKeyPair::generate();
            let signer = BundleSigner::from_key_pair(&key_pair, "e2e-key".to_string());
            let signed = signer.sign(&bundle);

            // Serialize
            let _bytes = signed.bundle.to_bytes();

            // Verify
            let mut verifier = BundleVerifier::new();
            verifier.add_public_key("e2e-key", key_pair.verifying_key());
            let _result = verifier.verify(&signed);

            black_box(())
        })
    });

    // Pre-generate key for workflow that reuses keys
    let key_pair = SigningKeyPair::generate();
    let signer = BundleSigner::from_key_pair(&key_pair, "e2e-key".to_string());
    let mut verifier = BundleVerifier::new();
    verifier.add_public_key("e2e-key", key_pair.verifying_key());

    group.bench_function("full_workflow_reuse_key", |b| {
        b.iter(|| {
            // Create bundle
            let bundle = create_small_bundle();

            // Sign (reusing signer)
            let signed = signer.sign(&bundle);

            // Serialize
            let bytes = signed.bundle.to_bytes().unwrap();

            // Deserialize
            let restored = Bundle::from_bytes(&bytes).unwrap();

            // Re-sign and verify
            let re_signed = signer.sign(&restored);
            let _result = verifier.verify(&re_signed);

            black_box(())
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_bundle_creation,
    bench_bundle_signing,
    bench_bundle_verification,
    bench_checksum_computation,
    bench_serialization,
    bench_key_operations,
    bench_end_to_end,
);

criterion_main!(benches);
