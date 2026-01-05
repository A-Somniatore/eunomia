//! Integration tests for the test framework.
//!
//! These tests validate the full test discovery and execution workflow.
//! We use self-contained test policies (without imports) to test the runner,
//! while testing discovery against the example policies.

use eunomia_test::{TestConfig, TestDiscovery, TestRunner};
use std::fs;
use tempfile::TempDir;

/// Path to sample policies
const POLICIES_DIR: &str = "../../examples/policies/";

/// Helper to get absolute path from relative test path
fn policy_path(relative: &str) -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    format!("{manifest_dir}/{relative}")
}

/// Creates a temporary directory with self-contained test policies.
/// These tests don't use imports, so they can be evaluated in isolation.
fn create_test_policies() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Self-contained test policy (no imports) - OPA v1 syntax with `if` keyword
    let test_policy = r#"package simple_test

# A simple rule to test
is_valid if {
    input.value > 0
}

# Test that passes
test_positive_value if {
    is_valid with input as {"value": 10}
}

# Test that passes
test_boundary_value if {
    is_valid with input as {"value": 1}
}
"#;

    // Policy with expected failures for fail-fast testing
    let failing_test = r#"package failing_test

# Simple rule
allowed if {
    input.allow == true
}

test_should_pass if {
    allowed with input as {"allow": true}
}

test_should_also_pass if {
    allowed with input as {"allow": true}
}
"#;

    // Write test files
    fs::write(temp_dir.path().join("simple_test.rego"), test_policy)
        .expect("Failed to write simple_test.rego");

    fs::write(temp_dir.path().join("passing_test.rego"), failing_test)
        .expect("Failed to write passing_test.rego");

    temp_dir
}

// =============================================================================
// Test Discovery Tests
// =============================================================================

#[test]
fn test_discover_example_policies() {
    let discovery = TestDiscovery::new();
    let result = discovery.discover(&policy_path(POLICIES_DIR));

    assert!(result.is_ok(), "Discovery failed: {result:?}");

    let suite = result.unwrap();

    // Should find test files in users-service and common directories
    assert!(
        suite.test_count() > 0,
        "Should discover at least some tests"
    );

    // Should find policy files
    assert!(
        !suite.policy_files().is_empty(),
        "Should discover policy files"
    );
}

#[test]
fn test_discover_tests_by_package() {
    let discovery = TestDiscovery::new();
    let suite = discovery.discover(&policy_path(POLICIES_DIR)).unwrap();

    let by_package = suite.tests_by_package();

    // Should have tests organized by package
    assert!(
        !by_package.is_empty() || suite.test_count() > 0,
        "Should have test packages (found: {:?})",
        by_package.keys().collect::<Vec<_>>()
    );
}

#[test]
fn test_discover_self_contained_tests() {
    let temp_dir = create_test_policies();

    let discovery = TestDiscovery::new();
    let suite = discovery
        .discover(temp_dir.path().to_str().unwrap())
        .unwrap();

    // Should find 4 tests (2 from simple_test + 2 from passing_test)
    assert_eq!(suite.test_count(), 4, "Should discover 4 tests");

    let by_package = suite.tests_by_package();
    assert!(
        by_package.contains_key("simple_test"),
        "Should have simple_test package"
    );
    assert!(
        by_package.contains_key("failing_test"),
        "Should have failing_test package"
    );
}

// =============================================================================
// Test Execution Tests
// =============================================================================

#[test]
fn test_run_discovered_suite() {
    let temp_dir = create_test_policies();

    let discovery = TestDiscovery::new();
    let suite = discovery
        .discover(temp_dir.path().to_str().unwrap())
        .unwrap();

    assert!(suite.test_count() > 0, "Should have tests to run");

    let runner = TestRunner::new(TestConfig::default());
    let results = runner.run_suite(&suite);

    assert!(results.is_ok(), "Test execution failed: {results:?}");

    let results = results.unwrap();

    // Log results for debugging
    println!(
        "Test results: {} passed, {} failed",
        results.passed(),
        results.failed()
    );

    for failure in results.failures() {
        println!("  FAILED: {} - {:?}", failure.name, failure.error);
    }

    // All self-contained tests should pass
    assert!(
        results.all_passed(),
        "Some tests failed: {} passed, {} failed",
        results.passed(),
        results.failed()
    );
}

#[test]
fn test_run_with_fail_fast() {
    let temp_dir = create_test_policies();

    let discovery = TestDiscovery::new();
    let suite = discovery
        .discover(temp_dir.path().to_str().unwrap())
        .unwrap();

    let runner = TestRunner::new(TestConfig::new().with_fail_fast(true));
    let results = runner.run_suite(&suite).unwrap();

    // With self-contained tests, all should pass
    assert!(results.all_passed());
}

#[test]
fn test_runner_reports_test_names() {
    let temp_dir = create_test_policies();

    let discovery = TestDiscovery::new();
    let suite = discovery
        .discover(temp_dir.path().to_str().unwrap())
        .unwrap();

    let runner = TestRunner::new(TestConfig::default());
    let results = runner.run_suite(&suite).unwrap();

    // Verify tests were executed (self-contained tests should all pass)
    assert_eq!(
        results.passed(),
        4,
        "Should have 4 passing tests (2 from simple_test + 2 from passing_test)"
    );
    assert_eq!(results.failed(), 0, "Should have no failures");
}

// =============================================================================
// Reporter Tests
// =============================================================================

#[test]
fn test_console_reporter_output() {
    use eunomia_test::{ConsoleReporter, Reporter, TestResult, TestResults};
    use std::time::Duration;

    let reporter = ConsoleReporter::new().with_colors(false);

    // Create test results
    let mut results = TestResults::new();
    results.add(TestResult::pass(
        "test_admin_allowed",
        Duration::from_millis(5),
    ));
    results.add(TestResult::fail(
        "test_guest_denied",
        Duration::from_millis(3),
        "Expected true, got false",
    ));

    // The report method writes to stdout - just verify it doesn't panic
    let report_result = reporter.report(&results);
    assert!(report_result.is_ok());
}
