//! Bundle compilation.
//!
//! This module provides functionality for compiling policies into distributable bundles.

use std::path::Path;

use eunomia_core::{Bundle, Policy};
use tracing::info;

use crate::analyzer::Analyzer;
use crate::error::{CompilerError, Result};
use crate::optimizer::Optimizer;
use crate::parser::Parser;

/// Compiles Rego policies into distributable bundles.
///
/// The bundler collects policies, validates them, and produces an OPA-compatible
/// bundle.
///
/// # Examples
///
/// ```rust
/// use eunomia_compiler::Bundler;
/// use eunomia_core::Policy;
///
/// let policy = Policy::new("users_service.authz", r#"
/// package users_service.authz
/// default allow := false
/// "#);
///
/// let bundle = Bundler::new("users-service")
///     .version("1.0.0")
///     .add_policy(policy)
///     .compile()
///     .unwrap();
/// ```
#[derive(Debug)]
pub struct Bundler {
    /// Bundle name.
    name: String,
    /// Bundle version.
    version: Option<String>,
    /// Git commit SHA.
    git_commit: Option<String>,
    /// Policies to include.
    policies: Vec<Policy>,
    /// Data files to include.
    data_files: Vec<(String, String)>,
    /// Whether to optimize policies.
    optimize: bool,
    /// Whether to validate policies.
    validate: bool,
}

impl Bundler {
    /// Creates a new bundler for the given service.
    ///
    /// # Arguments
    ///
    /// * `name` - Bundle name (typically the service name)
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: None,
            git_commit: None,
            policies: Vec::new(),
            data_files: Vec::new(),
            optimize: false,
            validate: true,
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
    pub fn add_policy(mut self, policy: Policy) -> Self {
        self.policies.push(policy);
        self
    }

    /// Adds a policy file to the bundle.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or parsed.
    pub fn add_policy_file(mut self, path: impl AsRef<Path>) -> Result<Self> {
        let parser = Parser::new();
        let policy = parser.parse_file(path)?;
        self.policies.push(policy);
        Ok(self)
    }

    /// Loads all policy files from a directory.
    ///
    /// Recursively scans for `.rego` files, excluding test files (`*_test.rego`).
    ///
    /// # Arguments
    ///
    /// * `dir` - Directory containing policy files
    ///
    /// # Errors
    ///
    /// Returns an error if the directory cannot be read or any policy fails to parse.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use eunomia_compiler::Bundler;
    ///
    /// let bundler = Bundler::new("users-service")
    ///     .version("1.0.0")
    ///     .add_policy_dir("policies/users-service")
    ///     .unwrap();
    /// ```
    pub fn add_policy_dir(mut self, dir: impl AsRef<Path>) -> Result<Self> {
        let dir = dir.as_ref();
        self.load_policies_recursive(dir)?;
        Ok(self)
    }

    /// Recursively loads all .rego files from a directory.
    fn load_policies_recursive(&mut self, dir: &Path) -> Result<()> {
        let entries = std::fs::read_dir(dir).map_err(|e| CompilerError::Io {
            path: dir.to_path_buf(),
            source: e,
        })?;

        let parser = Parser::new();

        for entry in entries {
            let entry = entry.map_err(|e| CompilerError::Io {
                path: dir.to_path_buf(),
                source: e,
            })?;

            let path = entry.path();

            if path.is_dir() {
                self.load_policies_recursive(&path)?;
            } else if let Some(ext) = path.extension() {
                if ext == "rego" {
                    // Skip test files
                    if let Some(stem) = path.file_stem() {
                        if stem.to_string_lossy().ends_with("_test") {
                            continue;
                        }
                    }
                    let policy = parser.parse_file(&path)?;
                    self.policies.push(policy);
                }
            }
        }

        Ok(())
    }

    /// Loads data files from a directory.
    ///
    /// Looks for `data.json` or `data.yaml` files.
    ///
    /// # Errors
    ///
    /// Returns an error if files cannot be read.
    pub fn add_data_dir(mut self, dir: impl AsRef<Path>) -> Result<Self> {
        let dir = dir.as_ref();
        self.load_data_recursive(dir)?;
        Ok(self)
    }

    /// Recursively loads data files from a directory.
    fn load_data_recursive(&mut self, dir: &Path) -> Result<()> {
        let entries = std::fs::read_dir(dir).map_err(|e| CompilerError::Io {
            path: dir.to_path_buf(),
            source: e,
        })?;

        for entry in entries {
            let entry = entry.map_err(|e| CompilerError::Io {
                path: dir.to_path_buf(),
                source: e,
            })?;

            let path = entry.path();

            if path.is_dir() {
                self.load_data_recursive(&path)?;
            } else if let Some(name) = path.file_name() {
                let name_str = name.to_string_lossy();
                if name_str == "data.json" || name_str == "data.yaml" {
                    let content = std::fs::read_to_string(&path).map_err(|e| CompilerError::Io {
                        path: path.clone(),
                        source: e,
                    })?;
                    // Use relative path from the policy root as the data path
                    let relative_path = path.strip_prefix(dir).unwrap_or(&path);
                    self.data_files.push((
                        relative_path.to_string_lossy().to_string(),
                        content,
                    ));
                }
            }
        }

        Ok(())
    }

    /// Adds a data file to the bundle.
    #[must_use]
    pub fn add_data_file(mut self, path: impl Into<String>, content: impl Into<String>) -> Self {
        self.data_files.push((path.into(), content.into()));
        self
    }

    /// Sets whether to optimize policies.
    #[must_use]
    pub const fn with_optimization(mut self, optimize: bool) -> Self {
        self.optimize = optimize;
        self
    }

    /// Sets whether to validate policies.
    #[must_use]
    pub const fn with_validation(mut self, validate: bool) -> Self {
        self.validate = validate;
        self
    }

    /// Compiles the policies into a bundle.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - No version is set
    /// - No policies are added
    /// - Policy validation fails
    pub fn compile(self) -> Result<Bundle> {
        let version = self.version.ok_or_else(|| CompilerError::BundleError {
            message: "Bundle version is required".to_string(),
        })?;

        if self.policies.is_empty() {
            return Err(CompilerError::BundleError {
                message: "At least one policy is required".to_string(),
            });
        }

        info!(
            bundle = %self.name,
            version = %version,
            policy_count = self.policies.len(),
            "Compiling bundle"
        );

        // Optionally validate policies
        let analyzer = Analyzer::new().with_warn_missing_tests(false);
        if self.validate {
            for policy in &self.policies {
                // Skip test policies during validation
                if !policy.is_test() {
                    analyzer.validate(policy)?;
                }
            }
        }

        // Optionally optimize policies
        let optimizer = Optimizer::new()
            .with_strip_comments(self.optimize)
            .with_minimize_whitespace(self.optimize);

        let policies: Vec<Policy> = if self.optimize {
            self.policies
                .iter()
                .map(|p| optimizer.optimize(p))
                .collect()
        } else {
            self.policies
        };

        // Build the bundle
        let mut builder = Bundle::builder(&self.name).version(&version);

        if let Some(commit) = self.git_commit {
            builder = builder.git_commit(commit);
        }

        // Add policies
        for policy in &policies {
            builder = builder.add_policy(&policy.package_name, &policy.source);
        }

        // Add data files
        for (path, content) in self.data_files {
            builder = builder.add_data_file(path, content);
        }

        // Add roots based on service name
        builder = builder.add_root(self.name.replace('-', "_"));

        Ok(builder.build())
    }

    /// Compiles and writes the bundle to a tar.gz file.
    ///
    /// This is a convenience method that combines `compile()` and `Bundle::write_to_file()`.
    ///
    /// # Arguments
    ///
    /// * `output_path` - Path to write the bundle file
    ///
    /// # Errors
    ///
    /// Returns an error if compilation fails or the file cannot be written.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use eunomia_compiler::Bundler;
    /// use eunomia_core::Policy;
    ///
    /// let policy = Policy::new(
    ///     "users_service.authz",
    ///     "package users_service.authz\ndefault allow := false",
    /// );
    ///
    /// Bundler::new("users-service")
    ///     .version("1.0.0")
    ///     .add_policy(policy)
    ///     .compile_to_file("users-service-v1.0.0.bundle.tar.gz")
    ///     .unwrap();
    /// ```
    pub fn compile_to_file(self, output_path: impl AsRef<Path>) -> Result<Bundle> {
        let bundle = self.compile()?;
        bundle.write_to_file(output_path).map_err(|e| CompilerError::BundleError {
            message: format!("failed to write bundle: {e}"),
        })?;
        Ok(bundle)
    }
}

#[cfg(test)]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bundler_basic() {
        let policy = Policy::new(
            "users_service.authz",
            "package users_service.authz\ndefault allow := false",
        );

        let bundle = Bundler::new("users-service")
            .version("1.0.0")
            .add_policy(policy)
            .compile()
            .unwrap();

        assert_eq!(bundle.name, "users-service");
        assert_eq!(bundle.version, "1.0.0");
        assert_eq!(bundle.policy_count(), 1);
        assert!(bundle.has_policy("users_service.authz"));
    }

    #[test]
    fn test_bundler_with_git_commit() {
        let policy = Policy::new("test.authz", "package test.authz\ndefault allow := false");

        let bundle = Bundler::new("test")
            .version("1.0.0")
            .git_commit("abc123def456")
            .add_policy(policy)
            .compile()
            .unwrap();

        assert_eq!(bundle.git_commit, Some("abc123def456".to_string()));
    }

    #[test]
    fn test_bundler_missing_version() {
        let policy = Policy::new("test.authz", "package test.authz\ndefault allow := false");

        let result = Bundler::new("test").add_policy(policy).compile();

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, CompilerError::BundleError { .. }));
    }

    #[test]
    fn test_bundler_no_policies() {
        let result = Bundler::new("test").version("1.0.0").compile();

        assert!(result.is_err());
    }

    #[test]
    fn test_bundler_validation_fails() {
        let policy = Policy::new(
            "test.authz",
            "package test.authz\nallow if { true }", // Missing default
        );

        let result = Bundler::new("test")
            .version("1.0.0")
            .add_policy(policy)
            .compile();

        assert!(result.is_err());
    }

    #[test]
    fn test_bundler_skip_validation() {
        let policy = Policy::new(
            "test.authz",
            "package test.authz\nallow if { true }", // Missing default
        );

        let result = Bundler::new("test")
            .version("1.0.0")
            .add_policy(policy)
            .with_validation(false)
            .compile();

        assert!(result.is_ok());
    }

    #[test]
    fn test_bundler_with_data_file() {
        let policy = Policy::new("test.authz", "package test.authz\ndefault allow := false");

        let bundle = Bundler::new("test")
            .version("1.0.0")
            .add_policy(policy)
            .add_data_file("data/roles.json", r#"{"admin": ["read", "write"]}"#)
            .compile()
            .unwrap();

        assert!(bundle.data_files.contains_key("data/roles.json"));
    }

    #[test]
    fn test_bundler_compile_to_file() {
        let dir = tempfile::tempdir().unwrap();
        let output_path = dir.path().join("test.bundle.tar.gz");

        let policy = Policy::new("test.authz", "package test.authz\ndefault allow := false");

        let bundle = Bundler::new("test-service")
            .version("1.0.0")
            .add_policy(policy)
            .compile_to_file(&output_path)
            .unwrap();

        assert!(output_path.exists());
        assert_eq!(bundle.name, "test-service");

        // Verify we can read it back
        let loaded = Bundle::from_file(&output_path).unwrap();
        assert_eq!(loaded.name, "test-service");
        assert_eq!(loaded.version, "1.0.0");
    }

    #[test]
    fn test_bundler_add_policy_dir() {
        let dir = tempfile::tempdir().unwrap();
        let policy_dir = dir.path().join("policies");
        std::fs::create_dir_all(&policy_dir).unwrap();

        // Create a policy file
        std::fs::write(
            policy_dir.join("authz.rego"),
            "package test.authz\ndefault allow := false",
        )
        .unwrap();

        // Create a test file (should be skipped)
        std::fs::write(
            policy_dir.join("authz_test.rego"),
            "package test.authz_test\ntest_foo { true }",
        )
        .unwrap();

        let bundle = Bundler::new("test")
            .version("1.0.0")
            .add_policy_dir(&policy_dir)
            .unwrap()
            .compile()
            .unwrap();

        // Should only include the non-test policy
        assert_eq!(bundle.policy_count(), 1);
        assert!(bundle.has_policy("test.authz"));
    }

    #[test]
    fn test_bundler_add_data_dir() {
        let dir = tempfile::tempdir().unwrap();
        let data_dir = dir.path().join("policies");
        std::fs::create_dir_all(&data_dir).unwrap();

        // Create a data file
        std::fs::write(
            data_dir.join("data.json"),
            r#"{"roles": ["admin", "user"]}"#,
        )
        .unwrap();

        let policy = Policy::new("test.authz", "package test.authz\ndefault allow := false");

        let bundle = Bundler::new("test")
            .version("1.0.0")
            .add_policy(policy)
            .add_data_dir(&data_dir)
            .unwrap()
            .compile()
            .unwrap();

        assert_eq!(bundle.data_files.len(), 1);
    }
}
