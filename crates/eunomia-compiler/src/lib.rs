//! # Eunomia Compiler
//!
//! Rego policy parsing and bundle compilation for the Eunomia authorization platform.
//!
//! This crate provides functionality for:
//!
//! - Parsing Rego policy files
//! - Real OPA/Rego evaluation using `regorus`
//! - Static analysis and validation
//! - Semantic validation with mock contracts
//! - Linting with configurable rules
//! - Bundle compilation
//! - Bundle optimization
//!
//! ## Example
//!
//! ```rust,ignore
//! use eunomia_compiler::{Parser, RegoEngine, Bundler, Linter, SemanticValidator};
//!
//! // Parse a policy file
//! let policy = Parser::parse_file("policies/authz.rego")?;
//!
//! // Lint the policy for issues
//! let linter = Linter::new();
//! let violations = linter.lint(&policy.source, "authz.rego");
//! for v in &violations {
//!     println!("[{}] {}", v.severity.as_str(), v.message);
//! }
//!
//! // Run semantic validation with mock contracts
//! let mut semantic = SemanticValidator::new();
//! semantic.register_operation("getUser");
//! let issues = semantic.validate_source(&policy.source, "authz.rego");
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
pub mod lint;
pub mod optimizer;
pub mod parser;
pub mod semantic;
pub mod validator;

pub use analyzer::Analyzer;
pub use bundler::Bundler;
pub use engine::{EvalResult, PolicyInfo, RegoEngine, TestRule};
pub use error::{CompilerError, Result};
pub use lint::{LintRule, LintViolation, Linter, RuleCategory, Severity};
pub use parser::Parser;
pub use semantic::{
    InputSchema, MockServiceContract, SemanticCategory, SemanticIssue, SemanticSeverity,
    SemanticValidator,
};
pub use validator::{
    validate_file, validate_source, IssueCategory, IssueSeverity, PolicyValidator, ValidationIssue,
    ValidationReport, ValidatorConfig,
};
