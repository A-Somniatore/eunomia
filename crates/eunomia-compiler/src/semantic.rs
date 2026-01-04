//! Semantic validation for Rego policies.
//!
//! This module provides deeper semantic analysis beyond syntax checking,
//! including:
//! - Rule reference validation
//! - Input schema validation against expected structure
//! - Operation ID validation against service contracts
//! - Data flow analysis for potential issues
//!
//! # Example
//!
//! ```rust,ignore
//! use eunomia_compiler::semantic::{SemanticValidator, ServiceContract};
//!
//! let mut validator = SemanticValidator::new();
//!
//! // Register known operation IDs
//! validator.register_operation("getUser");
//! validator.register_operation("updateUser");
//!
//! // Validate policy
//! let issues = validator.validate_source(source)?;
//! ```

use std::collections::{HashMap, HashSet};
use std::path::Path;

use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::error::{CompilerError, Result};

/// A semantic issue found during validation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticIssue {
    /// Severity of the issue.
    pub severity: SemanticSeverity,
    /// Category of the issue.
    pub category: SemanticCategory,
    /// Human-readable message.
    pub message: String,
    /// Line number (1-based, if applicable).
    pub line: Option<usize>,
    /// The problematic code snippet.
    pub snippet: Option<String>,
    /// Suggestion for fixing the issue.
    pub suggestion: Option<String>,
}

/// Severity of semantic issues.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SemanticSeverity {
    /// Informational hint.
    Hint,
    /// Warning - may indicate a problem.
    Warning,
    /// Error - semantic validation failure.
    Error,
}

/// Category of semantic issues.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SemanticCategory {
    /// Unknown or invalid operation ID.
    UnknownOperation,
    /// Undefined rule reference.
    UndefinedRule,
    /// Unused variable or rule.
    Unused,
    /// Input schema violation.
    InputSchema,
    /// Data reference issue.
    DataReference,
    /// Type mismatch or incompatibility.
    TypeMismatch,
}

/// A mock service contract for testing without Themis.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MockServiceContract {
    /// Service name.
    pub service_name: String,
    /// Known operation IDs for this service.
    pub operation_ids: HashSet<String>,
    /// HTTP methods allowed per operation.
    pub operation_methods: HashMap<String, Vec<String>>,
}

impl MockServiceContract {
    /// Creates a new mock contract for a service.
    #[must_use]
    pub fn new(service_name: impl Into<String>) -> Self {
        Self {
            service_name: service_name.into(),
            operation_ids: HashSet::new(),
            operation_methods: HashMap::new(),
        }
    }

    /// Adds an operation ID to the contract.
    pub fn add_operation(&mut self, operation_id: impl Into<String>) -> &mut Self {
        self.operation_ids.insert(operation_id.into());
        self
    }

    /// Adds an operation with allowed HTTP methods.
    pub fn add_operation_with_methods(
        &mut self,
        operation_id: impl Into<String>,
        methods: Vec<&str>,
    ) -> &mut Self {
        let op_id = operation_id.into();
        self.operation_ids.insert(op_id.clone());
        self.operation_methods
            .insert(op_id, methods.into_iter().map(String::from).collect());
        self
    }

    /// Checks if an operation ID is valid.
    #[must_use]
    pub fn has_operation(&self, operation_id: &str) -> bool {
        self.operation_ids.contains(operation_id)
    }

    /// Returns the allowed methods for an operation.
    #[must_use]
    pub fn get_methods(&self, operation_id: &str) -> Option<&Vec<String>> {
        self.operation_methods.get(operation_id)
    }
}

/// Expected structure for authorization input.
#[derive(Debug, Clone)]
pub struct InputSchema {
    /// Required top-level fields.
    pub required_fields: HashSet<String>,
    /// Optional top-level fields.
    pub optional_fields: HashSet<String>,
    /// Nested field requirements (e.g., "caller.type").
    pub nested_requirements: HashMap<String, FieldRequirement>,
}

/// Requirement for a field in the input schema.
#[derive(Debug, Clone)]
pub struct FieldRequirement {
    /// Whether the field is required.
    pub required: bool,
    /// Expected type (for documentation).
    pub expected_type: String,
    /// Allowed values (if enumerated).
    pub allowed_values: Option<Vec<String>>,
}

impl Default for InputSchema {
    fn default() -> Self {
        Self::themis_standard()
    }
}

impl InputSchema {
    /// Creates the standard Themis input schema.
    #[must_use]
    pub fn themis_standard() -> Self {
        let mut schema = Self {
            required_fields: HashSet::new(),
            optional_fields: HashSet::new(),
            nested_requirements: HashMap::new(),
        };

        // Required top-level fields
        schema.required_fields.insert("caller".to_string());
        schema.required_fields.insert("operation_id".to_string());
        schema.required_fields.insert("method".to_string());

        // Optional top-level fields
        schema.optional_fields.insert("path".to_string());
        schema.optional_fields.insert("headers".to_string());
        schema.optional_fields.insert("resource".to_string());
        schema.optional_fields.insert("context".to_string());
        schema.optional_fields.insert("time".to_string());
        schema.optional_fields.insert("environment".to_string());

        // Nested requirements
        schema.nested_requirements.insert(
            "caller.type".to_string(),
            FieldRequirement {
                required: true,
                expected_type: "string".to_string(),
                allowed_values: Some(vec![
                    "user".to_string(),
                    "spiffe".to_string(),
                    "api_key".to_string(),
                    "anonymous".to_string(),
                ]),
            },
        );

        schema.nested_requirements.insert(
            "method".to_string(),
            FieldRequirement {
                required: true,
                expected_type: "string".to_string(),
                allowed_values: Some(vec![
                    "GET".to_string(),
                    "POST".to_string(),
                    "PUT".to_string(),
                    "PATCH".to_string(),
                    "DELETE".to_string(),
                    "HEAD".to_string(),
                    "OPTIONS".to_string(),
                ]),
            },
        );

        schema
    }

    /// Creates a minimal schema with only essential fields.
    #[must_use]
    pub fn minimal() -> Self {
        let mut schema = Self {
            required_fields: HashSet::new(),
            optional_fields: HashSet::new(),
            nested_requirements: HashMap::new(),
        };

        schema.required_fields.insert("caller".to_string());
        schema
    }
}

/// Semantic validator for Rego policies.
#[derive(Debug)]
pub struct SemanticValidator {
    /// Known service contracts for operation validation.
    contracts: HashMap<String, MockServiceContract>,
    /// Global operation IDs (not service-specific).
    global_operations: HashSet<String>,
    /// Input schema to validate against.
    input_schema: InputSchema,
    /// Whether to validate operation IDs.
    validate_operations: bool,
    /// Whether to validate input schema usage.
    validate_input_schema: bool,
    /// Whether to check for unused rules.
    check_unused: bool,
}

impl Default for SemanticValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl SemanticValidator {
    /// Creates a new semantic validator.
    #[must_use]
    pub fn new() -> Self {
        Self {
            contracts: HashMap::new(),
            global_operations: HashSet::new(),
            input_schema: InputSchema::default(),
            validate_operations: false, // Disabled by default without contracts
            validate_input_schema: true,
            check_unused: true,
        }
    }

    /// Registers a service contract for operation validation.
    pub fn register_contract(&mut self, contract: MockServiceContract) -> &mut Self {
        self.validate_operations = true;
        self.contracts.insert(contract.service_name.clone(), contract);
        self
    }

    /// Registers a global operation ID.
    pub fn register_operation(&mut self, operation_id: impl Into<String>) -> &mut Self {
        self.validate_operations = true;
        self.global_operations.insert(operation_id.into());
        self
    }

    /// Sets the input schema to validate against.
    pub fn with_input_schema(&mut self, schema: InputSchema) -> &mut Self {
        self.input_schema = schema;
        self
    }

    /// Enables or disables operation ID validation.
    pub const fn with_operation_validation(&mut self, enabled: bool) -> &mut Self {
        self.validate_operations = enabled;
        self
    }

    /// Enables or disables input schema validation.
    pub const fn with_input_schema_validation(&mut self, enabled: bool) -> &mut Self {
        self.validate_input_schema = enabled;
        self
    }

    /// Enables or disables unused rule checking.
    pub const fn with_unused_checking(&mut self, enabled: bool) -> &mut Self {
        self.check_unused = enabled;
        self
    }

    /// Validates a policy file.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read.
    pub fn validate_file(&self, path: impl AsRef<Path>) -> Result<Vec<SemanticIssue>> {
        let source = std::fs::read_to_string(path.as_ref()).map_err(|e| CompilerError::FileReadError {
            path: path.as_ref().to_path_buf(),
            source: e,
        })?;
        let file_name = path.as_ref().to_string_lossy();
        Ok(self.validate_source(&source, &file_name))
    }

    /// Validates policy source code.
    #[must_use]
    pub fn validate_source(&self, source: &str, file_name: &str) -> Vec<SemanticIssue> {
        let mut issues = Vec::new();

        debug!(file = %file_name, "Running semantic validation");

        // Collect information about the policy
        let policy_info = self.analyze_policy(source);

        // Check operation IDs
        if self.validate_operations {
            self.check_operation_ids(source, &policy_info, &mut issues);
        }

        // Check input schema usage
        if self.validate_input_schema {
            self.check_input_schema_usage(source, &policy_info, &mut issues);
        }

        // Check for undefined rule references
        self.check_rule_references(source, &policy_info, &mut issues);

        // Check for unused rules
        if self.check_unused {
            self.check_unused_rules(&policy_info, &mut issues);
        }

        issues
    }

    /// Analyzes a policy to extract structural information.
    fn analyze_policy(&self, source: &str) -> PolicyAnalysis {
        let mut analysis = PolicyAnalysis::default();

        for (line_num, line) in source.lines().enumerate() {
            let trimmed = line.trim();

            // Skip comments and empty lines
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            // Extract package name
            if let Some(rest) = trimmed.strip_prefix("package ") {
                analysis.package = Some(rest.trim().to_string());
                continue;
            }

            // Extract imports
            if let Some(rest) = trimmed.strip_prefix("import ") {
                let import = rest.trim_end_matches(';').trim();
                analysis.imports.push(import.to_string());
                continue;
            }

            // Extract rule definitions
            if let Some(rule_name) = Self::extract_rule_name(trimmed) {
                analysis.defined_rules.insert(rule_name.clone());
                analysis.rule_lines.insert(rule_name, line_num + 1);
            }

            // Extract rule references (simplified)
            self.extract_rule_references(trimmed, &mut analysis.referenced_rules);

            // Extract input field accesses
            self.extract_input_accesses(trimmed, &mut analysis.input_accesses);

            // Extract operation ID literals
            self.extract_operation_ids(trimmed, &mut analysis.operation_ids);
        }

        analysis
    }

    fn extract_rule_name(line: &str) -> Option<String> {
        // Match patterns like "allow if {" or "rule_name := value"
        if let Some(idx) = line.find(" if {") {
            let name = line[..idx].trim();
            if !name.starts_with("default ") && !name.contains(' ') {
                return Some(name.to_string());
            }
        } else if let Some(idx) = line.find(" if ") {
            let name = line[..idx].trim();
            if !name.starts_with("default ") && !name.contains(' ') {
                return Some(name.to_string());
            }
        } else if let Some(idx) = line.find(" := ") {
            let name = line[..idx].trim();
            if !name.starts_with("default ") && !name.contains(' ') {
                return Some(name.to_string());
            }
        } else if let Some(idx) = line.find(" = ") {
            let name = line[..idx].trim();
            if !name.starts_with("default ") && !name.contains(' ') {
                return Some(name.to_string());
            }
        }
        None
    }

    #[allow(clippy::unused_self)]
    fn extract_rule_references(&self, line: &str, refs: &mut HashSet<String>) {
        // Simple pattern matching for rule references
        let trimmed = line.trim();
        
        // Skip lines that are rule definitions, imports, or package declarations
        if trimmed.starts_with("package ")
            || trimmed.starts_with("import ")
            || trimmed.starts_with("default ")
            || trimmed.starts_with('#')
            || trimmed.contains(" := ")
            || trimmed.contains(" = ")
            || trimmed.ends_with(" if {")
        {
            return;
        }

        // Look for standalone identifiers that could be rule references
        // These are identifiers that appear on their own line within a rule body
        if !trimmed.contains("input.") && !trimmed.contains("data.") {
            // Check if this looks like a rule reference (identifier only, possibly with 'not')
            let potential_ref = trimmed.trim_start_matches("not ").trim();
            if !potential_ref.is_empty()
                && potential_ref.chars().all(|c| c.is_alphanumeric() || c == '_')
                && potential_ref != "true"
                && potential_ref != "false"
            {
                refs.insert(potential_ref.to_string());
            }
        }

        // Look for identifiers after specific patterns
        let patterns = [" if ", " with ", "not "];

        for pattern in patterns {
            if let Some(idx) = line.find(pattern) {
                let after = &line[idx + pattern.len()..];
                if let Some(end) = after.find(|c: char| !c.is_alphanumeric() && c != '_' && c != '.') {
                    let reference = after[..end].trim();
                    if !reference.is_empty() && !reference.starts_with("input.") {
                        refs.insert(reference.to_string());
                    }
                } else if !after.trim().is_empty() {
                    // Reference goes to end of segment
                    let reference = after.trim();
                    if !reference.starts_with("input.") {
                        refs.insert(reference.to_string());
                    }
                }
            }
        }

        // Look for data.* references
        let mut remaining = line;
        while let Some(idx) = remaining.find("data.") {
            let start = idx + 5; // Skip "data."
            if start < remaining.len() {
                let after = &remaining[start..];
                let end = after
                    .find(|c: char| !c.is_alphanumeric() && c != '_' && c != '.')
                    .unwrap_or(after.len());
                let reference = &after[..end];
                if !reference.is_empty() {
                    refs.insert(format!("data.{reference}"));
                }
            }
            remaining = &remaining[idx + 5..];
        }
    }

    #[allow(clippy::unused_self)]
    fn extract_input_accesses(&self, line: &str, accesses: &mut HashSet<String>) {
        // Find all input.* accesses
        let mut remaining = line;
        while let Some(idx) = remaining.find("input.") {
            let start = idx + 6; // Skip "input."
            if start < remaining.len() {
                let after = &remaining[start..];
                let end = after
                    .find(|c: char| !c.is_alphanumeric() && c != '_' && c != '.')
                    .unwrap_or(after.len());
                let access = after[..end].trim();
                if !access.is_empty() {
                    accesses.insert(format!("input.{access}"));
                }
            }
            remaining = &remaining[idx + 6..];
        }
    }

    #[allow(clippy::unused_self)]
    fn extract_operation_ids(&self, line: &str, ops: &mut HashSet<String>) {
        // Look for operation_id comparisons
        if line.contains("operation_id") {
            // Extract string literals
            let mut in_string = false;
            let mut current = String::new();

            for c in line.chars() {
                if c == '"' {
                    if in_string {
                        if !current.is_empty() {
                            ops.insert(current.clone());
                        }
                        current.clear();
                    }
                    in_string = !in_string;
                } else if in_string {
                    current.push(c);
                }
            }
        }
    }

    fn check_operation_ids(
        &self,
        _source: &str,
        analysis: &PolicyAnalysis,
        issues: &mut Vec<SemanticIssue>,
    ) {
        for op_id in &analysis.operation_ids {
            let is_known = self.global_operations.contains(op_id)
                || self
                    .contracts
                    .values()
                    .any(|c| c.has_operation(op_id));

            if !is_known {
                issues.push(SemanticIssue {
                    severity: SemanticSeverity::Warning,
                    category: SemanticCategory::UnknownOperation,
                    message: format!("Unknown operation ID: '{op_id}'"),
                    line: None,
                    snippet: Some(format!("operation_id == \"{op_id}\"")),
                    suggestion: Some(
                        "Register this operation in a service contract or use register_operation()"
                            .to_string(),
                    ),
                });
            }
        }
    }

    fn check_input_schema_usage(
        &self,
        _source: &str,
        analysis: &PolicyAnalysis,
        issues: &mut Vec<SemanticIssue>,
    ) {
        // Check for deprecated or incorrect input field usage
        let deprecated_patterns = [
            ("input.action", "Use input.operation_id instead"),
            ("input.resource.type", "Use input.resource.owner_id or context instead"),
        ];

        for access in &analysis.input_accesses {
            for (pattern, suggestion) in &deprecated_patterns {
                if access.starts_with(pattern) {
                    issues.push(SemanticIssue {
                        severity: SemanticSeverity::Warning,
                        category: SemanticCategory::InputSchema,
                        message: format!("Deprecated input field: '{access}'"),
                        line: None,
                        snippet: Some(access.clone()),
                        suggestion: Some((*suggestion).to_string()),
                    });
                }
            }
        }

        // Check for typos in common field names
        let known_fields = [
            "input.caller",
            "input.caller.type",
            "input.caller.user_id",
            "input.caller.roles",
            "input.caller.service_name",
            "input.caller.trust_domain",
            "input.caller.scopes",
            "input.operation_id",
            "input.method",
            "input.path",
            "input.headers",
            "input.resource",
            "input.context",
            "input.time",
        ];

        for access in &analysis.input_accesses {
            // Simple typo detection for top-level fields
            let parts: Vec<&str> = access.split('.').collect();
            if parts.len() >= 2 {
                let top_level = format!("input.{}", parts[1]);
                let is_known = known_fields.iter().any(|f| f.starts_with(&top_level));
                
                if !is_known && !access.starts_with("input.context.") && !access.starts_with("input.resource.") {
                    // Could be a typo
                    let suggestions = self.find_similar_fields(&top_level, &known_fields);
                    if !suggestions.is_empty() {
                        issues.push(SemanticIssue {
                            severity: SemanticSeverity::Hint,
                            category: SemanticCategory::InputSchema,
                            message: format!("Unknown input field: '{access}'"),
                            line: None,
                            snippet: Some(access.clone()),
                            suggestion: Some(format!("Did you mean: {}?", suggestions.join(", "))),
                        });
                    }
                }
            }
        }
    }

    #[allow(clippy::unused_self)]
    fn find_similar_fields(&self, target: &str, known: &[&str]) -> Vec<String> {
        known
            .iter()
            .filter(|f| {
                // Simple similarity check - same prefix or edit distance
                f.starts_with(&target[..target.len().min(8)])
                    || target.starts_with(&f[..f.len().min(8)])
            })
            .map(|s| (*s).to_string())
            .collect()
    }

    #[allow(clippy::unused_self)]
    fn check_rule_references(
        &self,
        _source: &str,
        analysis: &PolicyAnalysis,
        issues: &mut Vec<SemanticIssue>,
    ) {
        // Check for references to undefined rules
        for reference in &analysis.referenced_rules {
            // Skip data.* references (external data)
            if reference.starts_with("data.") {
                continue;
            }

            // Skip built-in functions and common patterns
            let builtins = [
                "count", "sum", "max", "min", "sort", "contains", "startswith",
                "endswith", "trim", "lower", "upper", "split", "concat", "sprintf",
                "json", "yaml", "base64", "urlquery", "regex", "time", "http",
                "io", "opa", "rego", "future", "true", "false", "null",
            ];

            let base_name = reference.split('.').next().unwrap_or(reference);
            if builtins.contains(&base_name) {
                continue;
            }

            // Check if it's a defined rule
            if !analysis.defined_rules.contains(reference)
                && !analysis.defined_rules.contains(base_name)
            {
                issues.push(SemanticIssue {
                    severity: SemanticSeverity::Hint,
                    category: SemanticCategory::UndefinedRule,
                    message: format!("Reference to undefined rule: '{reference}'"),
                    line: None,
                    snippet: None,
                    suggestion: Some(format!(
                        "Define '{reference}' or import it from another package"
                    )),
                });
            }
        }
    }

    #[allow(clippy::unused_self)]
    fn check_unused_rules(&self, analysis: &PolicyAnalysis, issues: &mut Vec<SemanticIssue>) {
        // Entry point rules that should not be flagged as unused
        let entry_points = ["allow", "deny", "violation", "warn", "final_allow"];

        for rule in &analysis.defined_rules {
            // Skip entry points
            if entry_points.contains(&rule.as_str()) {
                continue;
            }

            // Skip test rules
            if rule.starts_with("test_") {
                continue;
            }

            // Check if rule is referenced
            let is_referenced = analysis.referenced_rules.iter().any(|r| {
                r == rule || r.starts_with(&format!("{rule}.")) || r.ends_with(&format!(".{rule}"))
            });

            if !is_referenced {
                let line = analysis.rule_lines.get(rule).copied();
                issues.push(SemanticIssue {
                    severity: SemanticSeverity::Hint,
                    category: SemanticCategory::Unused,
                    message: format!("Rule '{rule}' appears to be unused"),
                    line,
                    snippet: None,
                    suggestion: Some(format!(
                        "Consider removing '{rule}' or using it in another rule"
                    )),
                });
            }
        }
    }
}

/// Internal analysis of a policy's structure.
#[derive(Debug, Default)]
struct PolicyAnalysis {
    /// Package name.
    package: Option<String>,
    /// Import statements.
    imports: Vec<String>,
    /// Defined rule names.
    defined_rules: HashSet<String>,
    /// Line numbers for defined rules.
    rule_lines: HashMap<String, usize>,
    /// Referenced rule names.
    referenced_rules: HashSet<String>,
    /// Input field accesses.
    input_accesses: HashSet<String>,
    /// Operation IDs found in the policy.
    operation_ids: HashSet<String>,
}

/// Creates a mock contract for the users service.
#[must_use]
pub fn users_service_contract() -> MockServiceContract {
    let mut contract = MockServiceContract::new("users-service");
    contract
        .add_operation_with_methods("getUser", vec!["GET"])
        .add_operation_with_methods("updateUser", vec!["PUT", "PATCH"])
        .add_operation_with_methods("deleteUser", vec!["DELETE"])
        .add_operation_with_methods("listUsers", vec!["GET"])
        .add_operation_with_methods("createUser", vec!["POST"])
        .add_operation_with_methods("getUserEmail", vec!["GET"])
        .add_operation_with_methods("getUserBillingInfo", vec!["GET"])
        .add_operation_with_methods("getUserNotificationPreferences", vec!["GET"]);
    contract
}

/// Creates a mock contract for the orders service.
#[must_use]
pub fn orders_service_contract() -> MockServiceContract {
    let mut contract = MockServiceContract::new("orders-service");
    contract
        .add_operation_with_methods("getOrder", vec!["GET"])
        .add_operation_with_methods("createOrder", vec!["POST"])
        .add_operation_with_methods("updateOrder", vec!["PUT", "PATCH"])
        .add_operation_with_methods("deleteOrder", vec!["DELETE"])
        .add_operation_with_methods("listOrders", vec!["GET"])
        .add_operation_with_methods("cancelOrder", vec!["POST"])
        .add_operation_with_methods("getOrderItems", vec!["GET"])
        .add_operation_with_methods("addOrderNote", vec!["POST"])
        .add_operation_with_methods("escalateOrder", vec!["POST"])
        .add_operation_with_methods("updateOrderPaymentStatus", vec!["PUT", "PATCH"])
        .add_operation_with_methods("updateOrderShippingStatus", vec!["PUT", "PATCH"])
        .add_operation_with_methods("getOrderCustomerInfo", vec!["GET"]);
    contract
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_contract_creation() {
        let mut contract = MockServiceContract::new("test-service");
        contract.add_operation("getItem");
        contract.add_operation_with_methods("updateItem", vec!["PUT", "PATCH"]);

        assert!(contract.has_operation("getItem"));
        assert!(contract.has_operation("updateItem"));
        assert!(!contract.has_operation("deleteItem"));

        let methods = contract.get_methods("updateItem").unwrap();
        assert!(methods.contains(&"PUT".to_string()));
        assert!(methods.contains(&"PATCH".to_string()));
    }

    #[test]
    fn test_semantic_validator_operation_check() {
        let mut validator = SemanticValidator::new();
        validator.register_operation("getUser");
        validator.register_operation("updateUser");

        let source = r#"
package test.authz

default allow := false

allow if {
    input.operation_id == "getUser"
    input.caller.type == "user"
}

allow if {
    input.operation_id == "unknownOperation"
}
"#;

        let issues = validator.validate_source(source, "test.rego");
        
        // Should find unknown operation
        let unknown_ops: Vec<_> = issues
            .iter()
            .filter(|i| i.category == SemanticCategory::UnknownOperation)
            .collect();
        
        assert_eq!(unknown_ops.len(), 1);
        assert!(unknown_ops[0].message.contains("unknownOperation"));
    }

    #[test]
    fn test_semantic_validator_deprecated_input() {
        let validator = SemanticValidator::new();

        let source = r#"
package test.authz

default allow := false

allow if {
    input.action == "read"  # deprecated
    input.caller.type == "user"
}
"#;

        let issues = validator.validate_source(source, "test.rego");
        
        // Should find deprecated field usage
        let deprecated: Vec<_> = issues
            .iter()
            .filter(|i| i.category == SemanticCategory::InputSchema)
            .collect();
        
        assert!(!deprecated.is_empty());
        assert!(deprecated.iter().any(|i| i.message.contains("action")));
    }

    #[test]
    fn test_semantic_validator_with_contract() {
        let mut validator = SemanticValidator::new();
        validator.register_contract(users_service_contract());

        let source = r#"
package users_service.authz

default allow := false

allow if {
    input.operation_id == "getUser"
}

allow if {
    input.operation_id == "invalidOperation"
}
"#;

        let issues = validator.validate_source(source, "authz.rego");
        
        // getUser is valid, invalidOperation is not
        let unknown_ops: Vec<_> = issues
            .iter()
            .filter(|i| i.category == SemanticCategory::UnknownOperation)
            .collect();
        
        assert_eq!(unknown_ops.len(), 1);
        assert!(unknown_ops[0].message.contains("invalidOperation"));
    }

    #[test]
    fn test_unused_rule_detection() {
        let validator = SemanticValidator::new();

        let source = r#"
package test.authz

default allow := false

allow if {
    is_admin
}

is_admin if {
    input.caller.roles[_] == "admin"
}

unused_helper if {
    true
}
"#;

        let issues = validator.validate_source(source, "test.rego");
        
        // Should find unused_helper as unused
        let unused: Vec<_> = issues
            .iter()
            .filter(|i| i.category == SemanticCategory::Unused)
            .collect();
        
        assert!(unused.iter().any(|i| i.message.contains("unused_helper")));
        // is_admin should NOT be flagged as unused
        assert!(!unused.iter().any(|i| i.message.contains("is_admin")));
    }

    #[test]
    fn test_input_schema_validation() {
        let schema = InputSchema::themis_standard();
        
        assert!(schema.required_fields.contains("caller"));
        assert!(schema.required_fields.contains("operation_id"));
        assert!(schema.optional_fields.contains("context"));
        
        let caller_type = schema.nested_requirements.get("caller.type").unwrap();
        assert!(caller_type.required);
        assert!(caller_type.allowed_values.as_ref().unwrap().contains(&"user".to_string()));
    }

    #[test]
    fn test_predefined_contracts() {
        let users = users_service_contract();
        assert!(users.has_operation("getUser"));
        assert!(users.has_operation("updateUser"));

        let orders = orders_service_contract();
        assert!(orders.has_operation("getOrder"));
        assert!(orders.has_operation("cancelOrder"));
    }
}
