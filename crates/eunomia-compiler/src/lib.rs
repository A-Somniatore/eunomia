//! # Eunomia Compiler
//!
//! Rego policy parsing and bundle compilation for the Eunomia authorization platform.
//!
//! This crate provides functionality for:
//!
//! - Parsing Rego policy files
//! - Static analysis and validation
//! - Bundle compilation
//! - Bundle optimization
//!
//! ## Example
//!
//! ```rust,ignore
//! use eunomia_compiler::{Parser, Bundler};
//!
//! // Parse a policy file
//! let policy = Parser::parse_file("policies/authz.rego")?;
//!
//! // Compile into a bundle
//! let bundle = Bundler::new("users-service")
//!     .add_policy(policy)
//!     .compile()?;
//! ```

pub mod analyzer;
pub mod bundler;
pub mod error;
pub mod optimizer;
pub mod parser;

pub use analyzer::Analyzer;
pub use bundler::Bundler;
pub use error::{CompilerError, Result};
pub use parser::Parser;
