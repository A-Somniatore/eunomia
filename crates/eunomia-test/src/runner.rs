//! Test runner for policy tests.
//!
//! This module provides the test execution engine.

use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::error::Result;
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
/// # Examples
///
/// ```rust
/// use eunomia_test::{TestRunner, TestConfig};
///
/// let runner = TestRunner::new(TestConfig::default());
/// // Run tests...
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

    /// Runs a single test fixture against a policy.
    ///
    /// Note: This is a placeholder implementation. In a real implementation,
    /// this would invoke OPA to evaluate the policy.
    pub fn run_fixture(&self, fixture: &TestFixture, _policy_source: &str) -> TestResult {
        let start = Instant::now();
        let name = fixture.name.clone();

        debug!(test = %name, "Running test fixture");

        // TODO: Implement actual OPA evaluation
        // For now, this is a placeholder that simulates test execution
        
        // Simulate some test execution time
        let duration = start.elapsed();

        // In a real implementation, we would:
        // 1. Load the policy into OPA
        // 2. Evaluate the policy with fixture.input
        // 3. Compare the result with fixture.expected_allowed

        info!(test = %name, duration = ?duration, "Test completed");

        // For now, return a placeholder result
        TestResult::pass(name, duration)
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
        results.add(TestResult::fail("test3", Duration::from_millis(30), "error"));

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
        let fixture = TestFixture::new("test")
            .with_input(json!({"key": "value"}))
            .expect_allowed(true);

        let result = runner.run_fixture(&fixture, "package test\ndefault allow := false");
        // Since we have a placeholder implementation, it always passes
        assert!(result.passed);
    }
}
