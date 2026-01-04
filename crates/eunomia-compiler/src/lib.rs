//! # Eunomia Compiler
//!
//! Rego policy parsing and bundle compilation for the Eunomia authorization platform.
//!
//! This crate provides functionality for:
//!
//! - Parsing Rego policy files
//! - Real OPA/Rego evaluation using `regorus`
//! - Static analysis and validation
//! - Bundle compilation
//! - Bundle optimization
//!
//! ## Example
//!
//! ```rust,ignore
//! use eunomia_compiler::{Parser, RegoEngine, Bundler};
//!
//! // Parse a policy file
//! let policy = Parser::parse_file("policies/authz.rego")?;
//!
//! // Or use the Rego engine for evaluation
//! let mut engine = RegoEngine::new();
//! engine.add_policy_from_file("policies/authz.rego")?;
//! engine.set_input_json(&serde_json::json!({
//!     "caller": { "type": "user", "roles": ["admin"] }
//! }))?;
//! let allowed = engine.eval_bool("data.authz.allow")?;
//!
//! // Compile into a bundle
//! let bundle = Bundler::new("users-service")
//!     .add_policy(policy)
//!     .compile()?;
//! ```

pub mod analyzer;
pub mod bundler;
pub mod engine;
pub mod error;
pub mod optimizer;
pub mod parser;

pub use analyzer::Analyzer;
pub use bundler::Bundler;
pub use engine::{EvalResult, PolicyInfo, RegoEngine, TestRule};
pub use error::{CompilerError, Result};
pub use parser::Parser;
