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

pub mod coverage;
pub mod discovery;
pub mod error;
pub mod fixtures;
pub mod mock_identity;
pub mod reporter;
pub mod runner;

pub use discovery::{DiscoveredTest, DiscoveryConfig, FixtureFormat, TestDiscovery, TestSuite};
pub use error::{Result, TestError};
pub use fixtures::TestFixture;
pub use mock_identity::{MockApiKey, MockSpiffe, MockUser};
pub use reporter::{ConsoleReporter, Reporter};
pub use runner::{TestConfig, TestResult, TestResults, TestRunner};
