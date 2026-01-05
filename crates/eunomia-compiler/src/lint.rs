//! Rego policy linting rules.
//!
//! This module provides linting rules for static analysis of Rego policies.
//! It detects common security issues, best practice violations, and potential bugs.

use serde::{Deserialize, Serialize};

/// Severity level for lint violations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Severity {
    /// Informational hint.
    Hint,
    /// Warning - may indicate a problem.
    Warning,
    /// Error - definitely a problem.
    Error,
}

impl Severity {
    /// Returns the string representation for display.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Hint => "hint",
            Self::Warning => "warning",
            Self::Error => "error",
        }
    }
}

/// A lint rule that can be applied to policy source code.
#[derive(Debug, Clone)]
pub struct LintRule {
    /// Unique identifier for the rule.
    pub id: &'static str,
    /// Human-readable name.
    pub name: &'static str,
    /// Description of what the rule checks.
    pub description: &'static str,
    /// Severity of violations.
    pub severity: Severity,
    /// Category of the rule.
    pub category: RuleCategory,
    /// Whether the rule is enabled by default.
    pub enabled_by_default: bool,
}

/// Category of lint rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RuleCategory {
    /// Security-related rules.
    Security,
    /// Best practice rules.
    BestPractice,
    /// Performance-related rules.
    Performance,
    /// Code style rules.
    Style,
    /// Potential bugs.
    Bugs,
}

impl RuleCategory {
    /// Returns the string representation.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Security => "security",
            Self::BestPractice => "best-practice",
            Self::Performance => "performance",
            Self::Style => "style",
            Self::Bugs => "bugs",
        }
    }
}

/// A violation found by a lint rule.
#[derive(Debug, Clone)]
pub struct LintViolation {
    /// The rule that was violated.
    pub rule_id: &'static str,
    /// Severity of the violation.
    pub severity: Severity,
    /// Human-readable message.
    pub message: String,
    /// Line number (1-based, if applicable).
    pub line: Option<usize>,
    /// Column number (1-based, if applicable).
    pub column: Option<usize>,
    /// Suggestion for fixing the violation.
    pub suggestion: Option<String>,
}

impl LintViolation {
    /// Creates a new lint violation.
    #[must_use]
    pub fn new(rule_id: &'static str, severity: Severity, message: impl Into<String>) -> Self {
        Self {
            rule_id,
            severity,
            message: message.into(),
            line: None,
            column: None,
            suggestion: None,
        }
    }

    /// Sets the line number.
    #[must_use]
    pub const fn at_line(mut self, line: usize) -> Self {
        self.line = Some(line);
        self
    }

    /// Sets the suggestion.
    #[must_use]
    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }
}

// Built-in lint rules
/// Rule: Policies should default to deny.
pub const RULE_DEFAULT_DENY: LintRule = LintRule {
    id: "security/default-deny",
    name: "Default Deny",
    description: "Policies should explicitly default to deny for security",
    severity: Severity::Error,
    category: RuleCategory::Security,
    enabled_by_default: true,
};

/// Rule: Avoid hardcoded secrets.
pub const RULE_NO_HARDCODED_SECRETS: LintRule = LintRule {
    id: "security/no-hardcoded-secrets",
    name: "No Hardcoded Secrets",
    description: "Policy should not contain hardcoded secrets or credentials",
    severity: Severity::Error,
    category: RuleCategory::Security,
    enabled_by_default: true,
};

/// Rule: Use explicit imports.
pub const RULE_EXPLICIT_IMPORTS: LintRule = LintRule {
    id: "style/explicit-imports",
    name: "Explicit Imports",
    description: "Use explicit imports for future.keywords",
    severity: Severity::Warning,
    category: RuleCategory::Style,
    enabled_by_default: true,
};

/// Rule: Avoid overly permissive rules.
pub const RULE_NO_WILDCARD_ALLOW: LintRule = LintRule {
    id: "security/no-wildcard-allow",
    name: "No Wildcard Allow",
    description: "Avoid allow rules that match everything without conditions",
    severity: Severity::Warning,
    category: RuleCategory::Security,
    enabled_by_default: true,
};

/// Rule: Policy packages should follow naming conventions.
pub const RULE_PACKAGE_NAMING: LintRule = LintRule {
    id: "style/package-naming",
    name: "Package Naming",
    description: "Package names should follow the service.module convention",
    severity: Severity::Hint,
    category: RuleCategory::Style,
    enabled_by_default: false,
};

/// Rule: Test files should test all rules.
pub const RULE_TEST_COVERAGE: LintRule = LintRule {
    id: "best-practice/test-coverage",
    name: "Test Coverage",
    description: "All allow/deny rules should have corresponding tests",
    severity: Severity::Warning,
    category: RuleCategory::BestPractice,
    enabled_by_default: true,
};

/// The default set of lint rules.
pub static DEFAULT_RULES: &[&LintRule] = &[
    &RULE_DEFAULT_DENY,
    &RULE_NO_HARDCODED_SECRETS,
    &RULE_EXPLICIT_IMPORTS,
    &RULE_NO_WILDCARD_ALLOW,
    &RULE_PACKAGE_NAMING,
    &RULE_TEST_COVERAGE,
];

/// A linter for Rego policies.
#[derive(Debug, Default)]
pub struct Linter {
    /// Enabled rule IDs.
    enabled_rules: Vec<&'static str>,
    /// Disabled rule IDs.
    disabled_rules: Vec<&'static str>,
}

impl Linter {
    /// Creates a new linter with default rules.
    #[must_use]
    pub fn new() -> Self {
        let enabled_rules = DEFAULT_RULES
            .iter()
            .filter(|r| r.enabled_by_default)
            .map(|r| r.id)
            .collect();

        Self {
            enabled_rules,
            disabled_rules: Vec::new(),
        }
    }

    /// Enables a rule by ID.
    pub fn enable_rule(&mut self, rule_id: &'static str) {
        if !self.enabled_rules.contains(&rule_id) {
            self.enabled_rules.push(rule_id);
        }
        self.disabled_rules.retain(|&id| id != rule_id);
    }

    /// Disables a rule by ID.
    pub fn disable_rule(&mut self, rule_id: &'static str) {
        if !self.disabled_rules.contains(&rule_id) {
            self.disabled_rules.push(rule_id);
        }
        self.enabled_rules.retain(|&id| id != rule_id);
    }

    /// Checks if a rule is enabled.
    #[must_use]
    pub fn is_rule_enabled(&self, rule_id: &str) -> bool {
        self.enabled_rules.contains(&rule_id) && !self.disabled_rules.contains(&rule_id)
    }

    /// Lints a policy source and returns violations.
    #[must_use]
    pub fn lint(&self, source: &str, file_name: &str) -> Vec<LintViolation> {
        let mut violations = Vec::new();

        if self.is_rule_enabled("security/default-deny") {
            violations.extend(Self::check_default_deny(source));
        }

        if self.is_rule_enabled("security/no-hardcoded-secrets") {
            violations.extend(Self::check_no_hardcoded_secrets(source, file_name));
        }

        if self.is_rule_enabled("style/explicit-imports") {
            violations.extend(Self::check_explicit_imports(source));
        }

        if self.is_rule_enabled("security/no-wildcard-allow") {
            violations.extend(Self::check_no_wildcard_allow(source));
        }

        violations
    }

    fn check_default_deny(source: &str) -> Vec<LintViolation> {
        let has_default = source.lines().any(|line| {
            let trimmed = line.trim();
            trimmed.starts_with("default allow")
        });

        let has_default_deny = source.lines().any(|line| {
            let trimmed = line.trim();
            (trimmed.starts_with("default allow") || trimmed.starts_with("default deny"))
                && (trimmed.contains(":= false") || trimmed.contains("= false"))
        });

        let has_default_allow_true = source.lines().enumerate().find(|(_, line)| {
            let trimmed = line.trim();
            trimmed.starts_with("default allow")
                && (trimmed.contains(":= true") || trimmed.contains("= true"))
        });

        let mut violations = Vec::new();

        if let Some((line_num, _)) = has_default_allow_true {
            violations.push(
                LintViolation::new(
                    "security/default-deny",
                    Severity::Error,
                    "Default allow is set to true - this is insecure",
                )
                .at_line(line_num + 1)
                .with_suggestion("Use 'default allow := false' for secure default deny"),
            );
        } else if !has_default && !has_default_deny {
            violations.push(
                LintViolation::new(
                    "security/default-deny",
                    Severity::Error,
                    "No default decision rule found",
                )
                .with_suggestion("Add 'default allow := false' at the start of the policy"),
            );
        }

        violations
    }

    fn check_no_hardcoded_secrets(source: &str, file_name: &str) -> Vec<LintViolation> {
        // Patterns that might indicate hardcoded secrets
        const SECRET_PATTERNS: &[&str] = &[
            "password",
            "secret",
            "api_key",
            "apikey",
            "access_token",
            "private_key",
            "credential",
        ];

        let mut violations = Vec::new();

        for (line_num, line) in source.lines().enumerate() {
            let trimmed = line.trim();

            // Skip comments
            if trimmed.starts_with('#') {
                continue;
            }

            let lower = trimmed.to_lowercase();

            // Look for string assignments with secret-like patterns
            for pattern in SECRET_PATTERNS {
                if lower.contains(pattern) && (lower.contains('"') || lower.contains(":=")) {
                    // Check if it's an actual value assignment, not just accessing input
                    if !lower.contains("input.") && !lower.contains("data.") {
                        violations.push(
                            LintViolation::new(
                                "security/no-hardcoded-secrets",
                                Severity::Error,
                                format!(
                                    "Possible hardcoded secret detected: pattern '{pattern}' in {file_name}"
                                ),
                            )
                            .at_line(line_num + 1)
                            .with_suggestion("Use external data or environment variables for secrets"),
                        );
                        break;
                    }
                }
            }
        }

        violations
    }

    #[allow(clippy::similar_names)]
    fn check_explicit_imports(source: &str) -> Vec<LintViolation> {
        let mut violations = Vec::new();

        // Check for usage of future.keywords without explicit import
        let uses_if_keyword = source.contains(" if {") || source.contains(" if\n");
        let uses_in_keyword = source.contains(" in ") && !source.contains('"');
        let uses_every = source.contains("every ");
        let uses_contains = source.contains("contains ");

        let has_keywords_import = source.lines().any(|line| {
            let trimmed = line.trim();
            trimmed.starts_with("import future.keywords")
        });

        if (uses_if_keyword || uses_in_keyword || uses_every || uses_contains)
            && !has_keywords_import
        {
            violations.push(
                LintViolation::new(
                    "style/explicit-imports",
                    Severity::Warning,
                    "Using future keywords without explicit import",
                )
                .with_suggestion("Add 'import future.keywords.if' (or .in, .every, .contains)"),
            );
        }

        violations
    }

    fn check_no_wildcard_allow(source: &str) -> Vec<LintViolation> {
        let mut violations = Vec::new();

        for (line_num, line) in source.lines().enumerate() {
            let trimmed = line.trim();

            // Look for allow rules with no conditions
            // Pattern: `allow := true` or `allow { true }` without input checks
            if trimmed.starts_with("allow") && !trimmed.starts_with("default allow") {
                // Check if this allow has any input conditions
                let has_input_check = source
                    .lines()
                    .skip(line_num)
                    .take(10) // Look at next 10 lines for the rule body
                    .any(|l| l.contains("input.") || l.contains("data."));

                if !has_input_check
                    && (trimmed.contains(":= true") || trimmed.ends_with("{ true }"))
                {
                    violations.push(
                        LintViolation::new(
                            "security/no-wildcard-allow",
                            Severity::Warning,
                            "Allow rule without input validation - may allow all requests",
                        )
                        .at_line(line_num + 1)
                        .with_suggestion("Add conditions that check input.caller or similar"),
                    );
                }
            }
        }

        violations
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lint_default_deny_missing() {
        let source = r#"
package test.authz

allow if {
    input.caller.role == "admin"
}
"#;

        let linter = Linter::new();
        let violations = linter.lint(source, "test.rego");

        assert!(violations
            .iter()
            .any(|v| v.rule_id == "security/default-deny"));
    }

    #[test]
    fn test_lint_default_deny_true() {
        let source = r#"
package test.authz

default allow := true
"#;

        let linter = Linter::new();
        let violations = linter.lint(source, "test.rego");

        assert!(violations
            .iter()
            .any(|v| v.rule_id == "security/default-deny" && v.message.contains("insecure")));
    }

    #[test]
    fn test_lint_default_deny_false_ok() {
        let source = r#"
package test.authz

import future.keywords.if

default allow := false

allow if {
    input.caller.role == "admin"
}
"#;

        let linter = Linter::new();
        let violations = linter.lint(source, "test.rego");

        assert!(!violations
            .iter()
            .any(|v| v.rule_id == "security/default-deny"));
    }

    #[test]
    fn test_lint_hardcoded_secrets() {
        let source = r#"
package test.authz

api_key := "sk_live_12345"

allow if {
    input.caller.role == "admin"
}
"#;

        let linter = Linter::new();
        let violations = linter.lint(source, "test.rego");

        assert!(violations
            .iter()
            .any(|v| v.rule_id == "security/no-hardcoded-secrets"));
    }

    #[test]
    fn test_lint_secrets_in_input_ok() {
        let source = r#"
package test.authz

default allow := false

allow if {
    input.api_key == data.valid_keys[_]
}
"#;

        let linter = Linter::new();
        let violations = linter.lint(source, "test.rego");

        // Should not flag input.api_key or data references
        assert!(!violations
            .iter()
            .any(|v| v.rule_id == "security/no-hardcoded-secrets"));
    }

    #[test]
    fn test_lint_missing_import() {
        let source = r#"
package test.authz

default allow := false

allow if {
    input.caller.role == "admin"
}
"#;

        let linter = Linter::new();
        let violations = linter.lint(source, "test.rego");

        assert!(violations
            .iter()
            .any(|v| v.rule_id == "style/explicit-imports"));
    }

    #[test]
    fn test_lint_with_import_ok() {
        let source = r#"
package test.authz

import future.keywords.if

default allow := false

allow if {
    input.caller.role == "admin"
}
"#;

        let linter = Linter::new();
        let violations = linter.lint(source, "test.rego");

        assert!(!violations
            .iter()
            .any(|v| v.rule_id == "style/explicit-imports"));
    }

    #[test]
    fn test_lint_wildcard_allow() {
        let source = r#"
package test.authz

default allow := false

allow := true
"#;

        let linter = Linter::new();
        let violations = linter.lint(source, "test.rego");

        assert!(violations
            .iter()
            .any(|v| v.rule_id == "security/no-wildcard-allow"));
    }

    #[test]
    fn test_enable_disable_rules() {
        let mut linter = Linter::new();

        assert!(linter.is_rule_enabled("security/default-deny"));

        linter.disable_rule("security/default-deny");
        assert!(!linter.is_rule_enabled("security/default-deny"));

        linter.enable_rule("security/default-deny");
        assert!(linter.is_rule_enabled("security/default-deny"));
    }

    #[test]
    fn test_severity_ordering() {
        assert!(Severity::Error > Severity::Warning);
        assert!(Severity::Warning > Severity::Hint);
    }
}
