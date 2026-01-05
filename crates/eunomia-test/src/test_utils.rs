//! Test utilities for policy testing.
//!
//! This module provides convenience functions and assertion helpers
//! for writing policy tests.
//!
//! # Overview
//!
//! The utilities are organized into:
//! - [`InputBuilder`] - Fluent builder for policy input
//! - Assertion helpers for common test patterns
//! - Result matchers for test outcomes
//!
//! # Examples
//!
//! ```rust
//! use eunomia_test::{InputBuilder, MockUser};
//! use serde_json::json;
//!
//! // Build input using the fluent API
//! let input = InputBuilder::new()
//!     .caller(MockUser::admin())
//!     .operation("deleteUser")
//!     .method("DELETE")
//!     .path("/users/user-123")
//!     .service("users-service")
//!     .build();
//! ```

use eunomia_core::CallerIdentity;
use serde_json::{json, Value};
use std::collections::HashMap;

/// Fluent builder for constructing policy input JSON.
///
/// This builder creates input objects that match the Themis platform's
/// `PolicyInput` schema, with convenient defaults and validation.
///
/// # Examples
///
/// ```rust
/// use eunomia_test::{InputBuilder, MockUser};
///
/// let input = InputBuilder::new()
///     .caller(MockUser::viewer())
///     .operation("getUser")
///     .method("GET")
///     .path("/users/me")
///     .build();
///
/// // Input is ready for policy evaluation
/// assert!(input.get("caller").is_some());
/// assert_eq!(input["operation_id"], "getUser");
/// ```
#[derive(Debug, Clone, Default)]
pub struct InputBuilder {
    caller: Option<Value>,
    operation_id: Option<String>,
    method: Option<String>,
    path: Option<String>,
    service: Option<String>,
    headers: HashMap<String, String>,
    context: HashMap<String, Value>,
    environment: Option<String>,
}

impl InputBuilder {
    /// Creates a new input builder with defaults.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the caller identity from a `CallerIdentity` value.
    ///
    /// The identity is automatically serialized to the expected JSON format.
    #[must_use]
    pub fn caller(mut self, identity: CallerIdentity) -> Self {
        self.caller = serde_json::to_value(identity).ok();
        self
    }

    /// Sets the caller identity from raw JSON.
    #[must_use]
    pub fn caller_json(mut self, caller: Value) -> Self {
        self.caller = Some(caller);
        self
    }

    /// Sets the operation ID (from Themis contract).
    #[must_use]
    pub fn operation(mut self, operation_id: impl Into<String>) -> Self {
        self.operation_id = Some(operation_id.into());
        self
    }

    /// Sets the HTTP method.
    #[must_use]
    pub fn method(mut self, method: impl Into<String>) -> Self {
        self.method = Some(method.into());
        self
    }

    /// Sets the request path.
    #[must_use]
    pub fn path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }

    /// Sets the service name.
    #[must_use]
    pub fn service(mut self, service: impl Into<String>) -> Self {
        self.service = Some(service.into());
        self
    }

    /// Adds a header to the request.
    #[must_use]
    pub fn header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    /// Adds a context value (for resource attributes, extracted parameters).
    #[must_use]
    pub fn context_value(mut self, key: impl Into<String>, value: Value) -> Self {
        self.context.insert(key.into(), value);
        self
    }

    /// Adds a string context value.
    #[must_use]
    pub fn context_string(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.context.insert(key.into(), Value::String(value.into()));
        self
    }

    /// Sets the environment (e.g., "production", "staging").
    #[must_use]
    pub fn environment(mut self, env: impl Into<String>) -> Self {
        self.environment = Some(env.into());
        self
    }

    /// Builds the input as a JSON value.
    #[must_use]
    pub fn build(self) -> Value {
        let mut input = json!({});

        if let Some(caller) = self.caller {
            input["caller"] = caller;
        } else {
            // Default to anonymous
            input["caller"] = json!({"type": "anonymous"});
        }

        if let Some(op) = self.operation_id {
            input["operation_id"] = json!(op);
        }

        if let Some(method) = self.method {
            input["method"] = json!(method);
        }

        if let Some(path) = self.path {
            input["path"] = json!(path);
        }

        if let Some(service) = self.service {
            input["service"] = json!(service);
        }

        if !self.headers.is_empty() {
            input["headers"] = json!(self.headers);
        }

        if !self.context.is_empty() {
            input["context"] = json!(self.context);
        }

        if let Some(env) = self.environment {
            input["environment"] = json!(env);
        } else {
            input["environment"] = json!("test");
        }

        input
    }
}

/// Asserts that a test result indicates the request was allowed.
///
/// # Panics
///
/// Panics if the test did not pass or the decision was not "allowed".
#[track_caller]
pub fn assert_allowed(result: &crate::TestResult) {
    assert!(
        result.passed,
        "Expected request to be allowed, but test failed: {}",
        result.error.as_deref().unwrap_or("unknown error")
    );
}

/// Asserts that a test result indicates the request was denied.
///
/// # Panics
///
/// Panics if the test passed when denial was expected.
#[track_caller]
pub fn assert_denied(result: &crate::TestResult) {
    // For fixture tests that expect denial, passed means the denial was correct
    if result.expected == Some("false".to_string()) {
        assert!(
            result.passed,
            "Expected request to be denied, but it was allowed"
        );
    }
}

/// Asserts that all tests in a result set passed.
///
/// # Panics
///
/// Panics if any test failed, printing details of failures.
#[track_caller]
pub fn assert_all_passed(results: &crate::TestResults) {
    if !results.all_passed() {
        let failures: Vec<_> = results.failures().collect();
        let messages: Vec<String> = failures
            .iter()
            .map(|r| {
                format!(
                    "  - {}: {}",
                    r.name,
                    r.error.as_deref().unwrap_or("unknown")
                )
            })
            .collect();

        panic!(
            "Expected all tests to pass, but {} failed:\n{}",
            failures.len(),
            messages.join("\n")
        );
    }
}

/// Creates a simple allow policy for testing.
///
/// Returns a policy string that allows based on the caller type.
#[must_use]
pub fn simple_allow_policy(allowed_caller_type: &str) -> String {
    format!(
        r#"
package test

default allow := false

allow if {{
    input.caller.type == "{allowed_caller_type}"
}}
"#
    )
}

/// Creates a role-based policy for testing.
///
/// Returns a policy string that allows based on the caller having a specific role.
#[must_use]
pub fn role_based_policy(required_role: &str) -> String {
    format!(
        r#"
package test

import future.keywords.in

default allow := false

allow if {{
    input.caller.type == "user"
    "{required_role}" in input.caller.roles
}}
"#
    )
}

/// Creates a scope-based policy for testing API keys.
///
/// Returns a policy string that allows based on the API key having a specific scope.
#[must_use]
pub fn scope_based_policy(required_scope: &str) -> String {
    format!(
        r#"
package test

import future.keywords.in

default allow := false

allow if {{
    input.caller.type == "api_key"
    "{required_scope}" in input.caller.scopes
}}
"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock_identity::{MockApiKey, MockSpiffe, MockUser};

    #[test]
    fn test_input_builder_basic() {
        let input = InputBuilder::new()
            .caller(MockUser::admin())
            .operation("getUser")
            .method("GET")
            .path("/users/123")
            .service("users-service")
            .build();

        assert!(input.get("caller").is_some());
        assert_eq!(input["operation_id"], "getUser");
        assert_eq!(input["method"], "GET");
        assert_eq!(input["path"], "/users/123");
        assert_eq!(input["service"], "users-service");
        assert_eq!(input["environment"], "test");
    }

    #[test]
    fn test_input_builder_with_headers() {
        let input = InputBuilder::new()
            .caller(MockUser::viewer())
            .header("Authorization", "Bearer token")
            .header("X-Request-Id", "req-123")
            .build();

        let headers = input.get("headers").unwrap();
        assert_eq!(headers["Authorization"], "Bearer token");
        assert_eq!(headers["X-Request-Id"], "req-123");
    }

    #[test]
    fn test_input_builder_with_context() {
        let input = InputBuilder::new()
            .caller(MockUser::editor())
            .context_string("userId", "user-123")
            .context_value("permissions", json!(["read", "write"]))
            .build();

        let context = input.get("context").unwrap();
        assert_eq!(context["userId"], "user-123");
    }

    #[test]
    fn test_input_builder_default_anonymous() {
        let input = InputBuilder::new().operation("publicEndpoint").build();

        assert_eq!(input["caller"]["type"], "anonymous");
    }

    #[test]
    fn test_input_builder_spiffe_caller() {
        let input = InputBuilder::new()
            .caller(MockSpiffe::orders_service())
            .operation("getUser")
            .build();

        assert_eq!(input["caller"]["type"], "spiffe");
        assert_eq!(input["caller"]["service_name"], "orders-service");
    }

    #[test]
    fn test_input_builder_api_key_caller() {
        let input = InputBuilder::new()
            .caller(MockApiKey::read_only())
            .operation("listUsers")
            .build();

        assert_eq!(input["caller"]["type"], "api_key");
    }

    #[test]
    fn test_simple_allow_policy() {
        let policy = simple_allow_policy("admin");
        assert!(policy.contains("default allow := false"));
        assert!(policy.contains(r#"input.caller.type == "admin""#));
    }

    #[test]
    fn test_role_based_policy() {
        let policy = role_based_policy("super_admin");
        assert!(policy.contains(r#""super_admin" in input.caller.roles"#));
    }

    #[test]
    fn test_scope_based_policy() {
        let policy = scope_based_policy("read:users");
        assert!(policy.contains(r#""read:users" in input.caller.scopes"#));
    }
}
