//! Rego policy parser.
//!
//! This module provides functionality for parsing Rego policy files.

use std::fs;
use std::path::Path;

use eunomia_core::Policy;
use tracing::debug;

use crate::error::{CompilerError, Result};

/// Parser for Rego policy files.
///
/// The parser reads Rego source files and produces [`Policy`] instances
/// with extracted metadata.
///
/// # Examples
///
/// ```rust,ignore
/// use eunomia_compiler::Parser;
///
/// let policy = Parser::parse_file("policies/authz.rego")?;
/// println!("Package: {}", policy.package_name);
/// ```
#[derive(Debug, Default)]
pub struct Parser {
    /// Whether to extract metadata from comments.
    extract_metadata: bool,
}

impl Parser {
    /// Creates a new parser with default settings.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            extract_metadata: true,
        }
    }

    /// Sets whether to extract metadata from METADATA comments.
    #[must_use]
    pub const fn with_metadata_extraction(mut self, extract: bool) -> Self {
        self.extract_metadata = extract;
        self
    }

    /// Parses a Rego policy from a file.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the Rego file
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or parsed.
    pub fn parse_file(&self, path: impl AsRef<Path>) -> Result<Policy> {
        let path = path.as_ref();
        debug!(?path, "Parsing policy file");

        let source = fs::read_to_string(path).map_err(|e| CompilerError::FileReadError {
            path: path.to_path_buf(),
            source: e,
        })?;

        let mut policy = self.parse_source(&source, path.to_string_lossy().as_ref())?;
        policy.file_path = Some(path.to_path_buf());

        Ok(policy)
    }

    /// Parses a Rego policy from source code.
    ///
    /// # Arguments
    ///
    /// * `source` - Rego source code
    /// * `file_name` - Name of the source (for error messages)
    ///
    /// # Errors
    ///
    /// Returns an error if the source cannot be parsed.
    pub fn parse_source(&self, source: &str, file_name: &str) -> Result<Policy> {
        let package_name = Self::extract_package(source, file_name)?;

        let mut policy = Policy::new(package_name, source);

        if self.extract_metadata {
            if let Some(description) = Self::extract_description(source) {
                policy = policy.with_description(description);
            }
            let authors = Self::extract_authors(source);
            if !authors.is_empty() {
                policy = policy.with_authors(authors);
            }
        }

        Ok(policy)
    }

    /// Extracts the package name from Rego source.
    fn extract_package(source: &str, file_name: &str) -> Result<String> {
        for (line_num, line) in source.lines().enumerate() {
            let trimmed = line.trim();

            // Skip comments and empty lines
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            // Look for package declaration
            if let Some(rest) = trimmed.strip_prefix("package") {
                let package_name = rest.trim().trim_end_matches(';');
                if package_name.is_empty() {
                    return Err(CompilerError::ParseError {
                        file: file_name.to_string(),
                        line: line_num + 1,
                        message: "Empty package name".to_string(),
                    });
                }
                return Ok(package_name.to_string());
            }

            // If we hit a non-comment, non-package line first, error
            return Err(CompilerError::MissingPackage {
                file: file_name.to_string(),
            });
        }

        Err(CompilerError::MissingPackage {
            file: file_name.to_string(),
        })
    }

    /// Extracts description from METADATA comment.
    fn extract_description(source: &str) -> Option<String> {
        let mut in_metadata = false;
        let mut found_description = false;

        for line in source.lines() {
            let trimmed = line.trim();

            if trimmed == "# METADATA" {
                in_metadata = true;
                continue;
            }

            if in_metadata {
                if !trimmed.starts_with('#') {
                    break;
                }

                let comment = trimmed.trim_start_matches('#').trim();

                if comment.starts_with("description:") {
                    let desc = comment.strip_prefix("description:").unwrap().trim();
                    return Some(desc.to_string());
                }

                // Multi-line description after "description:"
                if found_description && !comment.contains(':') {
                    continue;
                }

                if comment.starts_with("description:") {
                    found_description = true;
                }
            }
        }

        None
    }

    /// Extracts authors from METADATA comment.
    fn extract_authors(source: &str) -> Vec<String> {
        let mut in_metadata = false;
        let mut in_authors = false;
        let mut authors = Vec::new();

        for line in source.lines() {
            let trimmed = line.trim();

            if trimmed == "# METADATA" {
                in_metadata = true;
                continue;
            }

            if in_metadata {
                if !trimmed.starts_with('#') {
                    break;
                }

                let comment = trimmed.trim_start_matches('#').trim();

                if comment.starts_with("authors:") {
                    in_authors = true;
                    continue;
                }

                if in_authors {
                    if comment.starts_with('-') {
                        let author = comment.trim_start_matches('-').trim();
                        authors.push(author.to_string());
                    } else if comment.contains(':') {
                        // Next section started
                        break;
                    }
                }
            }
        }

        authors
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_policy() {
        let source = r#"
package users_service.authz

default allow := false
"#;

        let parser = Parser::new();
        let policy = parser.parse_source(source, "test.rego").unwrap();

        assert_eq!(policy.package_name, "users_service.authz");
        assert!(policy.source.contains("default allow := false"));
    }

    #[test]
    fn test_parse_policy_with_comments() {
        let source = r#"
# This is a comment
# Another comment
package test.policy

default allow := false
"#;

        let parser = Parser::new();
        let policy = parser.parse_source(source, "test.rego").unwrap();

        assert_eq!(policy.package_name, "test.policy");
    }

    #[test]
    fn test_parse_policy_with_metadata() {
        let source = r#"
# METADATA
# title: Test Policy
# description: A test authorization policy
# authors:
#   - team@example.com
#   - other@example.com
package test.authz

default allow := false
"#;

        let parser = Parser::new();
        let policy = parser.parse_source(source, "test.rego").unwrap();

        assert_eq!(policy.package_name, "test.authz");
        assert_eq!(policy.description, Some("A test authorization policy".to_string()));
        assert_eq!(policy.authors, vec!["team@example.com", "other@example.com"]);
    }

    #[test]
    fn test_parse_missing_package() {
        let source = r#"
default allow := false
"#;

        let parser = Parser::new();
        let result = parser.parse_source(source, "test.rego");

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, CompilerError::MissingPackage { .. }));
    }

    #[test]
    fn test_parse_empty_package() {
        let source = "package";

        let parser = Parser::new();
        let result = parser.parse_source(source, "test.rego");

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, CompilerError::ParseError { .. }));
    }

    #[test]
    fn test_parse_without_metadata_extraction() {
        let source = r#"
# METADATA
# description: Should be ignored
package test.authz
"#;

        let parser = Parser::new().with_metadata_extraction(false);
        let policy = parser.parse_source(source, "test.rego").unwrap();

        assert!(policy.description.is_none());
        assert!(policy.authors.is_empty());
    }
}
