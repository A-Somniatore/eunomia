//! # Eunomia Test
//!
//! Policy testing framework for the Eunomia authorization platform.
//!
//! This crate provides functionality for:
//!
//! - Discovering and running policy tests
//! - Managing test fixtures
//! - Generating coverage reports
//! - Reporting test results
//! - Mock identity builders for testing
//! - Test utilities and assertion helpers
//!
//! ## Example
//!
//! ```rust,ignore
//! use eunomia_test::{TestRunner, TestConfig, TestDiscovery};
//!
//! // Discover tests in a directory
//! let discovery = TestDiscovery::new();
//! let suite = discovery.discover("policies/")?;
//!
//! println!("Found {} tests", suite.test_count());
//!
//! // Run tests
//! let runner = TestRunner::new(TestConfig::default());
//! let results = runner.run_suite(&suite)?;
//!
//! println!("Passed: {}, Failed: {}", results.passed(), results.failed());
//! ```
//!
//! ## Mock Identities
//!
//! ```rust
//! use eunomia_test::{MockUser, MockSpiffe, MockApiKey};
//!
//! // Create mock identities for testing
//! let admin = MockUser::admin();
//! let service = MockSpiffe::orders_service();
//! let api_key = MockApiKey::read_only();
//! ```
//!
//! ## Test Utilities
//!
//! ```rust
//! use eunomia_test::{InputBuilder, MockUser};
//! use serde_json::json;
//!
//! // Build input using the fluent API
//! let input = InputBuilder::new()
//!     .caller(MockUser::admin())
//!     .operation("deleteUser")
//!     .method("DELETE")
//!     .path("/users/user-123")
//!     .service("users-service")
//!     .build();
//! ```

pub mod coverage;
pub mod discovery;
pub mod error;
pub mod fixtures;
pub mod mock_identity;
pub mod reporter;
pub mod runner;
pub mod test_utils;

pub use discovery::{DiscoveredTest, DiscoveryConfig, FixtureFormat, TestDiscovery, TestSuite};
pub use error::{Result, TestError};
pub use fixtures::TestFixture;
pub use mock_identity::{MockApiKey, MockSpiffe, MockUser};
pub use reporter::{ConsoleReporter, Reporter};
pub use runner::{TestConfig, TestResult, TestResults, TestRunner};
pub use test_utils::{
    assert_all_passed, assert_allowed, assert_denied, role_based_policy, scope_based_policy,
    simple_allow_policy, InputBuilder,
};
