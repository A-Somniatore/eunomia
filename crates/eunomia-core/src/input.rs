//! Policy input schema for authorization requests.
//!
//! This module defines the [`PolicyInput`] structure that represents the
//! input data provided to OPA for policy evaluation.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::CallerIdentity;

/// Input data for policy evaluation.
///
/// This structure contains all the information needed to make an authorization
/// decision. It is serialized to JSON and passed to OPA as the `input` document.
///
/// # Examples
///
/// ```rust
/// use eunomia_core::{PolicyInput, CallerIdentity};
///
/// let input = PolicyInput::builder()
///     .caller(CallerIdentity::user("user-123", vec!["admin".to_string()]))
///     .service("users-service")
///     .operation_id("getUser")
///     .method("GET")
///     .path("/users/user-123")
///     .build();
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyInput {
    /// The caller's identity.
    pub caller: CallerIdentity,

    /// Target service name.
    pub service: String,

    /// Target operation identifier (from Themis contract).
    pub operation_id: String,

    /// HTTP method (GET, POST, PUT, DELETE, etc.).
    pub method: String,

    /// Request path.
    pub path: String,

    /// Filtered request headers.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub headers: HashMap<String, String>,

    /// Request timestamp.
    pub timestamp: DateTime<Utc>,

    /// Environment (production, staging, development, etc.).
    pub environment: String,
}

impl PolicyInput {
    /// Creates a new builder for constructing a [`PolicyInput`].
    ///
    /// # Examples
    ///
    /// ```rust
    /// use eunomia_core::{PolicyInput, CallerIdentity};
    ///
    /// let input = PolicyInput::builder()
    ///     .caller(CallerIdentity::anonymous())
    ///     .service("api")
    ///     .operation_id("healthCheck")
    ///     .method("GET")
    ///     .path("/health")
    ///     .build();
    /// ```
    #[must_use]
    pub fn builder() -> PolicyInputBuilder {
        PolicyInputBuilder::default()
    }
}

/// Builder for constructing [`PolicyInput`] instances.
#[derive(Debug, Default)]
pub struct PolicyInputBuilder {
    caller: Option<CallerIdentity>,
    service: Option<String>,
    operation_id: Option<String>,
    method: Option<String>,
    path: Option<String>,
    headers: HashMap<String, String>,
    timestamp: Option<DateTime<Utc>>,
    environment: Option<String>,
}

impl PolicyInputBuilder {
    /// Sets the caller identity.
    #[must_use]
    pub fn caller(mut self, caller: CallerIdentity) -> Self {
        self.caller = Some(caller);
        self
    }

    /// Sets the target service name.
    #[must_use]
    pub fn service(mut self, service: impl Into<String>) -> Self {
        self.service = Some(service.into());
        self
    }

    /// Sets the target operation ID.
    #[must_use]
    pub fn operation_id(mut self, operation_id: impl Into<String>) -> Self {
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

    /// Adds a header to the request headers.
    #[must_use]
    pub fn header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    /// Sets all headers at once.
    #[must_use]
    pub fn headers(mut self, headers: HashMap<String, String>) -> Self {
        self.headers = headers;
        self
    }

    /// Sets the request timestamp.
    #[must_use]
    pub const fn timestamp(mut self, timestamp: DateTime<Utc>) -> Self {
        self.timestamp = Some(timestamp);
        self
    }

    /// Sets the environment.
    #[must_use]
    pub fn environment(mut self, environment: impl Into<String>) -> Self {
        self.environment = Some(environment.into());
        self
    }

    /// Builds the [`PolicyInput`].
    ///
    /// # Panics
    ///
    /// Panics if required fields (caller, service, `operation_id`, method, path)
    /// are not set.
    #[must_use]
    pub fn build(self) -> PolicyInput {
        PolicyInput {
            caller: self.caller.expect("caller is required"),
            service: self.service.expect("service is required"),
            operation_id: self.operation_id.expect("operation_id is required"),
            method: self.method.expect("method is required"),
            path: self.path.expect("path is required"),
            headers: self.headers,
            timestamp: self.timestamp.unwrap_or_else(Utc::now),
            environment: self.environment.unwrap_or_else(|| "production".to_string()),
        }
    }

    /// Attempts to build the [`PolicyInput`], returning an error if required
    /// fields are missing.
    ///
    /// # Errors
    ///
    /// Returns an error if any required field is not set.
    pub fn try_build(self) -> crate::Result<PolicyInput> {
        let caller = self.caller.ok_or_else(|| crate::Error::InvalidInput {
            reason: "caller is required".to_string(),
        })?;
        let service = self.service.ok_or_else(|| crate::Error::InvalidInput {
            reason: "service is required".to_string(),
        })?;
        let operation_id = self
            .operation_id
            .ok_or_else(|| crate::Error::InvalidInput {
                reason: "operation_id is required".to_string(),
            })?;
        let method = self.method.ok_or_else(|| crate::Error::InvalidInput {
            reason: "method is required".to_string(),
        })?;
        let path = self.path.ok_or_else(|| crate::Error::InvalidInput {
            reason: "path is required".to_string(),
        })?;

        Ok(PolicyInput {
            caller,
            service,
            operation_id,
            method,
            path,
            headers: self.headers,
            timestamp: self.timestamp.unwrap_or_else(Utc::now),
            environment: self.environment.unwrap_or_else(|| "production".to_string()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_policy_input_builder() {
        let input = PolicyInput::builder()
            .caller(CallerIdentity::user("user-123", vec!["admin".to_string()]))
            .service("users-service")
            .operation_id("getUser")
            .method("GET")
            .path("/users/user-123")
            .environment("staging")
            .build();

        assert_eq!(input.service, "users-service");
        assert_eq!(input.operation_id, "getUser");
        assert_eq!(input.method, "GET");
        assert_eq!(input.path, "/users/user-123");
        assert_eq!(input.environment, "staging");
        assert!(input.caller.is_user());
    }

    #[test]
    fn test_policy_input_builder_with_headers() {
        let input = PolicyInput::builder()
            .caller(CallerIdentity::anonymous())
            .service("api")
            .operation_id("listItems")
            .method("GET")
            .path("/items")
            .header("x-request-id", "req-123")
            .header("x-trace-id", "trace-456")
            .build();

        assert_eq!(input.headers.len(), 2);
        assert_eq!(
            input.headers.get("x-request-id"),
            Some(&"req-123".to_string())
        );
        assert_eq!(
            input.headers.get("x-trace-id"),
            Some(&"trace-456".to_string())
        );
    }

    #[test]
    fn test_policy_input_default_environment() {
        let input = PolicyInput::builder()
            .caller(CallerIdentity::anonymous())
            .service("api")
            .operation_id("health")
            .method("GET")
            .path("/health")
            .build();

        assert_eq!(input.environment, "production");
    }

    #[test]
    fn test_policy_input_try_build_success() {
        let result = PolicyInput::builder()
            .caller(CallerIdentity::anonymous())
            .service("api")
            .operation_id("health")
            .method("GET")
            .path("/health")
            .try_build();

        assert!(result.is_ok());
    }

    #[test]
    fn test_policy_input_try_build_missing_caller() {
        let result = PolicyInput::builder()
            .service("api")
            .operation_id("health")
            .method("GET")
            .path("/health")
            .try_build();

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("caller is required"));
    }

    #[test]
    fn test_policy_input_try_build_missing_service() {
        let result = PolicyInput::builder()
            .caller(CallerIdentity::anonymous())
            .operation_id("health")
            .method("GET")
            .path("/health")
            .try_build();

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("service is required"));
    }

    #[test]
    fn test_policy_input_serialization() {
        let input = PolicyInput::builder()
            .caller(CallerIdentity::user("user-123", vec!["viewer".to_string()]))
            .service("users-service")
            .operation_id("getUser")
            .method("GET")
            .path("/users/user-123")
            .timestamp(
                DateTime::parse_from_rfc3339("2026-01-04T12:00:00Z")
                    .unwrap()
                    .with_timezone(&Utc),
            )
            .build();

        let json = serde_json::to_string(&input).unwrap();
        assert!(json.contains(r#""service":"users-service""#));
        assert!(json.contains(r#""operation_id":"getUser""#));

        let deserialized: PolicyInput = serde_json::from_str(&json).unwrap();
        assert_eq!(input, deserialized);
    }

    #[test]
    fn test_policy_input_empty_headers_not_serialized() {
        let input = PolicyInput::builder()
            .caller(CallerIdentity::anonymous())
            .service("api")
            .operation_id("health")
            .method("GET")
            .path("/health")
            .build();

        let json = serde_json::to_string(&input).unwrap();
        assert!(!json.contains("headers"));
    }
}
