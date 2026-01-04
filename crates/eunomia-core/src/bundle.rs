//! Policy bundle model.
//!
//! This module defines the [`Bundle`] structure that represents a compiled
//! and distributable policy bundle.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A compiled policy bundle ready for distribution.
///
/// Bundles contain compiled policies along with metadata and optional
/// signatures for integrity verification.
///
/// # Examples
///
/// ```rust
/// use eunomia_core::Bundle;
///
/// let bundle = Bundle::builder("users-service")
///     .version("1.2.0")
///     .add_policy("users_service.authz", "package users_service.authz")
///     .build();
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bundle {
    /// Bundle name (typically the service name).
    pub name: String,

    /// Semantic version of the bundle.
    pub version: String,

    /// Git commit SHA this bundle was built from.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_commit: Option<String>,

    /// When the bundle was created.
    pub created_at: DateTime<Utc>,

    /// Policies included in this bundle (package name -> source).
    pub policies: HashMap<String, String>,

    /// Data files included in this bundle (path -> content).
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub data_files: HashMap<String, String>,

    /// Bundle manifest with additional metadata.
    pub manifest: BundleManifest,

    /// Ed25519 signature of the bundle contents.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,

    /// Public key ID used for signing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signing_key_id: Option<String>,
}

/// Metadata about a bundle.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct BundleManifest {
    /// Revision number (increments with each build).
    pub revision: u64,

    /// Root documents exposed by the bundle.
    #[serde(default)]
    pub roots: Vec<String>,

    /// OPA version this bundle is compatible with.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opa_version: Option<String>,

    /// Custom metadata.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, String>,
}

impl Bundle {
    /// Creates a new builder for constructing a [`Bundle`].
    ///
    /// # Arguments
    ///
    /// * `name` - Bundle name (typically the service name)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use eunomia_core::Bundle;
    ///
    /// let bundle = Bundle::builder("users-service")
    ///     .version("1.0.0")
    ///     .build();
    /// ```
    #[must_use]
    pub fn builder(name: impl Into<String>) -> BundleBuilder {
        BundleBuilder::new(name)
    }

    /// Returns true if this bundle is signed.
    #[must_use]
    pub const fn is_signed(&self) -> bool {
        self.signature.is_some()
    }

    /// Returns the number of policies in this bundle.
    #[must_use]
    pub fn policy_count(&self) -> usize {
        self.policies.len()
    }

    /// Returns true if this bundle contains a policy with the given package name.
    #[must_use]
    pub fn has_policy(&self, package_name: &str) -> bool {
        self.policies.contains_key(package_name)
    }

    /// Returns the file name for this bundle.
    ///
    /// Format: `<name>-v<version>.bundle.tar.gz`
    #[must_use]
    pub fn file_name(&self) -> String {
        format!("{}-v{}.bundle.tar.gz", self.name, self.version)
    }
}

/// Builder for constructing [`Bundle`] instances.
#[derive(Debug)]
pub struct BundleBuilder {
    name: String,
    version: Option<String>,
    git_commit: Option<String>,
    policies: HashMap<String, String>,
    data_files: HashMap<String, String>,
    manifest: BundleManifest,
}

impl BundleBuilder {
    /// Creates a new bundle builder.
    fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: None,
            git_commit: None,
            policies: HashMap::new(),
            data_files: HashMap::new(),
            manifest: BundleManifest::default(),
        }
    }

    /// Sets the bundle version.
    #[must_use]
    pub fn version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self
    }

    /// Sets the git commit SHA.
    #[must_use]
    pub fn git_commit(mut self, commit: impl Into<String>) -> Self {
        self.git_commit = Some(commit.into());
        self
    }

    /// Adds a policy to the bundle.
    #[must_use]
    pub fn add_policy(mut self, package_name: impl Into<String>, source: impl Into<String>) -> Self {
        self.policies.insert(package_name.into(), source.into());
        self
    }

    /// Adds a data file to the bundle.
    #[must_use]
    pub fn add_data_file(mut self, path: impl Into<String>, content: impl Into<String>) -> Self {
        self.data_files.insert(path.into(), content.into());
        self
    }

    /// Sets the manifest revision.
    #[must_use]
    pub const fn revision(mut self, revision: u64) -> Self {
        self.manifest.revision = revision;
        self
    }

    /// Adds a root document to the manifest.
    #[must_use]
    pub fn add_root(mut self, root: impl Into<String>) -> Self {
        self.manifest.roots.push(root.into());
        self
    }

    /// Sets the OPA version compatibility.
    #[must_use]
    pub fn opa_version(mut self, version: impl Into<String>) -> Self {
        self.manifest.opa_version = Some(version.into());
        self
    }

    /// Adds custom metadata to the manifest.
    #[must_use]
    pub fn metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.manifest.metadata.insert(key.into(), value.into());
        self
    }

    /// Builds the bundle.
    ///
    /// # Panics
    ///
    /// Panics if version is not set.
    #[must_use]
    pub fn build(self) -> Bundle {
        Bundle {
            name: self.name,
            version: self.version.expect("version is required"),
            git_commit: self.git_commit,
            created_at: Utc::now(),
            policies: self.policies,
            data_files: self.data_files,
            manifest: self.manifest,
            signature: None,
            signing_key_id: None,
        }
    }

    /// Attempts to build the bundle, returning an error if required fields are missing.
    ///
    /// # Errors
    ///
    /// Returns an error if version is not set.
    pub fn try_build(self) -> crate::Result<Bundle> {
        let version = self.version.ok_or_else(|| crate::Error::InvalidInput {
            reason: "version is required".to_string(),
        })?;

        Ok(Bundle {
            name: self.name,
            version,
            git_commit: self.git_commit,
            created_at: Utc::now(),
            policies: self.policies,
            data_files: self.data_files,
            manifest: self.manifest,
            signature: None,
            signing_key_id: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bundle_builder_basic() {
        let bundle = Bundle::builder("users-service")
            .version("1.0.0")
            .build();

        assert_eq!(bundle.name, "users-service");
        assert_eq!(bundle.version, "1.0.0");
        assert!(bundle.policies.is_empty());
        assert!(!bundle.is_signed());
    }

    #[test]
    fn test_bundle_builder_with_policies() {
        let bundle = Bundle::builder("users-service")
            .version("1.2.0")
            .add_policy("users_service.authz", "package users_service.authz")
            .add_policy("users_service.roles", "package users_service.roles")
            .build();

        assert_eq!(bundle.policy_count(), 2);
        assert!(bundle.has_policy("users_service.authz"));
        assert!(bundle.has_policy("users_service.roles"));
        assert!(!bundle.has_policy("nonexistent"));
    }

    #[test]
    fn test_bundle_builder_with_data_files() {
        let bundle = Bundle::builder("users-service")
            .version("1.0.0")
            .add_data_file("data/roles.json", r#"{"admin": ["read", "write"]}"#)
            .build();

        assert!(bundle.data_files.contains_key("data/roles.json"));
    }

    #[test]
    fn test_bundle_builder_with_manifest() {
        let bundle = Bundle::builder("users-service")
            .version("1.0.0")
            .revision(42)
            .add_root("users_service")
            .opa_version("0.60.0")
            .metadata("team", "platform")
            .build();

        assert_eq!(bundle.manifest.revision, 42);
        assert_eq!(bundle.manifest.roots, vec!["users_service"]);
        assert_eq!(bundle.manifest.opa_version, Some("0.60.0".to_string()));
        assert_eq!(
            bundle.manifest.metadata.get("team"),
            Some(&"platform".to_string())
        );
    }

    #[test]
    fn test_bundle_file_name() {
        let bundle = Bundle::builder("users-service")
            .version("1.2.3")
            .build();

        assert_eq!(bundle.file_name(), "users-service-v1.2.3.bundle.tar.gz");
    }

    #[test]
    fn test_bundle_try_build_missing_version() {
        let result = Bundle::builder("users-service").try_build();

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("version is required"));
    }

    #[test]
    fn test_bundle_serialization() {
        let bundle = Bundle::builder("users-service")
            .version("1.0.0")
            .git_commit("abc123")
            .add_policy("users_service.authz", "package users_service.authz")
            .build();

        let json = serde_json::to_string(&bundle).unwrap();
        assert!(json.contains(r#""name":"users-service""#));
        assert!(json.contains(r#""version":"1.0.0""#));
        assert!(json.contains(r#""git_commit":"abc123""#));

        let deserialized: Bundle = serde_json::from_str(&json).unwrap();
        assert_eq!(bundle.name, deserialized.name);
        assert_eq!(bundle.version, deserialized.version);
        assert_eq!(bundle.git_commit, deserialized.git_commit);
    }
}
