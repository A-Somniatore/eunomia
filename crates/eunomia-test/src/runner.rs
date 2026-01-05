//! Test runner for policy tests.
//!
//! This module provides the test execution engine for running Rego policy tests.
//!
//! # Overview
//!
//! The test runner supports two modes of test execution:
//!
//! 1. **Native Rego Tests**: Tests written as `test_*` rules in `*_test.rego` files
//! 2. **Fixture-Based Tests**: Tests defined in JSON/YAML fixtures
//!
//! # Example
//!
//! ```rust,ignore
//! use eunomia_test::{TestRunner, TestConfig, TestDiscovery};
//!
//! // Discover tests
//! let discovery = TestDiscovery::new();
//! let suite = discovery.discover("policies/")?;
//!
//! // Run tests
//! let runner = TestRunner::new(TestConfig::default());
//! let results = runner.run_suite(&suite)?;
//!
//! println!("Passed: {}, Failed: {}", results.passed(), results.failed());
//! ```

use std::time::{Duration, Instant};

use eunomia_compiler::RegoEngine;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::discovery::{DiscoveredTest, TestSuite};
use crate::error::{Result, TestError};
use crate::fixtures::TestFixture;

/// Configuration for the test runner.
#[derive(Debug, Clone)]
pub struct TestConfig {
    /// Whether to fail fast on first error.
    pub fail_fast: bool,
    /// Timeout for each test.
    pub timeout: Duration,
    /// Whether to run tests in parallel.
    pub parallel: bool,
    /// Number of parallel workers.
    pub workers: usize,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            fail_fast: false,
            timeout: Duration::from_secs(30),
            parallel: false,
            workers: 4,
        }
    }
}

impl TestConfig {
    /// Creates a new test configuration with default values.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets fail-fast mode.
    #[must_use]
    pub const fn with_fail_fast(mut self, fail_fast: bool) -> Self {
        self.fail_fast = fail_fast;
        self
    }

    /// Sets the test timeout.
    #[must_use]
    pub const fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Enables parallel test execution.
    #[must_use]
    pub const fn with_parallel(mut self, parallel: bool) -> Self {
        self.parallel = parallel;
        self
    }

    /// Sets the number of parallel workers.
    #[must_use]
    pub const fn with_workers(mut self, workers: usize) -> Self {
        self.workers = workers;
        self
    }
}

/// Result of a single test execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    /// Name of the test.
    pub name: String,
    /// Whether the test passed.
    pub passed: bool,
    /// Duration of the test.
    pub duration: Duration,
    /// Error message if the test failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Expected value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected: Option<String>,
    /// Actual value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actual: Option<String>,
}

impl TestResult {
    /// Creates a passing test result.
    #[must_use]
    pub fn pass(name: impl Into<String>, duration: Duration) -> Self {
        Self {
            name: name.into(),
            passed: true,
            duration,
            error: None,
            expected: None,
            actual: None,
        }
    }

    /// Creates a failing test result.
    #[must_use]
    pub fn fail(name: impl Into<String>, duration: Duration, error: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            passed: false,
            duration,
            error: Some(error.into()),
            expected: None,
            actual: None,
        }
    }

    /// Adds expected/actual values for assertion failures.
    #[must_use]
    pub fn with_comparison(
        mut self,
        expected: impl Into<String>,
        actual: impl Into<String>,
    ) -> Self {
        self.expected = Some(expected.into());
        self.actual = Some(actual.into());
        self
    }
}

/// Aggregated results from running multiple tests.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct TestResults {
    /// Individual test results.
    pub results: Vec<TestResult>,
    /// Total duration of test run.
    pub total_duration: Duration,
}

impl TestResults {
    /// Creates a new empty test results container.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a test result.
    pub fn add(&mut self, result: TestResult) {
        self.results.push(result);
    }

    /// Returns the number of passed tests.
    #[must_use]
    pub fn passed(&self) -> usize {
        self.results.iter().filter(|r| r.passed).count()
    }

    /// Returns the number of failed tests.
    #[must_use]
    pub fn failed(&self) -> usize {
        self.results.iter().filter(|r| !r.passed).count()
    }

    /// Returns the total number of tests.
    #[must_use]
    pub const fn total(&self) -> usize {
        self.results.len()
    }

    /// Returns true if all tests passed.
    #[must_use]
    pub fn all_passed(&self) -> bool {
        self.results.iter().all(|r| r.passed)
    }

    /// Returns an iterator over failed tests.
    pub fn failures(&self) -> impl Iterator<Item = &TestResult> {
        self.results.iter().filter(|r| !r.passed)
    }
}

/// Test runner for executing policy tests.
///
/// The runner supports two modes:
/// - Native Rego tests (`test_*` rules in `*_test.rego` files)
/// - Fixture-based tests with JSON/YAML input
///
/// # Examples
///
/// ```rust,ignore
/// use eunomia_test::{TestRunner, TestConfig, TestDiscovery};
///
/// let discovery = TestDiscovery::new();
/// let suite = discovery.discover("policies/")?;
///
/// let runner = TestRunner::new(TestConfig::default());
/// let results = runner.run_suite(&suite)?;
///
/// println!("Passed: {}, Failed: {}", results.passed(), results.failed());
/// ```
#[derive(Debug)]
pub struct TestRunner {
    config: TestConfig,
}

impl TestRunner {
    /// Creates a new test runner with the given configuration.
    #[must_use]
    pub const fn new(config: TestConfig) -> Self {
        Self { config }
    }

    /// Runs all tests in a discovered test suite.
    ///
    /// This method:
    /// 1. Creates a Rego engine
    /// 2. Loads all policy files
    /// 3. Executes each discovered test
    /// 4. Collects and returns results
    ///
    /// # Errors
    ///
    /// Returns an error if policy loading fails.
    pub fn run_suite(&self, suite: &TestSuite) -> Result<TestResults> {
        let start = Instant::now();
        let mut results = TestResults::new();

        info!(tests = suite.test_count(), "Running test suite");

        // Create a Rego engine and load all policies
        let mut engine = RegoEngine::new();

        // Load all policy files
        for (path, source) in suite.policy_files() {
            let name = path.to_string_lossy().to_string();
            debug!(file = %name, "Loading policy file");

            engine
                .add_policy(&name, source)
                .map_err(|e| TestError::ExecutionError {
                    message: format!("Failed to load policy {name}: {e}"),
                })?;
        }

        // Run each test
        for test in suite.tests() {
            let result = self.run_test(&mut engine, test);
            let failed = !result.passed;
            results.add(result);

            if self.config.fail_fast && failed {
                warn!("Stopping early due to fail-fast mode");
                break;
            }
        }

        results.total_duration = start.elapsed();
        info!(
            passed = results.passed(),
            failed = results.failed(),
            duration = ?results.total_duration,
            "Test suite complete"
        );

        Ok(results)
    }

    /// Runs a single discovered test.
    #[allow(clippy::unused_self)]
    fn run_test(&self, engine: &mut RegoEngine, test: &DiscoveredTest) -> TestResult {
        let start = Instant::now();

        debug!(test = %test.qualified_name, "Running test");

        // Evaluate the test rule
        match engine.eval_bool(&test.qualified_name) {
            Ok(passed) => {
                let duration = start.elapsed();
                if passed {
                    debug!(test = %test.name, duration = ?duration, "Test passed");
                    TestResult::pass(&test.name, duration)
                } else {
                    debug!(test = %test.name, duration = ?duration, "Test failed");
                    TestResult::fail(&test.name, duration, "Test rule evaluated to false")
                        .with_comparison("true", "false")
                }
            }
            Err(e) => {
                let duration = start.elapsed();
                warn!(test = %test.name, error = %e, "Test execution error");
                TestResult::fail(&test.name, duration, format!("Evaluation error: {e}"))
            }
        }
    }

    /// Runs a single test fixture against a policy.
    ///
    /// This evaluates the policy with the fixture's input and compares
    /// the result to the expected outcome.
    pub fn run_fixture(&self, fixture: &TestFixture, policy_source: &str) -> TestResult {
        let start = Instant::now();
        let name = fixture.name.clone();

        debug!(test = %name, "Running test fixture");

        // Create engine and load policy
        let mut engine = RegoEngine::new();
        if let Err(e) = engine.add_policy("test", policy_source) {
            return TestResult::fail(
                &name,
                start.elapsed(),
                format!("Failed to load policy: {e}"),
            );
        }

        // Set input from fixture
        if let Err(e) = engine.set_input_json(&fixture.input) {
            return TestResult::fail(&name, start.elapsed(), format!("Failed to set input: {e}"));
        }

        // Evaluate the allow rule
        let query = "data.test.allow";
        match engine.eval_bool(query) {
            Ok(allowed) => {
                let duration = start.elapsed();
                if allowed == fixture.expected_allowed {
                    info!(test = %name, duration = ?duration, "Test passed");
                    TestResult::pass(name, duration)
                } else {
                    info!(test = %name, duration = ?duration, "Test failed - mismatch");
                    TestResult::fail(&name, duration, "Allow decision mismatch")
                        .with_comparison(fixture.expected_allowed.to_string(), allowed.to_string())
                }
            }
            Err(e) => {
                let duration = start.elapsed();
                warn!(test = %name, error = %e, "Test execution error");
                TestResult::fail(name, duration, format!("Evaluation error: {e}"))
            }
        }
    }

    /// Runs all fixtures in a set against a policy.
    ///
    /// # Errors
    ///
    /// Returns an error if test execution fails.
    pub fn run_fixture_set(
        &self,
        fixtures: &[TestFixture],
        policy_source: &str,
    ) -> Result<TestResults> {
        let start = Instant::now();
        let mut results = TestResults::new();

        for fixture in fixtures {
            let result = self.run_fixture(fixture, policy_source);
            let failed = !result.passed;
            results.add(result);

            if self.config.fail_fast && failed {
                warn!("Stopping early due to fail-fast mode");
                break;
            }
        }

        results.total_duration = start.elapsed();
        Ok(results)
    }

    /// Returns the test configuration.
    #[must_use]
    pub const fn config(&self) -> &TestConfig {
        &self.config
    }
}

impl Default for TestRunner {
    fn default() -> Self {
        Self::new(TestConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_config_defaults() {
        let config = TestConfig::default();
        assert!(!config.fail_fast);
        assert!(!config.parallel);
        assert_eq!(config.workers, 4);
    }

    #[test]
    fn test_config_builder() {
        let config = TestConfig::new()
            .with_fail_fast(true)
            .with_parallel(true)
            .with_workers(8)
            .with_timeout(Duration::from_secs(60));

        assert!(config.fail_fast);
        assert!(config.parallel);
        assert_eq!(config.workers, 8);
        assert_eq!(config.timeout, Duration::from_secs(60));
    }

    #[test]
    fn test_result_pass() {
        let result = TestResult::pass("my_test", Duration::from_millis(100));
        assert!(result.passed);
        assert!(result.error.is_none());
    }

    #[test]
    fn test_result_fail() {
        let result = TestResult::fail("my_test", Duration::from_millis(100), "assertion failed");
        assert!(!result.passed);
        assert_eq!(result.error, Some("assertion failed".to_string()));
    }

    #[test]
    fn test_result_with_comparison() {
        let result = TestResult::fail("my_test", Duration::from_millis(100), "mismatch")
            .with_comparison("true", "false");

        assert_eq!(result.expected, Some("true".to_string()));
        assert_eq!(result.actual, Some("false".to_string()));
    }

    #[test]
    fn test_results_aggregation() {
        let mut results = TestResults::new();
        results.add(TestResult::pass("test1", Duration::from_millis(10)));
        results.add(TestResult::pass("test2", Duration::from_millis(20)));
        results.add(TestResult::fail(
            "test3",
            Duration::from_millis(30),
            "error",
        ));

        assert_eq!(results.total(), 3);
        assert_eq!(results.passed(), 2);
        assert_eq!(results.failed(), 1);
        assert!(!results.all_passed());
    }

    #[test]
    fn test_results_all_passed() {
        let mut results = TestResults::new();
        results.add(TestResult::pass("test1", Duration::from_millis(10)));
        results.add(TestResult::pass("test2", Duration::from_millis(20)));

        assert!(results.all_passed());
    }

    #[test]
    fn test_runner_basic() {
        let runner = TestRunner::default();

        // Test with a policy that allows based on caller type
        let policy = r#"
package test

default allow := false

allow if {
    input.caller.type == "admin"
}
"#;

        // Test that admin is allowed
        let admin_fixture = TestFixture::new("admin_allowed")
            .with_input(json!({"caller": {"type": "admin"}}))
            .expect_allowed(true);

        let result = runner.run_fixture(&admin_fixture, policy);
        assert!(result.passed, "Admin should be allowed: {:?}", result.error);

        // Test that guest is denied
        let guest_fixture = TestFixture::new("guest_denied")
            .with_input(json!({"caller": {"type": "guest"}}))
            .expect_allowed(false);

        let result = runner.run_fixture(&guest_fixture, policy);
        assert!(result.passed, "Guest should be denied: {:?}", result.error);
    }

    #[test]
    fn test_runner_fixture_mismatch() {
        let runner = TestRunner::default();

        let policy = r#"
package test

default allow := false
"#;

        // Expect allowed but policy denies everything
        let fixture = TestFixture::new("mismatch")
            .with_input(json!({"caller": {"type": "admin"}}))
            .expect_allowed(true);

        let result = runner.run_fixture(&fixture, policy);
        assert!(!result.passed, "Should fail due to mismatch");
        assert!(result.error.is_some());
    }
}
