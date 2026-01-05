//! Error types for registry operations.

use std::path::PathBuf;
use thiserror::Error;

/// Errors that can occur during registry operations.
#[derive(Debug, Error)]
pub enum RegistryError {
    /// Failed to connect to registry.
    #[error("Failed to connect to registry at {url}: {source}")]
    ConnectionFailed {
        /// Registry URL.
        url: String,
        /// Underlying error.
        #[source]
        source: reqwest::Error,
    },

    /// Authentication failed.
    #[error("Authentication failed: {message}")]
    AuthenticationFailed {
        /// Error message.
        message: String,
    },

    /// Bundle not found in registry.
    #[error("Bundle not found: {service}:{version}")]
    NotFound {
        /// Service name.
        service: String,
        /// Version.
        version: String,
    },

    /// Invalid bundle format.
    #[error("Invalid bundle format: {message}")]
    InvalidBundle {
        /// Error message.
        message: String,
    },

    /// Checksum mismatch during download.
    #[error("Checksum mismatch for {service}:{version}: expected {expected}, got {actual}")]
    ChecksumMismatch {
        /// Service name.
        service: String,
        /// Version.
        version: String,
        /// Expected checksum.
        expected: String,
        /// Actual checksum.
        actual: String,
    },

    /// Version resolution failed.
    #[error("Failed to resolve version '{query}' for {service}: {message}")]
    VersionResolutionFailed {
        /// Service name.
        service: String,
        /// Version query.
        query: String,
        /// Error message.
        message: String,
    },

    /// Cache operation failed.
    #[error("Cache operation failed: {message}")]
    CacheError {
        /// Error message.
        message: String,
    },

    /// File I/O error.
    #[error("File I/O error at {path}: {source}")]
    IoError {
        /// File path.
        path: PathBuf,
        /// Underlying error.
        #[source]
        source: std::io::Error,
    },

    /// HTTP error from registry.
    #[error("HTTP error from registry: {status} - {message}")]
    HttpError {
        /// HTTP status code.
        status: u16,
        /// Error message.
        message: String,
    },

    /// JSON serialization/deserialization error.
    #[error("JSON error: {source}")]
    JsonError {
        /// Underlying error.
        #[source]
        source: serde_json::Error,
    },

    /// Invalid URL.
    #[error("Invalid URL: {url}")]
    InvalidUrl {
        /// URL string.
        url: String,
    },

    /// Invalid reference format.
    #[error("Invalid reference format: {reference}")]
    InvalidReference {
        /// Reference string.
        reference: String,
    },

    /// Blob upload failed.
    #[error("Failed to upload blob: {message}")]
    UploadFailed {
        /// Error message.
        message: String,
    },

    /// Manifest push failed.
    #[error("Failed to push manifest for {service}:{version}: {message}")]
    ManifestPushFailed {
        /// Service name.
        service: String,
        /// Version.
        version: String,
        /// Error message.
        message: String,
    },

    /// Registry API not supported.
    #[error("Registry does not support required API: {feature}")]
    UnsupportedApi {
        /// Feature name.
        feature: String,
    },
}

impl From<reqwest::Error> for RegistryError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_connect() {
            Self::ConnectionFailed {
                url: err
                    .url()
                    .map_or_else(|| "unknown".to_string(), ToString::to_string),
                source: err,
            }
        } else if err.is_status() {
            let status = err.status().map_or(0, |s| s.as_u16());
            Self::HttpError {
                status,
                message: err.to_string(),
            }
        } else {
            Self::HttpError {
                status: 0,
                message: err.to_string(),
            }
        }
    }
}

impl From<serde_json::Error> for RegistryError {
    fn from(err: serde_json::Error) -> Self {
        Self::JsonError { source: err }
    }
}

impl From<std::io::Error> for RegistryError {
    fn from(err: std::io::Error) -> Self {
        Self::IoError {
            path: PathBuf::new(),
            source: err,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display_not_found() {
        let err = RegistryError::NotFound {
            service: "users-service".to_string(),
            version: "v1.2.0".to_string(),
        };
        assert_eq!(err.to_string(), "Bundle not found: users-service:v1.2.0");
    }

    #[test]
    fn test_error_display_checksum_mismatch() {
        let err = RegistryError::ChecksumMismatch {
            service: "test".to_string(),
            version: "v1.0.0".to_string(),
            expected: "abc123".to_string(),
            actual: "def456".to_string(),
        };
        assert!(err.to_string().contains("Checksum mismatch"));
    }

    #[test]
    fn test_error_display_auth_failed() {
        let err = RegistryError::AuthenticationFailed {
            message: "invalid token".to_string(),
        };
        assert_eq!(err.to_string(), "Authentication failed: invalid token");
    }
}
