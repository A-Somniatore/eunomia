//! Policy bundle model.
//!
//! This module defines the [`Bundle`] structure that represents a compiled
//! and distributable policy bundle.
//!
//! # Bundle Format
//!
//! Bundles are exported as OPA-compatible tar.gz archives with the following structure:
//!
//! ```text
//! bundle.tar.gz
//! ├── .manifest           # OPA bundle manifest (JSON)
//! ├── <namespace>/        # Policy namespace directories
//! │   ├── policy.rego     # Policy files
//! │   └── data.json       # Optional data files
//! └── .signatures/        # Optional signatures
//!     └── .manifest.sig
//! ```

use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::Path;

use chrono::{DateTime, Utc};
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use hex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tar::{Archive, Builder};

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

    /// Converts a package name to a path within the bundle.
    ///
    /// Example: `"users_service.authz"` → `"users_service/authz.rego"`
    #[must_use]
    fn package_to_path(package: &str) -> String {
        let parts: Vec<&str> = package.split('.').collect();
        if parts.len() == 1 {
            format!("{}.rego", parts[0])
        } else {
            let dir = parts[..parts.len() - 1].join("/");
            let file = parts.last().unwrap_or(&"policy");
            format!("{dir}/{file}.rego")
        }
    }

    /// Computes the SHA-256 checksum of the bundle contents.
    ///
    /// The checksum is computed over sorted file paths and contents
    /// to ensure deterministic results regardless of iteration order.
    #[must_use]
    pub fn compute_checksum(&self) -> String {
        let mut hasher = Sha256::new();

        // Sort policies by package name for deterministic ordering
        let mut policy_entries: Vec<_> = self.policies.iter().collect();
        policy_entries.sort_by_key(|(k, _)| *k);

        for (package, source) in policy_entries {
            let path = Self::package_to_path(package);
            hasher.update(path.as_bytes());
            hasher.update(b"\n");
            hasher.update(source.as_bytes());
            hasher.update(b"\n");
        }

        // Sort data files by path
        let mut data_entries: Vec<_> = self.data_files.iter().collect();
        data_entries.sort_by_key(|(k, _)| *k);

        for (path, content) in data_entries {
            hasher.update(path.as_bytes());
            hasher.update(b"\n");
            hasher.update(content.as_bytes());
            hasher.update(b"\n");
        }

        let result = hasher.finalize();
        hex::encode(result)
    }

    /// Generates the OPA-compatible manifest JSON.
    ///
    /// The manifest follows OPA's bundle specification with Eunomia extensions
    /// stored under `metadata.eunomia`.
    #[must_use]
    pub fn generate_manifest(&self) -> serde_json::Value {
        let revision = self
            .created_at
            .format("%Y%m%d%H%M%S")
            .to_string()
            .parse::<u64>()
            .unwrap_or(self.manifest.revision);

        let checksum = self.compute_checksum();

        serde_json::json!({
            "revision": revision.to_string(),
            "roots": self.manifest.roots,
            "metadata": {
                "eunomia": {
                    "version": self.version,
                    "service": self.name,
                    "git_commit": self.git_commit,
                    "created_at": self.created_at.to_rfc3339()
                },
                "checksum": {
                    "algorithm": "sha256",
                    "value": checksum
                }
            }
        })
    }

    /// Writes the bundle to a tar.gz file.
    ///
    /// Creates an OPA-compatible bundle archive with the following structure:
    /// - `.manifest` - JSON manifest file
    /// - `<namespace>/<name>.rego` - Policy files organized by package
    /// - `<namespace>/data.json` - Data files (if any)
    ///
    /// # Arguments
    ///
    /// * `path` - Path to write the bundle file
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be created or written.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use eunomia_core::Bundle;
    ///
    /// let bundle = Bundle::builder("users-service")
    ///     .version("1.0.0")
    ///     .add_policy("users_service.authz", "package users_service.authz\ndefault allow := false")
    ///     .build();
    ///
    /// bundle.write_to_file("users-service-v1.0.0.bundle.tar.gz").unwrap();
    /// ```
    pub fn write_to_file(&self, path: impl AsRef<Path>) -> crate::Result<()> {
        let file = std::fs::File::create(path.as_ref()).map_err(|e| crate::Error::Io {
            message: format!("failed to create bundle file: {e}"),
        })?;

        self.write_to_writer(file)
    }

    /// Writes the bundle to any writer as a tar.gz archive.
    ///
    /// # Errors
    ///
    /// Returns an error if writing fails.
    pub fn write_to_writer<W: Write>(&self, writer: W) -> crate::Result<()> {
        let encoder = GzEncoder::new(writer, Compression::default());
        let mut archive = Builder::new(encoder);

        // Add manifest
        let manifest = self.generate_manifest();
        let manifest_bytes = serde_json::to_vec_pretty(&manifest).map_err(|e| {
            crate::Error::Serialization {
                message: format!("failed to serialize manifest: {e}"),
            }
        })?;
        Self::add_bytes_to_archive(&mut archive, ".manifest", &manifest_bytes)?;

        // Add policies
        for (package, source) in &self.policies {
            let path = Self::package_to_path(package);
            Self::add_bytes_to_archive(&mut archive, &path, source.as_bytes())?;
        }

        // Add data files
        for (path, content) in &self.data_files {
            Self::add_bytes_to_archive(&mut archive, path, content.as_bytes())?;
        }

        // Finish the archive
        let encoder = archive.into_inner().map_err(|e| crate::Error::Io {
            message: format!("failed to finish archive: {e}"),
        })?;
        encoder.finish().map_err(|e| crate::Error::Io {
            message: format!("failed to compress archive: {e}"),
        })?;

        Ok(())
    }

    /// Writes the bundle contents to a Vec<u8>.
    ///
    /// # Errors
    ///
    /// Returns an error if serialization fails.
    pub fn to_bytes(&self) -> crate::Result<Vec<u8>> {
        let mut buffer = Vec::new();
        self.write_to_writer(&mut buffer)?;
        Ok(buffer)
    }

    /// Adds bytes to a tar archive with the given path.
    fn add_bytes_to_archive<W: Write>(
        archive: &mut Builder<W>,
        path: &str,
        data: &[u8],
    ) -> crate::Result<()> {
        let mut header = tar::Header::new_gnu();
        header.set_size(data.len() as u64);
        header.set_mode(0o644);
        header.set_mtime(0); // Use fixed mtime for reproducibility
        header.set_cksum();

        archive
            .append_data(&mut header, path, data)
            .map_err(|e| crate::Error::Io {
                message: format!("failed to add {path} to archive: {e}"),
            })
    }

    /// Reads a bundle from a tar.gz file.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the bundle file
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or is not a valid bundle.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use eunomia_core::Bundle;
    ///
    /// let bundle = Bundle::from_file("users-service-v1.0.0.bundle.tar.gz").unwrap();
    /// assert_eq!(bundle.name, "users-service");
    /// ```
    pub fn from_file(path: impl AsRef<Path>) -> crate::Result<Self> {
        let file = std::fs::File::open(path.as_ref()).map_err(|e| crate::Error::Io {
            message: format!("failed to open bundle file: {e}"),
        })?;

        Self::from_reader(file)
    }

    /// Reads a bundle from any reader containing tar.gz data.
    ///
    /// # Errors
    ///
    /// Returns an error if the data is not a valid bundle.
    pub fn from_reader<R: Read>(reader: R) -> crate::Result<Self> {
        let decoder = GzDecoder::new(reader);
        let mut archive = Archive::new(decoder);

        let mut manifest_data: Option<serde_json::Value> = None;
        let mut policies = HashMap::new();
        let mut data_files = HashMap::new();

        for entry in archive.entries().map_err(|e| crate::Error::Io {
            message: format!("failed to read archive entries: {e}"),
        })? {
            let mut entry = entry.map_err(|e| crate::Error::Io {
                message: format!("failed to read archive entry: {e}"),
            })?;

            let path = entry
                .path()
                .map_err(|e| crate::Error::Io {
                    message: format!("failed to get entry path: {e}"),
                })?
                .to_string_lossy()
                .to_string();

            let mut contents = String::new();
            entry.read_to_string(&mut contents).map_err(|e| crate::Error::Io {
                message: format!("failed to read entry contents: {e}"),
            })?;

            let path_obj = Path::new(&path);

            if path == ".manifest" {
                manifest_data = Some(serde_json::from_str(&contents).map_err(|e| {
                    crate::Error::Serialization {
                        message: format!("failed to parse manifest: {e}"),
                    }
                })?);
            } else if path_obj
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("rego"))
            {
                // Convert path back to package name
                let package = Self::path_to_package(&path);
                policies.insert(package, contents);
            } else if path_obj
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("json") || ext.eq_ignore_ascii_case("yaml"))
            {
                data_files.insert(path, contents);
            }
        }

        let manifest = manifest_data.ok_or_else(|| crate::Error::InvalidInput {
            reason: "bundle missing .manifest file".to_string(),
        })?;

        Self::from_manifest(&manifest, policies, data_files)
    }

    /// Reads a bundle from raw bytes.
    ///
    /// # Errors
    ///
    /// Returns an error if the bytes are not a valid bundle.
    pub fn from_bytes(bytes: &[u8]) -> crate::Result<Self> {
        Self::from_reader(std::io::Cursor::new(bytes))
    }

    /// Converts a path back to a package name.
    ///
    /// Example: `"users_service/authz.rego"` → `"users_service.authz"`
    fn path_to_package(path: &str) -> String {
        path.trim_end_matches(".rego")
            .replace('/', ".")
    }

    /// Creates a Bundle from manifest data and file contents.
    fn from_manifest(
        manifest: &serde_json::Value,
        policies: HashMap<String, String>,
        data_files: HashMap<String, String>,
    ) -> crate::Result<Self> {
        // Extract metadata from manifest
        let eunomia = manifest
            .get("metadata")
            .and_then(|m| m.get("eunomia"));

        let name = eunomia
            .and_then(|e| e.get("service"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| crate::Error::InvalidInput {
                reason: "manifest missing metadata.eunomia.service".to_string(),
            })?
            .to_string();

        let version = eunomia
            .and_then(|e| e.get("version"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| crate::Error::InvalidInput {
                reason: "manifest missing metadata.eunomia.version".to_string(),
            })?
            .to_string();

        let git_commit = eunomia
            .and_then(|e| e.get("git_commit"))
            .and_then(|v| v.as_str())
            .map(ToString::to_string);

        let created_at_str = eunomia
            .and_then(|e| e.get("created_at"))
            .and_then(|v| v.as_str());

        let created_at = created_at_str
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map_or_else(Utc::now, |dt| dt.with_timezone(&Utc));

        let revision = manifest
            .get("revision")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        let roots: Vec<String> = manifest
            .get("roots")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(ToString::to_string))
                    .collect()
            })
            .unwrap_or_default();

        Ok(Self {
            name,
            version,
            git_commit,
            created_at,
            policies,
            data_files,
            manifest: BundleManifest {
                revision,
                roots,
                opa_version: None,
                metadata: HashMap::new(),
            },
            signature: None,
            signing_key_id: None,
        })
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
    pub fn add_policy(
        mut self,
        package_name: impl Into<String>,
        source: impl Into<String>,
    ) -> Self {
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
        let bundle = Bundle::builder("users-service").version("1.0.0").build();

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
        let bundle = Bundle::builder("users-service").version("1.2.3").build();

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

    #[test]
    fn test_package_to_path() {
        assert_eq!(Bundle::package_to_path("users_service.authz"), "users_service/authz.rego");
        assert_eq!(Bundle::package_to_path("common.roles"), "common/roles.rego");
        assert_eq!(Bundle::package_to_path("policy"), "policy.rego");
        assert_eq!(
            Bundle::package_to_path("deep.nested.package.name"),
            "deep/nested/package/name.rego"
        );
    }

    #[test]
    fn test_path_to_package() {
        assert_eq!(Bundle::path_to_package("users_service/authz.rego"), "users_service.authz");
        assert_eq!(Bundle::path_to_package("common/roles.rego"), "common.roles");
        assert_eq!(Bundle::path_to_package("policy.rego"), "policy");
    }

    #[test]
    fn test_compute_checksum_deterministic() {
        let bundle1 = Bundle::builder("test")
            .version("1.0.0")
            .add_policy("a.policy", "package a")
            .add_policy("b.policy", "package b")
            .build();

        let bundle2 = Bundle::builder("test")
            .version("1.0.0")
            .add_policy("b.policy", "package b")
            .add_policy("a.policy", "package a")
            .build();

        // Checksums should be the same regardless of insertion order
        assert_eq!(bundle1.compute_checksum(), bundle2.compute_checksum());
    }

    #[test]
    fn test_compute_checksum_changes_with_content() {
        let bundle1 = Bundle::builder("test")
            .version("1.0.0")
            .add_policy("test.authz", "package test.authz")
            .build();

        let bundle2 = Bundle::builder("test")
            .version("1.0.0")
            .add_policy("test.authz", "package test.authz\ndefault allow := false")
            .build();

        assert_ne!(bundle1.compute_checksum(), bundle2.compute_checksum());
    }

    #[test]
    fn test_generate_manifest() {
        let bundle = Bundle::builder("users-service")
            .version("1.2.3")
            .git_commit("abc123")
            .add_root("users_service")
            .build();

        let manifest = bundle.generate_manifest();

        // Check standard OPA fields
        assert!(manifest.get("revision").is_some());
        assert_eq!(manifest["roots"], serde_json::json!(["users_service"]));

        // Check Eunomia extensions
        let eunomia = &manifest["metadata"]["eunomia"];
        assert_eq!(eunomia["version"], "1.2.3");
        assert_eq!(eunomia["service"], "users-service");
        assert_eq!(eunomia["git_commit"], "abc123");

        // Check checksum
        let checksum = &manifest["metadata"]["checksum"];
        assert_eq!(checksum["algorithm"], "sha256");
        assert!(checksum["value"].as_str().is_some());
    }

    #[test]
    fn test_bundle_roundtrip_bytes() {
        let original = Bundle::builder("test-service")
            .version("2.0.0")
            .git_commit("def456")
            .add_root("test_service")
            .add_policy("test_service.authz", "package test_service.authz\ndefault allow := false")
            .add_policy("common.roles", "package common.roles\nis_admin(roles) = roles[_] == \"admin\"")
            .add_data_file("test_service/data.json", r#"{"key": "value"}"#)
            .build();

        // Write to bytes
        let bytes = original.to_bytes().expect("should serialize to bytes");

        // Read back
        let restored = Bundle::from_bytes(&bytes).expect("should deserialize from bytes");

        // Verify
        assert_eq!(restored.name, "test-service");
        assert_eq!(restored.version, "2.0.0");
        assert_eq!(restored.git_commit, Some("def456".to_string()));
        assert_eq!(restored.policy_count(), 2);
        assert!(restored.has_policy("test_service.authz"));
        assert!(restored.has_policy("common.roles"));
        assert!(restored.data_files.contains_key("test_service/data.json"));
    }

    #[test]
    fn test_bundle_to_bytes_not_empty() {
        let bundle = Bundle::builder("test")
            .version("1.0.0")
            .add_policy("test.authz", "package test.authz")
            .build();

        let bytes = bundle.to_bytes().unwrap();
        assert!(!bytes.is_empty());
        // Should be valid gzip (starts with gzip magic bytes)
        assert_eq!(bytes[0], 0x1f);
        assert_eq!(bytes[1], 0x8b);
    }

    #[test]
    fn test_bundle_from_bytes_invalid() {
        let result = Bundle::from_bytes(b"not valid gzip data");
        assert!(result.is_err());
    }

    #[test]
    fn test_bundle_checksum_in_manifest() {
        let bundle = Bundle::builder("test")
            .version("1.0.0")
            .add_policy("test.authz", "package test.authz\ndefault allow := false")
            .build();

        let manifest = bundle.generate_manifest();
        let checksum = manifest["metadata"]["checksum"]["value"]
            .as_str()
            .expect("checksum should be string");

        // Should be valid hex SHA-256 (64 chars)
        assert_eq!(checksum.len(), 64);
        assert!(checksum.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
