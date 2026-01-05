//! Error types for the Eunomia compiler.

use std::path::PathBuf;
use thiserror::Error;

/// Result type alias for compiler operations.
pub type Result<T> = std::result::Result<T, CompilerError>;

/// Errors that can occur during policy compilation.
#[derive(Error, Debug)]
pub enum CompilerError {
    /// Failed to read a policy file.
    #[error("Failed to read policy file {path}: {source}")]
    FileReadError {
        /// Path to the file.
        path: PathBuf,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },

    /// Failed to parse Rego syntax.
    #[error("Rego parse error in {file} at line {line}: {message}")]
    ParseError {
        /// File being parsed.
        file: String,
        /// Line number of the error.
        line: usize,
        /// Error message.
        message: String,
    },

    /// Policy validation failed.
    #[error("Policy validation error: {message}")]
    ValidationError {
        /// Validation error message.
        message: String,
    },

    /// Missing required package declaration.
    #[error("Missing package declaration in {file}")]
    MissingPackage {
        /// File missing the package.
        file: String,
    },

    /// Bundle creation failed.
    #[error("Bundle creation error: {message}")]
    BundleError {
        /// Error message.
        message: String,
    },

    /// I/O error during directory operations.
    #[error("I/O error at {path}: {source}")]
    Io {
        /// Path where error occurred.
        path: PathBuf,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },

    /// Core library error.
    #[error(transparent)]
    CoreError(#[from] eunomia_core::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_error_display() {
        let err = CompilerError::ParseError {
            file: "authz.rego".to_string(),
            line: 10,
            message: "unexpected token".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "Rego parse error in authz.rego at line 10: unexpected token"
        );
    }

    #[test]
    fn test_missing_package_display() {
        let err = CompilerError::MissingPackage {
            file: "test.rego".to_string(),
        };
        assert_eq!(err.to_string(), "Missing package declaration in test.rego");
    }
}
