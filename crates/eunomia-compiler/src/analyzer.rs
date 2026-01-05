//! Static analysis for Rego policies.
//!
//! This module provides validation and analysis of Rego policies.

use eunomia_core::Policy;
use tracing::warn;

use crate::error::{CompilerError, Result};

/// Static analyzer for Rego policies.
///
/// The analyzer validates policies and checks for common issues.
#[derive(Debug, Default)]
pub struct Analyzer {
    /// Whether to require a default decision.
    require_default: bool,
    /// Whether to warn about missing tests.
    warn_missing_tests: bool,
}

/// Result of analyzing a policy.
#[derive(Debug, Clone, Default)]
pub struct AnalysisResult {
    /// Warnings found during analysis.
    pub warnings: Vec<AnalysisWarning>,
    /// Whether the policy has a default allow rule.
    pub has_default_allow: bool,
    /// Whether the policy has a default deny rule.
    pub has_default_deny: bool,
    /// Imports found in the policy.
    pub imports: Vec<String>,
    /// Rules found in the policy.
    pub rules: Vec<String>,
}

/// A warning found during policy analysis.
#[derive(Debug, Clone)]
pub struct AnalysisWarning {
    /// Warning message.
    pub message: String,
    /// Line number (if applicable).
    pub line: Option<usize>,
}

impl Analyzer {
    /// Creates a new analyzer with default settings.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            require_default: true,
            warn_missing_tests: true,
        }
    }

    /// Sets whether to require a default decision rule.
    #[must_use]
    pub const fn with_require_default(mut self, require: bool) -> Self {
        self.require_default = require;
        self
    }

    /// Sets whether to warn about missing test files.
    #[must_use]
    pub const fn with_warn_missing_tests(mut self, warn: bool) -> Self {
        self.warn_missing_tests = warn;
        self
    }

    /// Analyzes a policy and returns the analysis result.
    ///
    /// # Arguments
    ///
    /// * `policy` - The policy to analyze
    ///
    /// # Errors
    ///
    /// Returns an error if validation fails and strict mode is enabled.
    pub fn analyze(&self, policy: &Policy) -> Result<AnalysisResult> {
        let mut result = AnalysisResult::default();

        // Check for default rules and collect imports/rules
        for (line_num, line) in policy.source.lines().enumerate() {
            let trimmed = line.trim();

            // Skip comments and empty lines
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            // Check for default allow
            if trimmed.starts_with("default allow") {
                result.has_default_allow = true;
                if trimmed.contains(":= true") || trimmed.contains("= true") {
                    result.warnings.push(AnalysisWarning {
                        message: "Default allow is true - this may be insecure".to_string(),
                        line: Some(line_num + 1),
                    });
                }
            }

            // Check for default deny (alternative pattern)
            if trimmed.starts_with("default allow")
                && (trimmed.contains(":= false") || trimmed.contains("= false"))
            {
                result.has_default_deny = true;
            }

            // Collect imports
            if let Some(rest) = trimmed.strip_prefix("import ") {
                let import = rest.trim_end_matches(';').trim();
                result.imports.push(import.to_string());
            }

            // Collect rule names (simplified detection)
            if let Some(idx) = trimmed.find(" if {") {
                let rule_name = trimmed[..idx].trim();
                if !rule_name.starts_with('#') && !rule_name.starts_with("default ") {
                    result.rules.push(rule_name.to_string());
                }
            } else if let Some(idx) = trimmed.find(" := ") {
                let rule_name = trimmed[..idx].trim();
                if !rule_name.starts_with('#') && !rule_name.starts_with("default ") {
                    result.rules.push(rule_name.to_string());
                }
            }
        }

        // Validation checks
        if self.require_default && !result.has_default_allow && !result.has_default_deny {
            return Err(CompilerError::ValidationError {
                message: format!(
                    "Policy '{}' has no default decision rule. Add 'default allow := false'.",
                    policy.package_name
                ),
            });
        }

        // Warn about potentially insecure patterns
        if !result.has_default_deny && result.has_default_allow {
            warn!(
                package = %policy.package_name,
                "Policy defaults to allow - consider defaulting to deny for security"
            );
        }

        // Warn about missing tests
        if self.warn_missing_tests && !policy.is_test() {
            let test_package = format!("{}_test", policy.package_name);
            result.warnings.push(AnalysisWarning {
                message: format!(
                    "No corresponding test file found. Expected package: {test_package}"
                ),
                line: None,
            });
        }

        Ok(result)
    }

    /// Validates that a policy meets basic requirements.
    ///
    /// # Errors
    ///
    /// Returns an error if the policy is invalid.
    pub fn validate(&self, policy: &Policy) -> Result<()> {
        // Run analysis which performs validation
        let _ = self.analyze(policy)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analyze_valid_policy() {
        let policy = Policy::new(
            "test.authz",
            r#"
package test.authz

default allow := false

allow if {
    input.caller.type == "admin"
}
"#,
        );

        let analyzer = Analyzer::new();
        let result = analyzer.analyze(&policy).unwrap();

        assert!(result.has_default_deny);
        assert!(!result.rules.is_empty());
    }

    #[test]
    fn test_analyze_missing_default() {
        let policy = Policy::new(
            "test.authz",
            r#"
package test.authz

allow if {
    input.caller.type == "admin"
}
"#,
        );

        let analyzer = Analyzer::new();
        let result = analyzer.analyze(&policy);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, CompilerError::ValidationError { .. }));
    }

    #[test]
    fn test_analyze_without_require_default() {
        let policy = Policy::new(
            "test.authz",
            r#"
package test.authz

allow if {
    input.caller.type == "admin"
}
"#,
        );

        let analyzer = Analyzer::new().with_require_default(false);
        let result = analyzer.analyze(&policy);

        assert!(result.is_ok());
    }

    #[test]
    fn test_analyze_default_allow_warning() {
        let policy = Policy::new(
            "test.authz",
            r#"
package test.authz

default allow := true
"#,
        );

        let analyzer = Analyzer::new();
        let result = analyzer.analyze(&policy).unwrap();

        assert!(result
            .warnings
            .iter()
            .any(|w| w.message.contains("insecure")));
    }

    #[test]
    fn test_analyze_collects_imports() {
        let policy = Policy::new(
            "test.authz",
            r#"
package test.authz

import future.keywords.if
import data.common.roles

default allow := false
"#,
        );

        let analyzer = Analyzer::new();
        let result = analyzer.analyze(&policy).unwrap();

        assert!(result.imports.contains(&"future.keywords.if".to_string()));
        assert!(result.imports.contains(&"data.common.roles".to_string()));
    }
}
