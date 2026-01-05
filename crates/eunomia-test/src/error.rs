//! Error types for the Eunomia testing framework.

use std::path::PathBuf;
use thiserror::Error;

/// Result type alias for test operations.
pub type Result<T> = std::result::Result<T, TestError>;

/// Errors that can occur during policy testing.
#[derive(Error, Debug)]
pub enum TestError {
    /// Failed to read a test file.
    #[error("Failed to read test file {path}: {source}")]
    FileReadError {
        /// Path to the file.
        path: PathBuf,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },

    /// I/O error with path context.
    #[error("I/O error for {path}: {source}")]
    Io {
        /// Path involved in the operation.
        path: PathBuf,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },

    /// Failed to parse test fixtures.
    #[error("Failed to parse test fixtures: {message}")]
    FixtureParseError {
        /// Error message.
        message: String,
    },

    /// Parse error (general).
    #[error("Parse error: {0}")]
    Parse(String),

    /// Discovery error.
    #[error("Test discovery failed: {0}")]
    Discovery(String),

    /// Test execution failed.
    #[error("Test execution failed: {message}")]
    ExecutionError {
        /// Error message.
        message: String,
    },

    /// Invalid test configuration.
    #[error("Invalid test configuration: {message}")]
    ConfigError {
        /// Error message.
        message: String,
    },

    /// Compiler error.
    #[error(transparent)]
    CompilerError(#[from] eunomia_compiler::CompilerError),

    /// Core error.
    #[error(transparent)]
    CoreError(#[from] eunomia_core::Error),

    /// JSON error.
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    /// YAML error.
    #[error("YAML error: {0}")]
    YamlError(#[from] serde_yaml::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = TestError::ExecutionError {
            message: "OPA evaluation failed".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "Test execution failed: OPA evaluation failed"
        );
    }

    #[test]
    fn test_fixture_parse_error() {
        let err = TestError::FixtureParseError {
            message: "invalid JSON".to_string(),
        };
        assert!(err.to_string().contains("invalid JSON"));
    }
}
