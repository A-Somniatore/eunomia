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
//! use eunomia_test::{TestRunner, TestConfig};
//!
//! let runner = TestRunner::new(TestConfig::default());
//! let results = runner.run_directory("policies/")?;
//!
//! println!("Passed: {}, Failed: {}", results.passed(), results.failed());
//! ```

pub mod coverage;
pub mod error;
pub mod fixtures;
pub mod reporter;
pub mod runner;

pub use error::{TestError, Result};
pub use fixtures::TestFixture;
pub use reporter::{Reporter, ConsoleReporter};
pub use runner::{TestRunner, TestConfig, TestResult, TestResults};
