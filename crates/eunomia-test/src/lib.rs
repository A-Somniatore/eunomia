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

pub mod coverage;
pub mod discovery;
pub mod error;
pub mod fixtures;
pub mod reporter;
pub mod runner;

pub use discovery::{DiscoveredTest, DiscoveryConfig, FixtureFormat, TestDiscovery, TestSuite};
pub use error::{Result, TestError};
pub use fixtures::TestFixture;
pub use reporter::{ConsoleReporter, Reporter};
pub use runner::{TestConfig, TestResult, TestResults, TestRunner};
