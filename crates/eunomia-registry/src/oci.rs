//! OCI Distribution Specification types.
//!
//! This module defines types that conform to the OCI Distribution Specification
//! for container registry APIs.

use serde::{Deserialize, Serialize};

/// OCI media types for Eunomia policy bundles.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MediaType(String);

impl MediaType {
    /// OCI image manifest media type.
    pub const OCI_MANIFEST: &'static str = "application/vnd.oci.image.manifest.v1+json";

    /// OCI image index media type.
    pub const OCI_INDEX: &'static str = "application/vnd.oci.image.index.v1+json";

    /// Eunomia policy bundle media type.
    pub const EUNOMIA_BUNDLE: &'static str = "application/vnd.eunomia.policy.bundle.v1+tar.gz";

    /// Eunomia policy manifest media type.
    pub const EUNOMIA_MANIFEST: &'static str = "application/vnd.eunomia.policy.manifest.v1+json";

    /// Eunomia policy signature media type.
    pub const EUNOMIA_SIGNATURE: &'static str = "application/vnd.eunomia.policy.signature.v1+json";

    /// Creates a new media type.
    #[must_use]
    pub fn new(media_type: impl Into<String>) -> Self {
        Self(media_type.into())
    }

    /// Returns the media type string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Creates the Eunomia bundle media type.
    #[must_use]
    pub fn eunomia_bundle() -> Self {
        Self::new(Self::EUNOMIA_BUNDLE)
    }

    /// Creates the Eunomia manifest media type.
    #[must_use]
    pub fn eunomia_manifest() -> Self {
        Self::new(Self::EUNOMIA_MANIFEST)
    }

    /// Creates the Eunomia signature media type.
    #[must_use]
    pub fn eunomia_signature() -> Self {
        Self::new(Self::EUNOMIA_SIGNATURE)
    }
}

impl std::fmt::Display for MediaType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for MediaType {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl Serialize for MediaType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for MediaType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(Self(s))
    }
}

/// OCI content descriptor.
///
/// A descriptor describes the disposition of targeted content. It includes
/// the type of the content, a content identifier (digest), and the byte-size
/// of the raw content.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Descriptor {
    /// Media type of the referenced content.
    pub media_type: MediaType,

    /// Digest of the targeted content.
    pub digest: String,

    /// Size in bytes of the content.
    pub size: u64,

    /// Optional URLs for alternative locations.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub urls: Option<Vec<String>>,

    /// Optional annotations (key-value metadata).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<std::collections::HashMap<String, String>>,
}

impl Descriptor {
    /// Creates a new descriptor.
    ///
    /// # Examples
    ///
    /// ```
    /// use eunomia_registry::{Descriptor, MediaType};
    ///
    /// let desc = Descriptor::new(
    ///     MediaType::eunomia_bundle(),
    ///     "sha256:abc123...",
    ///     1024,
    /// );
    /// ```
    #[must_use]
    pub fn new(media_type: MediaType, digest: impl Into<String>, size: u64) -> Self {
        Self {
            media_type,
            digest: digest.into(),
            size,
            urls: None,
            annotations: None,
        }
    }

    /// Adds an annotation to the descriptor.
    #[must_use]
    pub fn with_annotation(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.annotations
            .get_or_insert_with(std::collections::HashMap::new)
            .insert(key.into(), value.into());
        self
    }

    /// Returns the digest algorithm (e.g., "sha256").
    #[must_use]
    pub fn digest_algorithm(&self) -> &str {
        self.digest.split(':').next().unwrap_or("sha256")
    }

    /// Returns the digest value (without algorithm prefix).
    #[must_use]
    pub fn digest_value(&self) -> &str {
        self.digest.split(':').nth(1).unwrap_or(&self.digest)
    }
}

/// OCI Image Manifest.
///
/// This structure describes a single container image or artifact.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    /// Schema version (always 2).
    pub schema_version: u32,

    /// Media type of this manifest.
    pub media_type: MediaType,

    /// Configuration descriptor (optional for artifacts).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<Descriptor>,

    /// Layers that make up the artifact.
    pub layers: Vec<Descriptor>,

    /// Optional annotations.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<std::collections::HashMap<String, String>>,

    /// Optional artifact type (OCI 1.1+).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifact_type: Option<String>,

    /// Optional subject descriptor for referrers API (OCI 1.1+).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<Descriptor>,
}

impl Manifest {
    /// Creates a new manifest with the given layers.
    ///
    /// # Examples
    ///
    /// ```
    /// use eunomia_registry::{Manifest, Descriptor, MediaType};
    ///
    /// let bundle_layer = Descriptor::new(
    ///     MediaType::eunomia_bundle(),
    ///     "sha256:abc123...",
    ///     1024,
    /// );
    ///
    /// let manifest = Manifest::new(vec![bundle_layer]);
    /// ```
    #[must_use]
    pub fn new(layers: Vec<Descriptor>) -> Self {
        Self {
            schema_version: 2,
            media_type: MediaType::new(MediaType::OCI_MANIFEST),
            config: None,
            layers,
            annotations: None,
            artifact_type: Some(MediaType::EUNOMIA_BUNDLE.to_string()),
            subject: None,
        }
    }

    /// Creates a manifest for an Eunomia policy bundle.
    #[must_use]
    pub fn for_bundle(
        bundle_descriptor: Descriptor,
        signature_descriptor: Option<Descriptor>,
    ) -> Self {
        let mut layers = vec![bundle_descriptor];
        if let Some(sig) = signature_descriptor {
            layers.push(sig);
        }

        Self {
            schema_version: 2,
            media_type: MediaType::new(MediaType::OCI_MANIFEST),
            config: None,
            layers,
            annotations: None,
            artifact_type: Some(MediaType::EUNOMIA_BUNDLE.to_string()),
            subject: None,
        }
    }

    /// Adds an annotation to the manifest.
    #[must_use]
    pub fn with_annotation(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.annotations
            .get_or_insert_with(std::collections::HashMap::new)
            .insert(key.into(), value.into());
        self
    }

    /// Returns the bundle layer descriptor, if present.
    #[must_use]
    pub fn bundle_layer(&self) -> Option<&Descriptor> {
        self.layers
            .iter()
            .find(|d| d.media_type.as_str() == MediaType::EUNOMIA_BUNDLE)
    }

    /// Returns the signature layer descriptor, if present.
    #[must_use]
    pub fn signature_layer(&self) -> Option<&Descriptor> {
        self.layers
            .iter()
            .find(|d| d.media_type.as_str() == MediaType::EUNOMIA_SIGNATURE)
    }
}

/// Response from the `/v2/<name>/tags/list` endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagList {
    /// Repository name.
    pub name: String,

    /// List of tags.
    pub tags: Vec<String>,
}

/// Error response from registry API.
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    /// List of errors.
    pub errors: Vec<RegistryApiError>,
}

/// Individual error from registry API.
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryApiError {
    /// Error code.
    pub code: String,

    /// Human-readable message.
    pub message: String,

    /// Additional details.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_media_type_eunomia_bundle() {
        let mt = MediaType::eunomia_bundle();
        assert_eq!(
            mt.as_str(),
            "application/vnd.eunomia.policy.bundle.v1+tar.gz"
        );
    }

    #[test]
    fn test_descriptor_new() {
        let desc = Descriptor::new(MediaType::eunomia_bundle(), "sha256:abc123def456", 1024);
        assert_eq!(desc.size, 1024);
        assert_eq!(desc.digest, "sha256:abc123def456");
        assert_eq!(desc.digest_algorithm(), "sha256");
        assert_eq!(desc.digest_value(), "abc123def456");
    }

    #[test]
    fn test_descriptor_with_annotation() {
        let desc = Descriptor::new(MediaType::eunomia_bundle(), "sha256:abc123", 100)
            .with_annotation("version", "1.2.0");

        assert!(desc.annotations.is_some());
        let annot = desc.annotations.as_ref().unwrap();
        assert_eq!(annot.get("version"), Some(&"1.2.0".to_string()));
    }

    #[test]
    fn test_manifest_new() {
        let layer = Descriptor::new(MediaType::eunomia_bundle(), "sha256:abc", 100);
        let manifest = Manifest::new(vec![layer]);

        assert_eq!(manifest.schema_version, 2);
        assert_eq!(manifest.layers.len(), 1);
        assert_eq!(
            manifest.artifact_type,
            Some(MediaType::EUNOMIA_BUNDLE.to_string())
        );
    }

    #[test]
    fn test_manifest_for_bundle() {
        let bundle = Descriptor::new(MediaType::eunomia_bundle(), "sha256:bundle", 1000);
        let sig = Descriptor::new(MediaType::eunomia_signature(), "sha256:sig", 100);

        let manifest = Manifest::for_bundle(bundle, Some(sig));

        assert!(manifest.bundle_layer().is_some());
        assert!(manifest.signature_layer().is_some());
    }

    #[test]
    fn test_manifest_serialization() {
        let layer = Descriptor::new(MediaType::eunomia_bundle(), "sha256:abc", 100);
        let manifest =
            Manifest::new(vec![layer]).with_annotation("org.opencontainers.image.version", "1.2.0");

        let json = serde_json::to_string_pretty(&manifest).unwrap();
        assert!(json.contains("schemaVersion"));
        assert!(json.contains("mediaType"));
        assert!(json.contains("layers"));
    }

    #[test]
    fn test_tag_list_deserialization() {
        let json = r#"{
            "name": "policies/users-service",
            "tags": ["v1.0.0", "v1.1.0", "v1.2.0", "latest"]
        }"#;

        let tags: TagList = serde_json::from_str(json).unwrap();
        assert_eq!(tags.name, "policies/users-service");
        assert_eq!(tags.tags.len(), 4);
    }
}
