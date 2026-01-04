//! OPA/Rego policy engine integration.
//!
//! This module provides a Rust interface to the OPA policy engine using `regorus`,
//! a pure Rust implementation of OPA/Rego.
//!
//! # Architecture
//!
//! The engine wraps the `regorus` crate to provide:
//! - Policy loading and parsing with validation
//! - Data injection for policy evaluation
//! - Query evaluation and result handling
//! - Rule enumeration for test discovery
//!
//! # Examples
//!
//! ```rust,ignore
//! use eunomia_compiler::engine::RegoEngine;
//!
//! let mut engine = RegoEngine::new();
//! engine.add_policy_from_file("policies/authz.rego")?;
//! engine.set_input_json(&serde_json::json!({
//!     "caller": { "type": "user", "roles": ["admin"] },
//!     "action": "read"
//! }))?;
//!
//! let allowed = engine.eval_bool("data.authz.allow")?;
//! ```

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use serde_json::Value;
use tracing::{debug, instrument, warn};

use crate::error::{CompilerError, Result};

/// A Rego policy engine based on `regorus`.
///
/// The engine maintains a collection of policies and data, and can evaluate
/// Rego queries against them.
#[derive(Debug)]
pub struct RegoEngine {
    /// The underlying regorus engine.
    inner: regorus::Engine,
    /// Loaded policy files and their packages.
    policies: HashMap<String, PolicyInfo>,
    /// Whether strict mode is enabled for additional validation.
    strict_mode: bool,
}

/// Information about a loaded policy.
#[derive(Debug, Clone)]
pub struct PolicyInfo {
    /// The policy's package name.
    pub package: String,
    /// Path to the source file (if loaded from file).
    pub file_path: Option<String>,
    /// List of rule names in the policy.
    pub rules: Vec<String>,
    /// Whether this is a test file.
    pub is_test: bool,
}

/// Result of evaluating a Rego query.
#[derive(Debug, Clone)]
pub enum EvalResult {
    /// Boolean result.
    Bool(bool),
    /// String result.
    String(String),
    /// Numeric result.
    Number(f64),
    /// Array result.
    Array(Vec<Value>),
    /// Object result.
    Object(Value),
    /// Undefined (no result).
    Undefined,
}

impl EvalResult {
    /// Returns `true` if this is a truthy boolean result.
    #[must_use]
    pub const fn is_truthy(&self) -> bool {
        match self {
            Self::Bool(b) => *b,
            Self::Undefined => false,
            _ => true,
        }
    }

    /// Converts to a boolean if possible.
    #[must_use]
    pub const fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(b) => Some(*b),
            Self::Undefined => Some(false),
            _ => None,
        }
    }

    /// Converts to a JSON value.
    #[must_use]
    pub fn to_json(&self) -> Value {
        match self {
            Self::Bool(b) => Value::Bool(*b),
            Self::String(s) => Value::String(s.clone()),
            Self::Number(n) => {
                serde_json::Number::from_f64(*n)
                    .map_or(Value::Null, Value::Number)
            }
            Self::Array(arr) => Value::Array(arr.clone()),
            Self::Object(obj) => obj.clone(),
            Self::Undefined => Value::Null,
        }
    }
}

impl Default for RegoEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl RegoEngine {
    /// Creates a new Rego engine.
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: regorus::Engine::new(),
            policies: HashMap::new(),
            strict_mode: false,
        }
    }

    /// Creates a new engine with strict mode enabled.
    ///
    /// Strict mode enables additional validation checks.
    #[must_use]
    pub fn with_strict_mode() -> Self {
        let mut engine = Self::new();
        engine.strict_mode = true;
        engine
    }

    /// Enables or disables strict mode.
    pub const fn set_strict_mode(&mut self, strict: bool) {
        self.strict_mode = strict;
    }

    /// Adds a policy from a file.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the Rego policy file
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or parsed.
    #[instrument(skip(self), fields(path = %path.as_ref().display()))]
    pub fn add_policy_from_file(&mut self, path: impl AsRef<Path>) -> Result<PolicyInfo> {
        let path = path.as_ref();
        let source = fs::read_to_string(path).map_err(|e| CompilerError::FileReadError {
            path: path.to_path_buf(),
            source: e,
        })?;

        let file_name = path.to_string_lossy().to_string();
        self.add_policy(&file_name, &source)
    }

    /// Adds a policy from source code.
    ///
    /// # Arguments
    ///
    /// * `name` - Name/path of the policy (for error messages)
    /// * `source` - Rego source code
    ///
    /// # Errors
    ///
    /// Returns an error if the source cannot be parsed.
    #[instrument(skip(self, source))]
    pub fn add_policy(&mut self, name: &str, source: &str) -> Result<PolicyInfo> {
        debug!(name, "Adding policy");

        // Add policy to regorus engine
        self.inner
            .add_policy(name.to_string(), source.to_string())
            .map_err(|e| CompilerError::ParseError {
                file: name.to_string(),
                line: extract_line_from_error(&e.to_string()).unwrap_or(1),
                message: e.to_string(),
            })?;

        // Extract policy info
        let package = extract_package_from_source(source, name)?;
        let rules = extract_rules_from_source(source);
        let is_test = name.ends_with("_test.rego") || package.ends_with("_test");

        let info = PolicyInfo {
            package,
            file_path: Some(name.to_string()),
            rules,
            is_test,
        };

        self.policies.insert(name.to_string(), info.clone());

        Ok(info)
    }

    /// Sets the input data for evaluation.
    ///
    /// # Arguments
    ///
    /// * `input` - JSON value to use as input
    ///
    /// # Errors
    ///
    /// Returns an error if the input cannot be set.
    pub fn set_input(&mut self, input: Value) -> Result<()> {
        // Convert serde_json::Value to regorus::Value
        let regorus_value: regorus::Value = input.into();
        self.inner.set_input(regorus_value);
        Ok(())
    }

    /// Sets the input data from a JSON value.
    ///
    /// This is a convenience method that calls [`set_input`](Self::set_input).
    ///
    /// # Errors
    ///
    /// Returns an error if the input cannot be set.
    pub fn set_input_json(&mut self, input: &Value) -> Result<()> {
        self.set_input(input.clone())
    }

    /// Adds data to the engine.
    ///
    /// # Arguments
    ///
    /// * `path` - Document path (e.g., "data.roles")
    /// * `data` - JSON data to add
    ///
    /// # Errors
    ///
    /// Returns an error if the data cannot be added.
    pub fn add_data(&mut self, data: Value) -> Result<()> {
        let regorus_value: regorus::Value = data.into();
        self.inner
            .add_data(regorus_value)
            .map_err(|e| CompilerError::ValidationError {
                message: format!("Failed to add data: {e}"),
            })
    }

    /// Clears the input data.
    pub fn clear_input(&mut self) {
        self.inner.clear_data();
    }

    /// Evaluates a Rego query and returns the result.
    ///
    /// # Arguments
    ///
    /// * `query` - Rego query string (e.g., "data.authz.allow")
    ///
    /// # Errors
    ///
    /// Returns an error if the query cannot be evaluated.
    #[instrument(skip(self))]
    pub fn eval(&mut self, query: &str) -> Result<EvalResult> {
        debug!(query, "Evaluating query");

        let results = self
            .inner
            .eval_query(query.to_string(), false)
            .map_err(|e| CompilerError::ValidationError {
                message: format!("Query evaluation failed: {e}"),
            })?;

        // Convert regorus result to our EvalResult
        if results.result.is_empty() {
            return Ok(EvalResult::Undefined);
        }

        // Get the first result's expressions
        let first_result = &results.result[0];
        if first_result.expressions.is_empty() {
            return Ok(EvalResult::Undefined);
        }

        let value = &first_result.expressions[0].value;
        Ok(convert_value(value))
    }

    /// Evaluates a query and returns a boolean result.
    ///
    /// Returns `false` if the result is undefined.
    ///
    /// # Errors
    ///
    /// Returns an error if the query cannot be evaluated.
    pub fn eval_bool(&mut self, query: &str) -> Result<bool> {
        let result = self.eval(query)?;
        Ok(result.is_truthy())
    }

    /// Gets the list of loaded policy files.
    #[must_use]
    pub fn policy_files(&self) -> Vec<&str> {
        self.policies.keys().map(String::as_str).collect()
    }

    /// Gets information about a loaded policy.
    #[must_use]
    pub fn get_policy_info(&self, name: &str) -> Option<&PolicyInfo> {
        self.policies.get(name)
    }

    /// Gets all loaded policy infos.
    pub fn all_policies(&self) -> impl Iterator<Item = (&String, &PolicyInfo)> {
        self.policies.iter()
    }

    /// Gets test rules across all loaded policies.
    ///
    /// Test rules are those starting with `test_` prefix.
    #[must_use]
    pub fn get_test_rules(&self) -> Vec<TestRule> {
        let mut tests = Vec::new();

        for (file, info) in &self.policies {
            for rule in &info.rules {
                if rule.starts_with("test_") {
                    tests.push(TestRule {
                        file: file.clone(),
                        package: info.package.clone(),
                        name: rule.clone(),
                        qualified_name: format!("data.{}.{}", info.package, rule),
                    });
                }
            }
        }

        tests
    }

    /// Returns whether strict mode is enabled.
    #[must_use]
    pub const fn is_strict_mode(&self) -> bool {
        self.strict_mode
    }
}

/// A test rule in a policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestRule {
    /// Source file containing the test.
    pub file: String,
    /// Package containing the test.
    pub package: String,
    /// Rule name (e.g., `test_admin_allowed`).
    pub name: String,
    /// Fully qualified name (e.g., `data.authz_test.test_admin_allowed`).
    pub qualified_name: String,
}

/// Extracts the package name from Rego source.
fn extract_package_from_source(source: &str, file: &str) -> Result<String> {
    for (line_num, line) in source.lines().enumerate() {
        let trimmed = line.trim();

        // Skip comments and empty lines
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Look for package declaration
        if let Some(rest) = trimmed.strip_prefix("package") {
            let package_name = rest.trim().trim_end_matches(';');
            if package_name.is_empty() {
                return Err(CompilerError::ParseError {
                    file: file.to_string(),
                    line: line_num + 1,
                    message: "Empty package name".to_string(),
                });
            }
            return Ok(package_name.to_string());
        }

        // If we hit a non-comment, non-package line first, error
        return Err(CompilerError::MissingPackage {
            file: file.to_string(),
        });
    }

    Err(CompilerError::MissingPackage {
        file: file.to_string(),
    })
}

/// Extracts rule names from Rego source.
fn extract_rules_from_source(source: &str) -> Vec<String> {
    let mut rules = Vec::new();

    for line in source.lines() {
        let trimmed = line.trim();

        // Skip comments and empty lines
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Look for rule definitions
        // Formats: `name := ...`, `name = ...`, `name { ... }`, `name if { ... }`
        if let Some(name) = extract_rule_name(trimmed) {
            if !rules.contains(&name) {
                rules.push(name);
            }
        }
    }

    rules
}

/// Extracts a rule name from a line.
fn extract_rule_name(line: &str) -> Option<String> {
    // Skip imports and package
    if line.starts_with("import") || line.starts_with("package") {
        return None;
    }

    // Skip `default` keyword but extract the name after it
    let line = line.strip_prefix("default").map_or(line, str::trim);

    // Find rule name (before := , = , if, {, [, or ()
    let name_end = line
        .find(":=")
        .or_else(|| line.find(" = "))
        .or_else(|| line.find(" if"))
        .or_else(|| line.find('{'))
        .or_else(|| line.find('['))
        .or_else(|| line.find('('))?;

    let name = line[..name_end].trim();

    // Validate it looks like an identifier
    if name.is_empty() || !name.chars().next()?.is_alphabetic() {
        return None;
    }

    // Handle array/object rules like `arr[x]` -> `arr`
    let name = name.split('[').next()?.trim();

    if is_valid_identifier(name) {
        Some(name.to_string())
    } else {
        None
    }
}

/// Checks if a string is a valid Rego identifier.
fn is_valid_identifier(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    let mut chars = s.chars();
    let first = chars.next().unwrap();
    if !first.is_alphabetic() && first != '_' {
        return false;
    }
    chars.all(|c| c.is_alphanumeric() || c == '_')
}

/// Extracts line number from error message.
fn extract_line_from_error(msg: &str) -> Option<usize> {
    // Try to find patterns like "line 5" or ":5:" or "at line 5"
    let msg_lower = msg.to_lowercase();

    // Pattern: "line N"
    if let Some(idx) = msg_lower.find("line ") {
        let rest = &msg[idx + 5..];
        if let Some(num_str) = rest.split_whitespace().next() {
            if let Ok(num) = num_str.trim_matches(|c: char| !c.is_ascii_digit()).parse() {
                return Some(num);
            }
        }
    }

    // Pattern: ":N:"
    for part in msg.split(':') {
        if let Ok(num) = part.trim().parse() {
            if num > 0 {
                return Some(num);
            }
        }
    }

    None
}

/// Converts a regorus Value to our `EvalResult`.
#[allow(clippy::cast_precision_loss)] // Expected: f64 can't represent all i64/u64 values precisely
fn convert_value(value: &regorus::Value) -> EvalResult {
    match value {
        regorus::Value::Bool(b) => EvalResult::Bool(*b),
        regorus::Value::String(s) => EvalResult::String(s.to_string()),
        regorus::Value::Number(n) => {
            // Try to get the best representation
            n.as_f64()
                .map(EvalResult::Number)
                .or_else(|| n.as_i64().map(|i| EvalResult::Number(i as f64)))
                .or_else(|| n.as_u64().map(|u| EvalResult::Number(u as f64)))
                .unwrap_or(EvalResult::Number(0.0))
        }
        regorus::Value::Array(arr) => {
            let values: Vec<Value> = arr.iter().map(regorus_to_json).collect();
            EvalResult::Array(values)
        }
        regorus::Value::Object(obj) => {
            let map: serde_json::Map<String, Value> = obj
                .iter()
                .map(|(k, v)| (k.to_string(), regorus_to_json(v)))
                .collect();
            EvalResult::Object(Value::Object(map))
        }
        regorus::Value::Set(set) => {
            let values: Vec<Value> = set.iter().map(regorus_to_json).collect();
            EvalResult::Array(values)
        }
        regorus::Value::Null | regorus::Value::Undefined => EvalResult::Undefined,
    }
}

/// Converts a regorus Value to a `serde_json` Value.
fn regorus_to_json(value: &regorus::Value) -> Value {
    match value {
        regorus::Value::Null | regorus::Value::Undefined => Value::Null,
        regorus::Value::Bool(b) => Value::Bool(*b),
        regorus::Value::String(s) => Value::String(s.to_string()),
        regorus::Value::Number(n) => {
            // Try f64 first, then fallback to integer representations
            n.as_f64()
                .and_then(serde_json::Number::from_f64)
                .map(Value::Number)
                .or_else(|| n.as_i64().map(|i| Value::Number(i.into())))
                .or_else(|| n.as_u64().map(|u| Value::Number(u.into())))
                .unwrap_or_else(|| Value::Number(0.into()))
        }
        regorus::Value::Array(arr) => {
            Value::Array(arr.iter().map(regorus_to_json).collect())
        }
        regorus::Value::Object(obj) => {
            let map: serde_json::Map<String, Value> = obj
                .iter()
                .map(|(k, v)| (k.to_string(), regorus_to_json(v)))
                .collect();
            Value::Object(map)
        }
        regorus::Value::Set(set) => {
            Value::Array(set.iter().map(regorus_to_json).collect())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const SIMPLE_POLICY: &str = r#"
package authz

default allow := false

allow if {
    input.user.role == "admin"
}
"#;

    const TEST_POLICY: &str = r#"
package authz_test

import data.authz

test_admin_allowed if {
    authz.allow with input as {"user": {"role": "admin"}}
}

test_guest_denied if {
    not authz.allow with input as {"user": {"role": "guest"}}
}
"#;

    #[test]
    fn test_engine_creation() {
        let engine = RegoEngine::new();
        assert!(!engine.is_strict_mode());

        let strict = RegoEngine::with_strict_mode();
        assert!(strict.is_strict_mode());
    }

    #[test]
    fn test_add_policy() {
        let mut engine = RegoEngine::new();
        let info = engine.add_policy("authz.rego", SIMPLE_POLICY).unwrap();

        assert_eq!(info.package, "authz");
        assert!(!info.is_test);
        assert!(info.rules.contains(&"allow".to_string()));
    }

    #[test]
    fn test_add_test_policy() {
        let mut engine = RegoEngine::new();
        engine.add_policy("authz.rego", SIMPLE_POLICY).unwrap();

        let info = engine.add_policy("authz_test.rego", TEST_POLICY).unwrap();

        assert_eq!(info.package, "authz_test");
        assert!(info.is_test);
        assert!(info.rules.contains(&"test_admin_allowed".to_string()));
        assert!(info.rules.contains(&"test_guest_denied".to_string()));
    }

    #[test]
    fn test_eval_allow_admin() {
        let mut engine = RegoEngine::new();
        engine.add_policy("authz.rego", SIMPLE_POLICY).unwrap();

        engine
            .set_input(json!({"user": {"role": "admin"}}))
            .unwrap();

        let result = engine.eval_bool("data.authz.allow").unwrap();
        assert!(result);
    }

    #[test]
    fn test_eval_deny_guest() {
        let mut engine = RegoEngine::new();
        engine.add_policy("authz.rego", SIMPLE_POLICY).unwrap();

        engine
            .set_input(json!({"user": {"role": "guest"}}))
            .unwrap();

        let result = engine.eval_bool("data.authz.allow").unwrap();
        assert!(!result);
    }

    #[test]
    fn test_eval_default_deny() {
        let mut engine = RegoEngine::new();
        engine.add_policy("authz.rego", SIMPLE_POLICY).unwrap();

        // No input set - should get default deny
        let result = engine.eval_bool("data.authz.allow").unwrap();
        assert!(!result);
    }

    #[test]
    fn test_get_test_rules() {
        let mut engine = RegoEngine::new();
        engine.add_policy("authz.rego", SIMPLE_POLICY).unwrap();
        engine.add_policy("authz_test.rego", TEST_POLICY).unwrap();

        let tests = engine.get_test_rules();

        assert_eq!(tests.len(), 2);
        assert!(tests.iter().any(|t| t.name == "test_admin_allowed"));
        assert!(tests.iter().any(|t| t.name == "test_guest_denied"));
    }

    #[test]
    fn test_parse_error() {
        let mut engine = RegoEngine::new();
        let result = engine.add_policy("bad.rego", "not valid rego syntax");

        assert!(result.is_err());
    }

    #[test]
    fn test_missing_package() {
        let mut engine = RegoEngine::new();
        let result = engine.add_policy("no_pkg.rego", "default allow := false");

        assert!(result.is_err());
    }

    #[test]
    fn test_extract_rule_name() {
        assert_eq!(extract_rule_name("allow := true"), Some("allow".to_string()));
        assert_eq!(extract_rule_name("allow = true"), Some("allow".to_string()));
        assert_eq!(extract_rule_name("allow if { true }"), Some("allow".to_string()));
        assert_eq!(extract_rule_name("allow { true }"), Some("allow".to_string()));
        assert_eq!(
            extract_rule_name("default allow := false"),
            Some("allow".to_string())
        );
        assert_eq!(
            extract_rule_name("arr[x] { x := 1 }"),
            Some("arr".to_string())
        );
        assert_eq!(extract_rule_name("import future.keywords"), None);
        assert_eq!(extract_rule_name("package foo"), None);
        assert_eq!(extract_rule_name("# comment"), None);
    }

    #[test]
    fn test_eval_result_is_truthy() {
        assert!(EvalResult::Bool(true).is_truthy());
        assert!(!EvalResult::Bool(false).is_truthy());
        assert!(!EvalResult::Undefined.is_truthy());
        assert!(EvalResult::String("hello".to_string()).is_truthy());
        assert!(EvalResult::Number(42.0).is_truthy());
    }

    #[test]
    fn test_policy_info_tracking() {
        let mut engine = RegoEngine::new();
        engine.add_policy("authz.rego", SIMPLE_POLICY).unwrap();

        let files: Vec<_> = engine.policy_files();
        assert!(files.contains(&"authz.rego"));

        let info = engine.get_policy_info("authz.rego").unwrap();
        assert_eq!(info.package, "authz");
    }
}
