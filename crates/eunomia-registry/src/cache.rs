//! Local bundle cache for offline access and performance.
//!
//! Provides a file-based cache with LRU eviction for policy bundles.

use crate::error::RegistryError;
use eunomia_core::Bundle;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Configuration for the bundle cache.
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Cache directory (default: `~/.eunomia/cache`).
    pub dir: PathBuf,

    /// Maximum cache size in bytes (default: 1GB).
    pub max_size: u64,

    /// Time-to-live for cache entries (default: 7 days).
    pub ttl: Duration,

    /// Enable cache integrity verification.
    pub verify_checksums: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            dir: dirs_default_cache_dir(),
            max_size: 1024 * 1024 * 1024, // 1GB
            ttl: Duration::from_secs(7 * 24 * 60 * 60), // 7 days
            verify_checksums: true,
        }
    }
}

impl CacheConfig {
    /// Creates a new cache configuration with the given directory.
    #[must_use]
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self {
            dir: dir.into(),
            ..Default::default()
        }
    }

    /// Sets the maximum cache size.
    #[must_use]
    pub const fn with_max_size(mut self, size: u64) -> Self {
        self.max_size = size;
        self
    }

    /// Sets the TTL for cache entries.
    #[must_use]
    pub const fn with_ttl(mut self, ttl: Duration) -> Self {
        self.ttl = ttl;
        self
    }

    /// Enables or disables checksum verification.
    #[must_use]
    pub const fn with_verify_checksums(mut self, verify: bool) -> Self {
        self.verify_checksums = verify;
        self
    }
}

/// Default cache directory.
fn dirs_default_cache_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("eunomia")
}

/// File-based bundle cache with LRU eviction.
#[derive(Debug)]
pub struct BundleCache {
    config: CacheConfig,
}

impl BundleCache {
    /// Creates a new bundle cache with the given configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if the cache directory cannot be created.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use eunomia_registry::{BundleCache, CacheConfig};
    ///
    /// let cache = BundleCache::new(CacheConfig::default())?;
    /// # Ok::<(), eunomia_registry::RegistryError>(())
    /// ```
    pub fn new(config: CacheConfig) -> Result<Self, RegistryError> {
        // Create cache directory structure
        std::fs::create_dir_all(&config.dir).map_err(|e| RegistryError::IoError {
            path: config.dir.clone(),
            source: e,
        })?;
        std::fs::create_dir_all(config.dir.join("bundles")).map_err(|e| RegistryError::IoError {
            path: config.dir.join("bundles"),
            source: e,
        })?;
        std::fs::create_dir_all(config.dir.join("signatures")).map_err(|e| {
            RegistryError::IoError {
                path: config.dir.join("signatures"),
                source: e,
            }
        })?;

        Ok(Self { config })
    }

    /// Returns the cache configuration.
    #[must_use]
    pub const fn config(&self) -> &CacheConfig {
        &self.config
    }

    /// Returns the path to a cached bundle file.
    #[must_use]
    pub fn bundle_path(&self, service: &str, version: &str) -> PathBuf {
        self.config
            .dir
            .join("bundles")
            .join(service)
            .join(format!("{version}.bundle.tar.gz"))
    }

    /// Returns the path to a cached manifest file.
    #[must_use]
    pub fn manifest_path(&self, service: &str, version: &str) -> PathBuf {
        self.config
            .dir
            .join("bundles")
            .join(service)
            .join(format!("{version}.manifest.json"))
    }

    /// Returns the path to a cached signature file.
    #[must_use]
    pub fn signature_path(&self, service: &str, version: &str) -> PathBuf {
        self.config
            .dir
            .join("signatures")
            .join(service)
            .join(format!("{version}.sig"))
    }

    /// Retrieves a bundle from the cache.
    ///
    /// Returns `None` if the bundle is not cached or has expired.
    ///
    /// # Errors
    ///
    /// Returns an error if the cached bundle is corrupt or cannot be read.
    pub fn get(&self, service: &str, version: &str) -> Result<Option<Bundle>, RegistryError> {
        let bundle_path = self.bundle_path(service, version);

        if !bundle_path.exists() {
            return Ok(None);
        }

        // Check TTL
        if self.is_expired(&bundle_path)? {
            tracing::debug!(
                service,
                version,
                "Cache entry expired, removing"
            );
            self.invalidate(service, version)?;
            return Ok(None);
        }

        // Load bundle
        let bundle = Bundle::from_file(&bundle_path).map_err(|e| RegistryError::CacheError {
            message: format!("Failed to load cached bundle: {e}"),
        })?;

        // Verify checksum if enabled
        if self.config.verify_checksums {
            let manifest_path = self.manifest_path(service, version);
            if manifest_path.exists() {
                let stored_checksum = Self::read_stored_checksum(&manifest_path)?;
                if let Some(expected) = stored_checksum {
                    let actual = bundle.compute_checksum();
                    if actual != expected {
                        tracing::warn!(
                            service,
                            version,
                            expected,
                            actual,
                            "Cache checksum mismatch, invalidating"
                        );
                        self.invalidate(service, version)?;
                        return Ok(None);
                    }
                }
            }
        }

        tracing::debug!(service, version, "Cache hit");
        Ok(Some(bundle))
    }

    /// Stores a bundle in the cache.
    ///
    /// # Errors
    ///
    /// Returns an error if the bundle cannot be written to disk.
    pub fn put(&self, service: &str, version: &str, bundle: &Bundle) -> Result<(), RegistryError> {
        let bundle_path = self.bundle_path(service, version);
        let manifest_path = self.manifest_path(service, version);

        // Create service directory
        if let Some(parent) = bundle_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| RegistryError::IoError {
                path: parent.to_path_buf(),
                source: e,
            })?;
        }

        // Write bundle
        bundle
            .write_to_file(&bundle_path)
            .map_err(|e| RegistryError::CacheError {
                message: format!("Failed to write bundle to cache: {e}"),
            })?;

        // Write manifest with checksum
        let manifest_data = serde_json::json!({
            "version": version,
            "checksum": bundle.compute_checksum(),
            "cached_at": chrono::Utc::now().to_rfc3339(),
        });

        std::fs::write(
            &manifest_path,
            serde_json::to_string_pretty(&manifest_data)?,
        )
        .map_err(|e| RegistryError::IoError {
            path: manifest_path,
            source: e,
        })?;

        tracing::debug!(service, version, "Cached bundle");

        // Enforce size limit
        self.enforce_size_limit()?;

        Ok(())
    }

    /// Removes a specific entry from the cache.
    ///
    /// # Errors
    ///
    /// Returns an error if files cannot be deleted.
    pub fn invalidate(&self, service: &str, version: &str) -> Result<(), RegistryError> {
        let bundle_path = self.bundle_path(service, version);
        let manifest_path = self.manifest_path(service, version);
        let sig_path = self.signature_path(service, version);

        for path in [bundle_path, manifest_path, sig_path] {
            if path.exists() {
                std::fs::remove_file(&path).map_err(|e| RegistryError::IoError {
                    path,
                    source: e,
                })?;
            }
        }

        Ok(())
    }

    /// Clears the entire cache.
    ///
    /// # Errors
    ///
    /// Returns an error if the cache directory cannot be cleared.
    pub fn clear(&self) -> Result<(), RegistryError> {
        for subdir in ["bundles", "signatures"] {
            let path = self.config.dir.join(subdir);
            if path.exists() {
                std::fs::remove_dir_all(&path).map_err(|e| RegistryError::IoError {
                    path: path.clone(),
                    source: e,
                })?;
                std::fs::create_dir_all(&path).map_err(|e| RegistryError::IoError {
                    path,
                    source: e,
                })?;
            }
        }

        tracing::info!("Cache cleared");
        Ok(())
    }

    /// Removes expired entries and enforces the size limit.
    ///
    /// # Errors
    ///
    /// Returns an error if cache files cannot be accessed or deleted.
    pub fn prune(&self) -> Result<PruneStats, RegistryError> {
        let mut stats = PruneStats::default();

        // Remove expired entries
        let bundles_dir = self.config.dir.join("bundles");
        if bundles_dir.exists() {
            for entry in walkdir::WalkDir::new(&bundles_dir)
                .into_iter()
                .filter_map(Result::ok)
                .filter(|e| e.path().extension().is_some_and(|ext| ext == "gz"))
            {
                if self.is_expired(entry.path())? {
                    if let Err(e) = std::fs::remove_file(entry.path()) {
                        tracing::warn!(path = ?entry.path(), error = %e, "Failed to remove expired cache entry");
                    } else {
                        stats.expired_removed += 1;
                    }
                }
            }
        }

        // Enforce size limit
        stats.size_evicted = self.enforce_size_limit()?;

        Ok(stats)
    }

    /// Returns the total size of the cache in bytes.
    ///
    /// # Errors
    ///
    /// Returns an error if cache directory cannot be read.
    pub fn size(&self) -> Result<u64, RegistryError> {
        let mut total = 0;

        for subdir in ["bundles", "signatures"] {
            let path = self.config.dir.join(subdir);
            if path.exists() {
                for entry in walkdir::WalkDir::new(&path)
                    .into_iter()
                    .filter_map(Result::ok)
                    .filter(|e| e.file_type().is_file())
                {
                    if let Ok(metadata) = entry.metadata() {
                        total += metadata.len();
                    }
                }
            }
        }

        Ok(total)
    }

    /// Checks if a cache entry has expired based on file modification time.
    fn is_expired(&self, path: &Path) -> Result<bool, RegistryError> {
        let metadata = std::fs::metadata(path).map_err(|e| RegistryError::IoError {
            path: path.to_path_buf(),
            source: e,
        })?;

        let modified = metadata.modified().map_err(|e| RegistryError::IoError {
            path: path.to_path_buf(),
            source: e,
        })?;

        let age = modified.elapsed().unwrap_or(Duration::ZERO);
        Ok(age > self.config.ttl)
    }

    /// Reads the stored checksum from a manifest file.
    fn read_stored_checksum(path: &Path) -> Result<Option<String>, RegistryError> {
        let content = std::fs::read_to_string(path).map_err(|e| RegistryError::IoError {
            path: path.to_path_buf(),
            source: e,
        })?;

        let manifest: serde_json::Value = serde_json::from_str(&content)?;
        Ok(manifest.get("checksum").and_then(|v| v.as_str()).map(String::from))
    }

    /// Enforces the cache size limit by removing oldest entries.
    fn enforce_size_limit(&self) -> Result<u64, RegistryError> {
        let current_size = self.size()?;
        if current_size <= self.config.max_size {
            return Ok(0);
        }

        let mut entries: Vec<(PathBuf, std::time::SystemTime, u64)> = Vec::new();

        let bundles_dir = self.config.dir.join("bundles");
        if bundles_dir.exists() {
            for entry in walkdir::WalkDir::new(&bundles_dir)
                .into_iter()
                .filter_map(Result::ok)
                .filter(|e| e.file_type().is_file())
            {
                if let Ok(metadata) = entry.metadata() {
                    if let Ok(modified) = metadata.modified() {
                        entries.push((entry.path().to_path_buf(), modified, metadata.len()));
                    }
                }
            }
        }

        // Sort by modification time (oldest first)
        entries.sort_by_key(|(_, time, _)| *time);

        let mut removed_size = 0u64;
        let target_size = self.config.max_size * 9 / 10; // Remove until 90% of limit

        for (path, _, size) in entries {
            if current_size - removed_size <= target_size {
                break;
            }

            if std::fs::remove_file(&path).is_ok() {
                removed_size += size;
                tracing::debug!(?path, size, "Evicted cache entry");
            }
        }

        Ok(removed_size)
    }

    /// Computes SHA-256 digest of data.
    #[allow(dead_code)]
    fn compute_digest(data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        format!("sha256:{}", hex::encode(hasher.finalize()))
    }
}

/// Statistics from a cache prune operation.
#[derive(Debug, Default)]
pub struct PruneStats {
    /// Number of expired entries removed.
    pub expired_removed: u64,

    /// Bytes evicted due to size limit.
    pub size_evicted: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_config_default() {
        let config = CacheConfig::default();
        assert_eq!(config.max_size, 1024 * 1024 * 1024);
        assert!(config.verify_checksums);
    }

    #[test]
    fn test_cache_config_builder() {
        let config = CacheConfig::new("/tmp/test-cache")
            .with_max_size(100 * 1024 * 1024)
            .with_ttl(Duration::from_secs(3600))
            .with_verify_checksums(false);

        assert_eq!(config.dir, PathBuf::from("/tmp/test-cache"));
        assert_eq!(config.max_size, 100 * 1024 * 1024);
        assert_eq!(config.ttl, Duration::from_secs(3600));
        assert!(!config.verify_checksums);
    }

    #[test]
    fn test_bundle_path() {
        let config = CacheConfig::new("/cache");
        let cache = BundleCache { config };

        let path = cache.bundle_path("users-service", "v1.2.0");
        assert_eq!(
            path,
            PathBuf::from("/cache/bundles/users-service/v1.2.0.bundle.tar.gz")
        );
    }

    #[test]
    fn test_manifest_path() {
        let config = CacheConfig::new("/cache");
        let cache = BundleCache { config };

        let path = cache.manifest_path("users-service", "v1.2.0");
        assert_eq!(
            path,
            PathBuf::from("/cache/bundles/users-service/v1.2.0.manifest.json")
        );
    }

    #[test]
    fn test_signature_path() {
        let config = CacheConfig::new("/cache");
        let cache = BundleCache { config };

        let path = cache.signature_path("users-service", "v1.2.0");
        assert_eq!(
            path,
            PathBuf::from("/cache/signatures/users-service/v1.2.0.sig")
        );
    }
}
