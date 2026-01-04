//! Error types for Eunomia core operations.
//!
//! This module defines the error types used throughout the `eunomia-core` crate.

use thiserror::Error;

/// Result type alias using [`Error`] as the error type.
pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can occur in Eunomia core operations.
#[derive(Error, Debug)]
pub enum Error {
    /// Policy file could not be loaded.
    #[error("Failed to load policy from {path}: {source}")]
    PolicyLoadError {
        /// Path to the policy file.
        path: String,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },

    /// Policy parsing failed.
    #[error("Failed to parse policy: {reason}")]
    PolicyParseError {
        /// Reason for the parse failure.
        reason: String,
    },

    /// Policy validation failed.
    #[error("Policy validation failed: {reason}")]
    PolicyValidationError {
        /// Reason for the validation failure.
        reason: String,
    },

    /// Bundle creation failed.
    #[error("Failed to create bundle: {reason}")]
    BundleCreationError {
        /// Reason for the bundle creation failure.
        reason: String,
    },

    /// Bundle signature verification failed.
    #[error("Bundle signature verification failed: {reason}")]
    BundleSignatureError {
        /// Reason for the signature verification failure.
        reason: String,
    },

    /// Serialization/deserialization error.
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    /// Invalid input provided.
    #[error("Invalid input: {reason}")]
    InvalidInput {
        /// Reason the input is invalid.
        reason: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display_policy_parse() {
        let err = Error::PolicyParseError {
            reason: "unexpected token".to_string(),
        };
        assert_eq!(err.to_string(), "Failed to parse policy: unexpected token");
    }

    #[test]
    fn test_error_display_validation() {
        let err = Error::PolicyValidationError {
            reason: "missing default rule".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "Policy validation failed: missing default rule"
        );
    }

    #[test]
    fn test_error_display_bundle() {
        let err = Error::BundleCreationError {
            reason: "no policies found".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "Failed to create bundle: no policies found"
        );
    }

    #[test]
    fn test_error_display_invalid_input() {
        let err = Error::InvalidInput {
            reason: "service name cannot be empty".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "Invalid input: service name cannot be empty"
        );
    }
}
