//! Policy model and metadata.
//!
//! This module defines the [`Policy`] structure that represents a Rego policy
//! file with its metadata.

use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Represents a Rego policy file with metadata.
///
/// A policy contains the raw Rego source code along with metadata like the
/// package name, file path, and timestamps.
///
/// # Examples
///
/// ```rust
/// use eunomia_core::Policy;
///
/// let policy = Policy::new(
///     "users_service.authz",
///     r#"
///         package users_service.authz
///         default allow := false
///         allow { input.caller.type == "user" }
///     "#,
/// );
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Policy {
    /// The Rego package name (e.g., "users_service.authz").
    pub package_name: String,

    /// Raw Rego source code.
    pub source: String,

    /// Original file path (if loaded from file).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_path: Option<PathBuf>,

    /// When the policy was created or loaded.
    pub created_at: DateTime<Utc>,

    /// Policy description (from METADATA comment).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Policy authors (from METADATA comment).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub authors: Vec<String>,
}

impl Policy {
    /// Creates a new policy from source code.
    ///
    /// # Arguments
    ///
    /// * `package_name` - The Rego package name
    /// * `source` - Raw Rego source code
    ///
    /// # Examples
    ///
    /// ```rust
    /// use eunomia_core::Policy;
    ///
    /// let policy = Policy::new("test.authz", "package test.authz\ndefault allow := false");
    /// assert_eq!(policy.package_name, "test.authz");
    /// ```
    #[must_use]
    pub fn new(package_name: impl Into<String>, source: impl Into<String>) -> Self {
        Self {
            package_name: package_name.into(),
            source: source.into(),
            file_path: None,
            created_at: Utc::now(),
            description: None,
            authors: Vec::new(),
        }
    }

    /// Creates a new policy with file path information.
    ///
    /// # Arguments
    ///
    /// * `package_name` - The Rego package name
    /// * `source` - Raw Rego source code
    /// * `file_path` - Path to the source file
    #[must_use]
    pub fn with_file_path(
        package_name: impl Into<String>,
        source: impl Into<String>,
        file_path: impl Into<PathBuf>,
    ) -> Self {
        Self {
            package_name: package_name.into(),
            source: source.into(),
            file_path: Some(file_path.into()),
            created_at: Utc::now(),
            description: None,
            authors: Vec::new(),
        }
    }

    /// Sets the policy description.
    #[must_use]
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Sets the policy authors.
    #[must_use]
    pub fn with_authors(mut self, authors: Vec<String>) -> Self {
        self.authors = authors;
        self
    }

    /// Adds an author to the policy.
    #[must_use]
    pub fn with_author(mut self, author: impl Into<String>) -> Self {
        self.authors.push(author.into());
        self
    }

    /// Returns the service name extracted from the package name.
    ///
    /// Assumes package names follow the convention `<service>.<module>`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use eunomia_core::Policy;
    ///
    /// let policy = Policy::new("users_service.authz", "");
    /// assert_eq!(policy.service_name(), Some("users_service"));
    ///
    /// let policy = Policy::new("simple", "");
    /// assert_eq!(policy.service_name(), None);
    /// ```
    #[must_use]
    pub fn service_name(&self) -> Option<&str> {
        self.package_name.split('.').next().filter(|s| {
            // Only return if there's more than one part
            self.package_name.contains('.')
        })
    }

    /// Returns true if this appears to be a test policy.
    ///
    /// Test policies are identified by:
    /// - Package name ending in `_test`
    /// - File path ending in `_test.rego`
    #[must_use]
    pub fn is_test(&self) -> bool {
        if self.package_name.ends_with("_test") {
            return true;
        }
        if let Some(path) = &self.file_path {
            if let Some(name) = path.file_name() {
                return name.to_string_lossy().ends_with("_test.rego");
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_policy_new() {
        let policy = Policy::new("users_service.authz", "package users_service.authz");

        assert_eq!(policy.package_name, "users_service.authz");
        assert_eq!(policy.source, "package users_service.authz");
        assert!(policy.file_path.is_none());
        assert!(policy.description.is_none());
        assert!(policy.authors.is_empty());
    }

    #[test]
    fn test_policy_with_file_path() {
        let policy = Policy::with_file_path(
            "users_service.authz",
            "package users_service.authz",
            "policies/users-service/authz.rego",
        );

        assert_eq!(
            policy.file_path,
            Some(PathBuf::from("policies/users-service/authz.rego"))
        );
    }

    #[test]
    fn test_policy_with_metadata() {
        let policy = Policy::new("test.authz", "")
            .with_description("Test policy")
            .with_author("team@example.com")
            .with_author("other@example.com");

        assert_eq!(policy.description, Some("Test policy".to_string()));
        assert_eq!(policy.authors, vec!["team@example.com", "other@example.com"]);
    }

    #[test]
    fn test_service_name_extraction() {
        let policy = Policy::new("users_service.authz", "");
        assert_eq!(policy.service_name(), Some("users_service"));

        let policy = Policy::new("orders_service.rules.admin", "");
        assert_eq!(policy.service_name(), Some("orders_service"));

        let policy = Policy::new("simple", "");
        assert_eq!(policy.service_name(), None);
    }

    #[test]
    fn test_is_test_by_package_name() {
        let policy = Policy::new("users_service.authz_test", "");
        assert!(policy.is_test());

        let policy = Policy::new("users_service.authz", "");
        assert!(!policy.is_test());
    }

    #[test]
    fn test_is_test_by_file_path() {
        let policy = Policy::with_file_path(
            "users_service.authz",
            "",
            "policies/authz_test.rego",
        );
        assert!(policy.is_test());

        let policy = Policy::with_file_path(
            "users_service.authz",
            "",
            "policies/authz.rego",
        );
        assert!(!policy.is_test());
    }

    #[test]
    fn test_policy_serialization() {
        let policy = Policy::new("test.authz", "package test.authz\ndefault allow := false")
            .with_description("A test policy");

        let json = serde_json::to_string(&policy).unwrap();
        assert!(json.contains(r#""package_name":"test.authz""#));
        assert!(json.contains(r#""description":"A test policy""#));

        let deserialized: Policy = serde_json::from_str(&json).unwrap();
        assert_eq!(policy.package_name, deserialized.package_name);
        assert_eq!(policy.source, deserialized.source);
        assert_eq!(policy.description, deserialized.description);
    }
}
