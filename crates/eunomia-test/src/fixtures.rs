//! Test fixtures for policy testing.
//!
//! Fixtures provide test input data and expected outcomes for policy evaluation.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::{Result, TestError};

/// A test fixture containing input and expected output for a policy test.
///
/// # Examples
///
/// ```rust
/// use eunomia_test::TestFixture;
/// use serde_json::json;
///
/// let fixture = TestFixture::new("admin_access")
///     .with_input(json!({
///         "caller": { "type": "user", "roles": ["admin"] },
///         "operation_id": "deleteUser"
///     }))
///     .expect_allowed(true);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestFixture {
    /// Name of the test fixture.
    pub name: String,

    /// Description of what this fixture tests.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Input data for policy evaluation.
    pub input: serde_json::Value,

    /// Expected decision (allow/deny).
    pub expected_allowed: bool,

    /// Expected reason (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_reason: Option<String>,

    /// Additional data to load into the policy.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub data: HashMap<String, serde_json::Value>,
}

impl TestFixture {
    /// Creates a new test fixture with the given name.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: None,
            input: serde_json::Value::Object(serde_json::Map::new()),
            expected_allowed: false,
            expected_reason: None,
            data: HashMap::new(),
        }
    }

    /// Sets the fixture description.
    #[must_use]
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Sets the input data.
    #[must_use]
    pub fn with_input(mut self, input: serde_json::Value) -> Self {
        self.input = input;
        self
    }

    /// Sets the expected allowed decision.
    #[must_use]
    pub const fn expect_allowed(mut self, allowed: bool) -> Self {
        self.expected_allowed = allowed;
        self
    }

    /// Sets the expected reason.
    #[must_use]
    pub fn expect_reason(mut self, reason: impl Into<String>) -> Self {
        self.expected_reason = Some(reason.into());
        self
    }

    /// Adds data to be loaded into the policy.
    #[must_use]
    pub fn with_data(mut self, path: impl Into<String>, data: serde_json::Value) -> Self {
        self.data.insert(path.into(), data);
        self
    }
}

/// A collection of test fixtures.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FixtureSet {
    /// Name of the fixture set.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Package this fixture set tests.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub package: Option<String>,

    /// The fixtures in this set.
    pub fixtures: Vec<TestFixture>,
}

impl FixtureSet {
    /// Creates a new empty fixture set.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the fixture set name.
    #[must_use]
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Sets the target package.
    #[must_use]
    pub fn with_package(mut self, package: impl Into<String>) -> Self {
        self.package = Some(package.into());
        self
    }

    /// Adds a fixture to the set.
    #[must_use]
    pub fn add_fixture(mut self, fixture: TestFixture) -> Self {
        self.fixtures.push(fixture);
        self
    }

    /// Returns the number of fixtures in this set.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.fixtures.len()
    }

    /// Returns true if there are no fixtures.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.fixtures.is_empty()
    }

    /// Loads fixtures from a JSON file.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or parsed.
    pub fn from_json_file(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let content = fs::read_to_string(path).map_err(|e| TestError::FileReadError {
            path: path.to_path_buf(),
            source: e,
        })?;

        let fixtures: Self = serde_json::from_str(&content)?;
        Ok(fixtures)
    }

    /// Loads fixtures from a YAML file.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or parsed.
    pub fn from_yaml_file(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let content = fs::read_to_string(path).map_err(|e| TestError::FileReadError {
            path: path.to_path_buf(),
            source: e,
        })?;

        let fixtures: Self = serde_yaml::from_str(&content)?;
        Ok(fixtures)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_fixture_creation() {
        let fixture = TestFixture::new("test_admin")
            .with_description("Test admin access")
            .with_input(json!({ "caller": { "type": "user", "roles": ["admin"] } }))
            .expect_allowed(true);

        assert_eq!(fixture.name, "test_admin");
        assert_eq!(fixture.description, Some("Test admin access".to_string()));
        assert!(fixture.expected_allowed);
    }

    #[test]
    fn test_fixture_with_data() {
        let fixture = TestFixture::new("test_roles")
            .with_data("roles", json!({"admin": ["read", "write"]}));

        assert!(fixture.data.contains_key("roles"));
    }

    #[test]
    fn test_fixture_set() {
        let set = FixtureSet::new()
            .with_name("User Service Tests")
            .with_package("users_service.authz")
            .add_fixture(TestFixture::new("test1").expect_allowed(true))
            .add_fixture(TestFixture::new("test2").expect_allowed(false));

        assert_eq!(set.len(), 2);
        assert!(!set.is_empty());
        assert_eq!(set.name, Some("User Service Tests".to_string()));
        assert_eq!(set.package, Some("users_service.authz".to_string()));
    }

    #[test]
    fn test_fixture_serialization() {
        let fixture = TestFixture::new("test")
            .with_input(json!({ "key": "value" }))
            .expect_allowed(true);

        let json = serde_json::to_string(&fixture).unwrap();
        assert!(json.contains(r#""name":"test""#));
        assert!(json.contains(r#""expected_allowed":true"#));

        let deserialized: TestFixture = serde_json::from_str(&json).unwrap();
        assert_eq!(fixture.name, deserialized.name);
        assert_eq!(fixture.expected_allowed, deserialized.expected_allowed);
    }
}
