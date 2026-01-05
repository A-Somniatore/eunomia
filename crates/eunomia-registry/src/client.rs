//! OCI Distribution API client for bundle registry operations.
//!
//! This module provides the main client interface for interacting with
//! OCI-compatible container registries.

use crate::cache::BundleCache;
use crate::config::{RegistryAuth, RegistryConfig};
use crate::error::RegistryError;
use crate::oci::{Descriptor, Manifest, MediaType, TagList};
use crate::version::{VersionQuery, VersionResolver};
use eunomia_core::Bundle;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION, CONTENT_TYPE};
use sha2::{Digest, Sha256};

/// Client for interacting with OCI-compatible bundle registries.
#[derive(Debug)]
pub struct RegistryClient {
    config: RegistryConfig,
    http: reqwest::Client,
    version_resolver: VersionResolver,
    cache: Option<BundleCache>,
}

impl RegistryClient {
    /// Creates a new registry client with the given configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP client cannot be created.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use eunomia_registry::{RegistryClient, RegistryConfig};
    ///
    /// let config = RegistryConfig::new("https://registry.example.com");
    /// let client = RegistryClient::new(config)?;
    /// # Ok::<(), eunomia_registry::RegistryError>(())
    /// ```
    pub fn new(config: RegistryConfig) -> Result<Self, RegistryError> {
        let http = Self::build_http_client(&config)?;

        Ok(Self {
            config,
            http,
            version_resolver: VersionResolver::new(),
            cache: None,
        })
    }

    /// Enables caching with the given cache instance.
    #[must_use]
    pub fn with_cache(mut self, cache: BundleCache) -> Self {
        self.cache = Some(cache);
        self
    }

    /// Returns the registry configuration.
    #[must_use]
    pub const fn config(&self) -> &RegistryConfig {
        &self.config
    }

    /// Checks if a bundle exists in the registry.
    ///
    /// # Arguments
    ///
    /// * `service` - Service name (repository name without namespace).
    /// * `version` - Version reference (tag or digest).
    ///
    /// # Errors
    ///
    /// Returns an error if the registry cannot be contacted.
    pub async fn exists(&self, service: &str, version: &str) -> Result<bool, RegistryError> {
        let repo = self.config.repository_name(service);
        let url = format!("{}/v2/{repo}/manifests/{version}", self.config.url);

        let response = self
            .http
            .head(&url)
            .headers(self.auth_headers()?)
            .header(ACCEPT, MediaType::OCI_MANIFEST)
            .send()
            .await?;

        Ok(response.status().is_success())
    }

    /// Lists all available tags for a service.
    ///
    /// # Arguments
    ///
    /// * `service` - Service name.
    ///
    /// # Returns
    ///
    /// List of available version tags.
    ///
    /// # Errors
    ///
    /// Returns an error if the tags cannot be retrieved.
    pub async fn list_tags(&self, service: &str) -> Result<Vec<String>, RegistryError> {
        let repo = self.config.repository_name(service);
        let url = format!("{}/v2/{repo}/tags/list", self.config.url);

        let response = self
            .http
            .get(&url)
            .headers(self.auth_headers()?)
            .send()
            .await?;

        if !response.status().is_success() {
            if response.status().as_u16() == 404 {
                return Ok(Vec::new());
            }
            return Err(RegistryError::HttpError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            });
        }

        let tag_list: TagList = response.json().await?;
        Ok(tag_list.tags)
    }

    /// Gets the latest version of a bundle.
    ///
    /// # Arguments
    ///
    /// * `service` - Service name.
    ///
    /// # Returns
    ///
    /// The latest semantic version tag.
    ///
    /// # Errors
    ///
    /// Returns an error if no versions are found.
    pub async fn get_latest_version(&self, service: &str) -> Result<String, RegistryError> {
        let tags = self.list_tags(service).await?;
        self.version_resolver
            .resolve(&VersionQuery::Latest, &tags, service)
    }

    /// Resolves a version query to a specific version.
    ///
    /// # Arguments
    ///
    /// * `service` - Service name.
    /// * `query` - Version query string (e.g., "latest", "v1.2", "v1.2.3").
    ///
    /// # Returns
    ///
    /// The resolved version tag.
    ///
    /// # Errors
    ///
    /// Returns an error if the version query is invalid or cannot be resolved.
    pub async fn resolve_version(
        &self,
        service: &str,
        query: &str,
    ) -> Result<String, RegistryError> {
        let query = VersionQuery::parse(query)?;

        if query.is_digest() {
            return Ok(query.to_string());
        }

        let tags = self.list_tags(service).await?;
        self.version_resolver.resolve(&query, &tags, service)
    }

    /// Fetches a bundle from the registry.
    ///
    /// If caching is enabled, checks the cache first.
    ///
    /// # Arguments
    ///
    /// * `service` - Service name.
    /// * `version` - Version reference.
    ///
    /// # Returns
    ///
    /// The requested bundle.
    ///
    /// # Errors
    ///
    /// Returns an error if the bundle cannot be fetched or is corrupt.
    pub async fn fetch(&self, service: &str, version: &str) -> Result<Bundle, RegistryError> {
        // Check cache first
        if let Some(ref cache) = self.cache {
            if let Some(bundle) = cache.get(service, version)? {
                return Ok(bundle);
            }
        }

        // Fetch manifest
        let manifest = self.fetch_manifest(service, version).await?;

        // Find bundle layer
        let bundle_layer = manifest.bundle_layer().ok_or_else(|| RegistryError::InvalidBundle {
            message: "Manifest does not contain a bundle layer".to_string(),
        })?;

        // Fetch bundle blob
        let bundle_data = self.fetch_blob(service, &bundle_layer.digest).await?;

        // Verify size
        if bundle_data.len() as u64 != bundle_layer.size {
            return Err(RegistryError::InvalidBundle {
                message: format!(
                    "Bundle size mismatch: expected {}, got {}",
                    bundle_layer.size,
                    bundle_data.len()
                ),
            });
        }

        // Verify digest
        let actual_digest = Self::compute_digest(&bundle_data);
        if actual_digest != bundle_layer.digest {
            return Err(RegistryError::ChecksumMismatch {
                service: service.to_string(),
                version: version.to_string(),
                expected: bundle_layer.digest.clone(),
                actual: actual_digest,
            });
        }

        // Parse bundle
        let bundle = Bundle::from_bytes(&bundle_data).map_err(|e| RegistryError::InvalidBundle {
            message: format!("Failed to parse bundle: {e}"),
        })?;

        // Cache if enabled
        if let Some(ref cache) = self.cache {
            if let Err(e) = cache.put(service, version, &bundle) {
                tracing::warn!(error = %e, "Failed to cache bundle");
            }
        }

        Ok(bundle)
    }

    /// Publishes a bundle to the registry.
    ///
    /// # Arguments
    ///
    /// * `service` - Service name.
    /// * `version` - Version tag for the bundle.
    /// * `bundle` - Bundle to publish.
    ///
    /// # Errors
    ///
    /// Returns an error if the bundle cannot be pushed.
    pub async fn publish(
        &self,
        service: &str,
        version: &str,
        bundle: &Bundle,
    ) -> Result<String, RegistryError> {
        // Serialize bundle
        let bundle_data = bundle.to_bytes().map_err(|e| RegistryError::UploadFailed {
            message: format!("Failed to serialize bundle: {e}"),
        })?;

        let bundle_digest = Self::compute_digest(&bundle_data);
        let bundle_size = bundle_data.len() as u64;

        // Upload bundle blob
        self.upload_blob(service, &bundle_data, &bundle_digest)
            .await?;

        // Create manifest
        let bundle_descriptor = Descriptor::new(MediaType::eunomia_bundle(), &bundle_digest, bundle_size)
            .with_annotation("org.opencontainers.image.title", format!("{service}-{version}.bundle.tar.gz"));

        let manifest = Manifest::for_bundle(bundle_descriptor, None)
            .with_annotation("org.opencontainers.image.version", version)
            .with_annotation("org.opencontainers.image.created", chrono::Utc::now().to_rfc3339());

        // Push manifest
        self.push_manifest(service, version, &manifest).await?;

        tracing::info!(service, version, digest = %bundle_digest, "Published bundle");

        Ok(bundle_digest)
    }

    /// Deletes a bundle from the registry.
    ///
    /// # Arguments
    ///
    /// * `service` - Service name.
    /// * `version` - Version reference to delete.
    ///
    /// # Errors
    ///
    /// Returns an error if the bundle cannot be deleted.
    pub async fn delete(&self, service: &str, version: &str) -> Result<(), RegistryError> {
        let repo = self.config.repository_name(service);
        let url = format!("{}/v2/{repo}/manifests/{version}", self.config.url);

        let response = self
            .http
            .delete(&url)
            .headers(self.auth_headers()?)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(RegistryError::HttpError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            });
        }

        // Invalidate cache
        if let Some(ref cache) = self.cache {
            cache.invalidate(service, version)?;
        }

        Ok(())
    }

    /// Fetches a manifest from the registry.
    async fn fetch_manifest(&self, service: &str, version: &str) -> Result<Manifest, RegistryError> {
        let repo = self.config.repository_name(service);
        let url = format!("{}/v2/{repo}/manifests/{version}", self.config.url);

        let response = self
            .http
            .get(&url)
            .headers(self.auth_headers()?)
            .header(ACCEPT, MediaType::OCI_MANIFEST)
            .send()
            .await?;

        if !response.status().is_success() {
            if response.status().as_u16() == 404 {
                return Err(RegistryError::NotFound {
                    service: service.to_string(),
                    version: version.to_string(),
                });
            }
            return Err(RegistryError::HttpError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            });
        }

        response.json().await.map_err(Into::into)
    }

    /// Fetches a blob from the registry.
    async fn fetch_blob(&self, service: &str, digest: &str) -> Result<Vec<u8>, RegistryError> {
        let repo = self.config.repository_name(service);
        let url = format!("{}/v2/{repo}/blobs/{digest}", self.config.url);

        let response = self
            .http
            .get(&url)
            .headers(self.auth_headers()?)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(RegistryError::HttpError {
                status: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            });
        }

        response.bytes().await.map(|b| b.to_vec()).map_err(Into::into)
    }

    /// Uploads a blob to the registry.
    async fn upload_blob(
        &self,
        service: &str,
        data: &[u8],
        digest: &str,
    ) -> Result<(), RegistryError> {
        let repo = self.config.repository_name(service);

        // Start upload session
        let start_url = format!("{}/v2/{repo}/blobs/uploads/", self.config.url);

        let response = self
            .http
            .post(&start_url)
            .headers(self.auth_headers()?)
            .send()
            .await?;

        if !response.status().is_success() && response.status().as_u16() != 202 {
            return Err(RegistryError::UploadFailed {
                message: format!("Failed to start upload: {}", response.status()),
            });
        }

        // Get upload location
        let location = response
            .headers()
            .get("location")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| RegistryError::UploadFailed {
                message: "No upload location returned".to_string(),
            })?;

        // Complete upload with PUT
        let upload_url = if location.contains('?') {
            format!("{location}&digest={digest}")
        } else {
            format!("{location}?digest={digest}")
        };

        let response = self
            .http
            .put(&upload_url)
            .headers(self.auth_headers()?)
            .header(CONTENT_TYPE, "application/octet-stream")
            .body(data.to_vec())
            .send()
            .await?;

        if !response.status().is_success() && response.status().as_u16() != 201 {
            return Err(RegistryError::UploadFailed {
                message: format!("Failed to upload blob: {}", response.status()),
            });
        }

        Ok(())
    }

    /// Pushes a manifest to the registry.
    async fn push_manifest(
        &self,
        service: &str,
        version: &str,
        manifest: &Manifest,
    ) -> Result<(), RegistryError> {
        let repo = self.config.repository_name(service);
        let url = format!("{}/v2/{repo}/manifests/{version}", self.config.url);

        let manifest_json = serde_json::to_vec(manifest)?;

        let response = self
            .http
            .put(&url)
            .headers(self.auth_headers()?)
            .header(CONTENT_TYPE, MediaType::OCI_MANIFEST)
            .body(manifest_json)
            .send()
            .await?;

        if !response.status().is_success() && response.status().as_u16() != 201 {
            return Err(RegistryError::ManifestPushFailed {
                service: service.to_string(),
                version: version.to_string(),
                message: format!(
                    "{}: {}",
                    response.status(),
                    response.text().await.unwrap_or_default()
                ),
            });
        }

        Ok(())
    }

    /// Builds the HTTP client with proper configuration.
    fn build_http_client(config: &RegistryConfig) -> Result<reqwest::Client, RegistryError> {
        let mut builder = reqwest::Client::builder()
            .timeout(config.timeout)
            .user_agent(&config.user_agent);

        // Configure TLS if provided
        if let Some(ref tls) = config.tls {
            if tls.insecure_skip_verify {
                builder = builder.danger_accept_invalid_certs(true);
            }

            if let Some(ref ca_cert) = tls.ca_cert {
                let cert_pem = std::fs::read(ca_cert).map_err(|e| RegistryError::IoError {
                    path: ca_cert.clone(),
                    source: e,
                })?;
                let cert = reqwest::Certificate::from_pem(&cert_pem).map_err(|e| {
                    RegistryError::CacheError {
                        message: format!("Invalid CA certificate: {e}"),
                    }
                })?;
                builder = builder.add_root_certificate(cert);
            }

            if let (Some(ref cert_path), Some(ref key_path)) =
                (&tls.client_cert, &tls.client_key)
            {
                let mut cert_pem = std::fs::read(cert_path).map_err(|e| RegistryError::IoError {
                    path: cert_path.clone(),
                    source: e,
                })?;
                let key_pem = std::fs::read(key_path).map_err(|e| RegistryError::IoError {
                    path: key_path.clone(),
                    source: e,
                })?;
                cert_pem.extend_from_slice(&key_pem);

                let identity = reqwest::Identity::from_pem(&cert_pem).map_err(|e| {
                    RegistryError::CacheError {
                        message: format!("Invalid client certificate: {e}"),
                    }
                })?;
                builder = builder.identity(identity);
            }
        }

        builder.build().map_err(|e| RegistryError::ConnectionFailed {
            url: config.url.clone(),
            source: e,
        })
    }

    /// Creates authentication headers based on configuration.
    fn auth_headers(&self) -> Result<HeaderMap, RegistryError> {
        let mut headers = HeaderMap::new();

        match &self.config.auth {
            RegistryAuth::None => {}
            RegistryAuth::Basic { username, password } => {
                let credentials = base64::Engine::encode(
                    &base64::engine::general_purpose::STANDARD,
                    format!("{username}:{password}"),
                );
                headers.insert(
                    AUTHORIZATION,
                    HeaderValue::from_str(&format!("Basic {credentials}"))
                        .map_err(|_| RegistryError::AuthenticationFailed {
                            message: "Invalid credentials".to_string(),
                        })?,
                );
            }
            RegistryAuth::Bearer { token } => {
                headers.insert(
                    AUTHORIZATION,
                    HeaderValue::from_str(&format!("Bearer {token}"))
                        .map_err(|_| RegistryError::AuthenticationFailed {
                            message: "Invalid token".to_string(),
                        })?,
                );
            }
            RegistryAuth::AwsEcr { .. } | RegistryAuth::GcpArtifact { .. } => {
                // Cloud provider auth would be implemented here
                // For now, return error as not yet implemented
                return Err(RegistryError::UnsupportedApi {
                    feature: "Cloud provider authentication".to_string(),
                });
            }
        }

        Ok(headers)
    }

    /// Computes SHA-256 digest of data.
    fn compute_digest(data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        format!("sha256:{}", hex::encode(hasher.finalize()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let config = RegistryConfig::new("https://registry.example.com");
        let client = RegistryClient::new(config);
        assert!(client.is_ok());
    }

    #[test]
    fn test_compute_digest() {
        let data = b"test data";
        let digest = RegistryClient::compute_digest(data);
        assert!(digest.starts_with("sha256:"));
        assert_eq!(digest.len(), 7 + 64); // "sha256:" + 64 hex chars
    }

    #[test]
    fn test_auth_headers_none() {
        let config = RegistryConfig::new("https://example.com");
        let client = RegistryClient::new(config).unwrap();
        let headers = client.auth_headers().unwrap();
        assert!(headers.is_empty());
    }

    #[test]
    fn test_auth_headers_basic() {
        let config = RegistryConfig::new("https://example.com")
            .with_auth(RegistryAuth::basic("user", "pass"));
        let client = RegistryClient::new(config).unwrap();
        let headers = client.auth_headers().unwrap();
        
        assert!(headers.contains_key(AUTHORIZATION));
        let auth = headers.get(AUTHORIZATION).unwrap().to_str().unwrap();
        assert!(auth.starts_with("Basic "));
    }

    #[test]
    fn test_auth_headers_bearer() {
        let config = RegistryConfig::new("https://example.com")
            .with_auth(RegistryAuth::bearer("my-token"));
        let client = RegistryClient::new(config).unwrap();
        let headers = client.auth_headers().unwrap();
        
        assert!(headers.contains_key(AUTHORIZATION));
        let auth = headers.get(AUTHORIZATION).unwrap().to_str().unwrap();
        assert_eq!(auth, "Bearer my-token");
    }
}
