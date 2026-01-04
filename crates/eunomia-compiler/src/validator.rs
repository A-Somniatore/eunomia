//! Policy validation combining parsing, analysis, and linting.
//!
//! This module provides a unified validation interface that combines:
//! - Rego syntax validation (via regorus)
//! - Static analysis (via the analyzer)
//! - Linting rules (via the linter)
//!
//! # Example
//!
//! ```rust,ignore
//! use eunomia_compiler::{PolicyValidator, ValidationReport};
//!
//! let validator = PolicyValidator::new();
//! let report = validator.validate_file("policies/authz.rego")?;
//!
//! if report.is_valid() {
//!     println!("Policy is valid!");
//! } else {
//!     for error in &report.errors {
//!         println!("Error: {}", error.message);
//!     }
//! }
//! ```

use std::fs;
use std::path::Path;

use crate::analyzer::{AnalysisResult, Analyzer};
use crate::engine::RegoEngine;
use crate::error::{CompilerError, Result};
use crate::lint::{Linter, Severity};
use crate::parser::Parser;

use eunomia_core::Policy;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

/// Configuration for policy validation.
#[derive(Debug, Clone)]
#[allow(clippy::struct_excessive_bools)]
pub struct ValidatorConfig {
    /// Whether to run the Rego parser for syntax validation.
    pub check_syntax: bool,
    /// Whether to run static analysis.
    pub run_analysis: bool,
    /// Whether to run linting.
    pub run_linting: bool,
    /// Whether validation should fail on warnings.
    pub fail_on_warnings: bool,
    /// Whether to require a default deny rule.
    pub require_default_deny: bool,
    /// Linting rules to disable.
    pub disabled_lint_rules: Vec<String>,
}

impl Default for ValidatorConfig {
    fn default() -> Self {
        Self {
            check_syntax: true,
            run_analysis: true,
            run_linting: true,
            fail_on_warnings: false,
            require_default_deny: true,
            disabled_lint_rules: Vec::new(),
        }
    }
}

impl ValidatorConfig {
    /// Creates a strict configuration that fails on warnings.
    #[must_use]
    pub fn strict() -> Self {
        Self {
            fail_on_warnings: true,
            ..Default::default()
        }
    }

    /// Creates a lenient configuration with minimal checks.
    #[must_use]
    pub fn lenient() -> Self {
        Self {
            run_linting: false,
            fail_on_warnings: false,
            require_default_deny: false,
            ..Default::default()
        }
    }
}

/// A validation issue found during policy validation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationIssue {
    /// Severity of the issue.
    pub severity: IssueSeverity,
    /// The category of the issue.
    pub category: IssueCategory,
    /// Human-readable message.
    pub message: String,
    /// Line number (1-based, if applicable).
    pub line: Option<usize>,
    /// Source file path.
    pub file: Option<String>,
    /// Rule ID (if from linting).
    pub rule_id: Option<String>,
    /// Suggestion for fixing the issue.
    pub suggestion: Option<String>,
}

/// Severity of a validation issue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum IssueSeverity {
    /// Informational hint.
    Hint,
    /// Warning - may indicate a problem.
    Warning,
    /// Error - validation failure.
    Error,
}

impl From<Severity> for IssueSeverity {
    fn from(s: Severity) -> Self {
        match s {
            Severity::Hint => Self::Hint,
            Severity::Warning => Self::Warning,
            Severity::Error => Self::Error,
        }
    }
}

/// Category of validation issues.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IssueCategory {
    /// Syntax errors.
    Syntax,
    /// Analysis findings.
    Analysis,
    /// Linting violations.
    Lint,
    /// I/O errors.
    Io,
}

/// Result of validating a policy.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ValidationReport {
    /// Path to the validated file.
    pub file: Option<String>,
    /// Package name (if successfully parsed).
    pub package: Option<String>,
    /// All validation issues found.
    pub issues: Vec<ValidationIssue>,
    /// Analysis result (if analysis was run).
    #[serde(skip)]
    pub analysis: Option<AnalysisResult>,
}

impl ValidationReport {
    /// Creates a new empty report.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a report for a specific file.
    #[must_use]
    pub fn for_file(file: impl Into<String>) -> Self {
        Self {
            file: Some(file.into()),
            ..Default::default()
        }
    }

    /// Adds an issue to the report.
    pub fn add_issue(&mut self, issue: ValidationIssue) {
        self.issues.push(issue);
    }

    /// Returns whether validation passed (no errors).
    #[must_use]
    pub fn is_valid(&self) -> bool {
        !self
            .issues
            .iter()
            .any(|i| i.severity == IssueSeverity::Error)
    }

    /// Returns whether validation passed strictly (no errors or warnings).
    #[must_use]
    pub fn is_valid_strict(&self) -> bool {
        !self
            .issues
            .iter()
            .any(|i| i.severity >= IssueSeverity::Warning)
    }

    /// Returns only error-level issues.
    #[must_use]
    pub fn errors(&self) -> Vec<&ValidationIssue> {
        self.issues
            .iter()
            .filter(|i| i.severity == IssueSeverity::Error)
            .collect()
    }

    /// Returns only warning-level issues.
    #[must_use]
    pub fn warnings(&self) -> Vec<&ValidationIssue> {
        self.issues
            .iter()
            .filter(|i| i.severity == IssueSeverity::Warning)
            .collect()
    }

    /// Returns the count of issues by severity.
    #[must_use]
    pub fn counts(&self) -> (usize, usize, usize) {
        let errors = self.errors().len();
        let warnings = self.warnings().len();
        let hints = self
            .issues
            .iter()
            .filter(|i| i.severity == IssueSeverity::Hint)
            .count();
        (errors, warnings, hints)
    }
}

/// Validates Rego policies with comprehensive checks.
#[derive(Debug)]
pub struct PolicyValidator {
    config: ValidatorConfig,
    linter: Linter,
    analyzer: Analyzer,
}

impl Default for PolicyValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl PolicyValidator {
    /// Creates a new validator with default configuration.
    #[must_use]
    pub fn new() -> Self {
        Self::with_config(ValidatorConfig::default())
    }

    /// Creates a new validator with the given configuration.
    #[must_use]
    pub fn with_config(config: ValidatorConfig) -> Self {
        let mut linter = Linter::new();

        // Disable any specified lint rules
        for rule_id in &config.disabled_lint_rules {
            // Note: We need to leak the string to get a static reference
            // This is acceptable since rules are typically disabled once at startup
            let static_id: &'static str = Box::leak(rule_id.clone().into_boxed_str());
            linter.disable_rule(static_id);
        }

        let analyzer = Analyzer::new().with_require_default(config.require_default_deny);

        Self {
            config,
            linter,
            analyzer,
        }
    }

    /// Creates a strict validator that fails on warnings.
    #[must_use]
    pub fn strict() -> Self {
        Self::with_config(ValidatorConfig::strict())
    }

    /// Validates a policy file.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the Rego policy file
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read.
    pub fn validate_file(&self, path: impl AsRef<Path>) -> Result<ValidationReport> {
        let path = path.as_ref();
        let file_name = path.to_string_lossy().to_string();

        info!(file = %file_name, "Validating policy file");

        let mut report = ValidationReport::for_file(&file_name);

        // Read the file
        let source = match fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) => {
                report.add_issue(ValidationIssue {
                    severity: IssueSeverity::Error,
                    category: IssueCategory::Io,
                    message: format!("Failed to read file: {e}"),
                    line: None,
                    file: Some(file_name),
                    rule_id: None,
                    suggestion: None,
                });
                return Ok(report);
            }
        };

        self.validate_source(&source, &file_name, &mut report);
        Ok(report)
    }

    /// Validates a policy from source code.
    ///
    /// # Arguments
    ///
    /// * `source` - Rego source code
    /// * `file_name` - Name of the source (for error messages)
    #[must_use]
    pub fn validate_source_str(&self, source: &str, file_name: &str) -> ValidationReport {
        let mut report = ValidationReport::for_file(file_name);
        self.validate_source(source, file_name, &mut report);
        report
    }

    fn validate_source(&self, source: &str, file_name: &str, report: &mut ValidationReport) {
        // Step 1: Syntax validation with regorus
        if self.config.check_syntax {
            debug!("Running syntax validation");
            Self::check_syntax(source, file_name, report);
        }

        // Step 2: Parse with our parser for metadata
        let policy = match Parser::new().parse_source(source, file_name) {
            Ok(p) => {
                report.package = Some(p.package_name.clone());
                Some(p)
            }
            Err(e) => {
                // Only add error if syntax check didn't already catch it
                if report.issues.is_empty() {
                    report.add_issue(ValidationIssue {
                        severity: IssueSeverity::Error,
                        category: IssueCategory::Syntax,
                        message: e.to_string(),
                        line: None,
                        file: Some(file_name.to_string()),
                        rule_id: None,
                        suggestion: None,
                    });
                }
                None
            }
        };

        // Step 3: Static analysis
        if self.config.run_analysis {
            if let Some(ref policy) = policy {
                debug!("Running static analysis");
                self.run_analysis(policy, file_name, report);
            }
        }

        // Step 4: Linting
        if self.config.run_linting {
            debug!("Running linting");
            self.run_linting(source, file_name, report);
        }
    }

    fn check_syntax(source: &str, file_name: &str, report: &mut ValidationReport) {
        let mut engine = RegoEngine::new();

        if let Err(e) = engine.add_policy(file_name, source) {
            let (line, message) = match &e {
                CompilerError::ParseError { line, message, .. } => (Some(*line), message.clone()),
                _ => (None, e.to_string()),
            };

            report.add_issue(ValidationIssue {
                severity: IssueSeverity::Error,
                category: IssueCategory::Syntax,
                message,
                line,
                file: Some(file_name.to_string()),
                rule_id: None,
                suggestion: Some("Check Rego syntax".to_string()),
            });
        }
    }

    fn run_analysis(&self, policy: &Policy, file_name: &str, report: &mut ValidationReport) {
        match self.analyzer.analyze(policy) {
            Ok(result) => {
                // Convert analysis warnings to issues
                for warning in &result.warnings {
                    report.add_issue(ValidationIssue {
                        severity: IssueSeverity::Warning,
                        category: IssueCategory::Analysis,
                        message: warning.message.clone(),
                        line: warning.line,
                        file: Some(file_name.to_string()),
                        rule_id: None,
                        suggestion: None,
                    });
                }
                report.analysis = Some(result);
            }
            Err(e) => {
                report.add_issue(ValidationIssue {
                    severity: IssueSeverity::Error,
                    category: IssueCategory::Analysis,
                    message: e.to_string(),
                    line: None,
                    file: Some(file_name.to_string()),
                    rule_id: None,
                    suggestion: None,
                });
            }
        }
    }

    fn run_linting(&self, source: &str, file_name: &str, report: &mut ValidationReport) {
        let violations = self.linter.lint(source, file_name);

        for violation in violations {
            // Skip lint errors for things already caught by analysis
            if report.issues.iter().any(|i| {
                i.category == IssueCategory::Analysis
                    && i.severity == IssueSeverity::Error
                    && violation.rule_id.contains("default-deny")
            }) {
                continue;
            }

            report.add_issue(ValidationIssue {
                severity: violation.severity.into(),
                category: IssueCategory::Lint,
                message: violation.message,
                line: violation.line,
                file: Some(file_name.to_string()),
                rule_id: Some(violation.rule_id.to_string()),
                suggestion: violation.suggestion,
            });
        }
    }

    /// Validates and returns a Result indicating success or failure.
    ///
    /// # Errors
    ///
    /// Returns an error if validation fails.
    pub fn validate_or_error(&self, path: impl AsRef<Path>) -> Result<ValidationReport> {
        let report = self.validate_file(path)?;

        let valid = if self.config.fail_on_warnings {
            report.is_valid_strict()
        } else {
            report.is_valid()
        };

        if !valid {
            let (errors, warnings, _) = report.counts();
            return Err(CompilerError::ValidationError {
                message: format!(
                    "Policy validation failed: {errors} error(s), {warnings} warning(s)"
                ),
            });
        }

        Ok(report)
    }
}

/// Convenience function to validate a single file.
///
/// # Errors
///
/// Returns an error if the file cannot be read.
pub fn validate_file(path: impl AsRef<Path>) -> Result<ValidationReport> {
    PolicyValidator::new().validate_file(path)
}

/// Convenience function to validate source code.
#[must_use]
pub fn validate_source(source: &str, file_name: &str) -> ValidationReport {
    PolicyValidator::new().validate_source_str(source, file_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_POLICY: &str = r#"
package test.authz

import future.keywords.if

default allow := false

allow if {
    input.caller.role == "admin"
}
"#;

    const INVALID_SYNTAX: &str = r#"
package test.authz

this is not valid rego
"#;

    const MISSING_DEFAULT: &str = r#"
package test.authz

import future.keywords.if

allow if {
    input.caller.role == "admin"
}
"#;

    const INSECURE_POLICY: &str = r#"
package test.authz

default allow := true
"#;

    #[test]
    fn test_validate_valid_policy() {
        let report = validate_source(VALID_POLICY, "valid.rego");

        assert!(report.is_valid());
        assert!(report.package.is_some());
        assert_eq!(report.package.as_ref().unwrap(), "test.authz");
    }

    #[test]
    fn test_validate_invalid_syntax() {
        let report = validate_source(INVALID_SYNTAX, "invalid.rego");

        assert!(!report.is_valid());
        assert!(report
            .errors()
            .iter()
            .any(|e| e.category == IssueCategory::Syntax));
    }

    #[test]
    fn test_validate_missing_default() {
        let validator = PolicyValidator::new();
        let report = validator.validate_source_str(MISSING_DEFAULT, "missing.rego");

        // Should have errors due to missing default
        assert!(!report.is_valid());
    }

    #[test]
    fn test_validate_insecure_policy() {
        let report = validate_source(INSECURE_POLICY, "insecure.rego");

        // Should have errors for default allow := true
        assert!(!report.is_valid());
        assert!(report
            .errors()
            .iter()
            .any(|e| e.message.contains("insecure") || e.message.contains("true")));
    }

    #[test]
    fn test_strict_validation() {
        let source = r#"
package test.authz

default allow := false

allow if {
    input.caller.role == "admin"
}
"#;
        let validator = PolicyValidator::strict();
        let report = validator.validate_source_str(source, "test.rego");

        // Has warnings for missing import
        assert!(!report.is_valid_strict());
        assert!(report.warnings().len() > 0);
    }

    #[test]
    fn test_lenient_validation() {
        let validator = PolicyValidator::with_config(ValidatorConfig::lenient());
        let report = validator.validate_source_str(MISSING_DEFAULT, "missing.rego");

        // Lenient mode doesn't require default
        assert!(report.is_valid());
    }

    #[test]
    fn test_validation_report_counts() {
        let report = validate_source(INSECURE_POLICY, "insecure.rego");

        let (errors, warnings, hints) = report.counts();
        assert!(errors > 0 || warnings > 0);
        // Just check it doesn't panic
        let _ = hints;
    }

    #[test]
    fn test_config_builder() {
        let config = ValidatorConfig {
            check_syntax: true,
            run_analysis: false,
            run_linting: true,
            fail_on_warnings: true,
            require_default_deny: false,
            disabled_lint_rules: vec!["style/explicit-imports".to_string()],
        };

        let validator = PolicyValidator::with_config(config);

        // Should work without crashing
        let _ = validator.validate_source_str("package test\ndefault allow := false", "test.rego");
    }
}
