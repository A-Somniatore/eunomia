//! Test discovery for policy tests.
//!
//! This module provides functionality to discover test cases from:
//! - `*_test.rego` files containing native Rego test rules
//! - Fixture files in JSON/YAML format
//!
//! # Discovery Process
//!
//! 1. Recursively scan a directory for `*_test.rego` files
//! 2. Parse each file to extract test rules (functions starting with `test_`)
//! 3. Also load the corresponding policy files (e.g., `authz.rego` for `authz_test.rego`)
//! 4. Build a test suite ready for execution
//!
//! # Example
//!
//! ```rust,ignore
//! use eunomia_test::discovery::TestDiscovery;
//!
//! let discovery = TestDiscovery::new();
//! let suite = discovery.discover("policies/")?;
//!
//! for test in suite.tests() {
//!     println!("Found test: {}", test.qualified_name);
//! }
//! ```

use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use tracing::{debug, info};

use crate::error::{Result, TestError};

/// Configuration for test discovery.
#[derive(Debug, Clone)]
pub struct DiscoveryConfig {
    /// Pattern for test files.
    pub test_file_pattern: String,
    /// Whether to search recursively.
    pub recursive: bool,
    /// Whether to include fixture files.
    pub include_fixtures: bool,
    /// Directories to exclude from discovery.
    pub exclude_dirs: Vec<String>,
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            test_file_pattern: "_test.rego".to_string(),
            recursive: true,
            include_fixtures: true,
            exclude_dirs: vec![
                ".git".to_string(),
                "node_modules".to_string(),
                "target".to_string(),
            ],
        }
    }
}

impl DiscoveryConfig {
    /// Creates a new discovery configuration.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets whether to search recursively.
    #[must_use]
    pub const fn with_recursive(mut self, recursive: bool) -> Self {
        self.recursive = recursive;
        self
    }

    /// Sets whether to include fixtures.
    #[must_use]
    pub const fn with_fixtures(mut self, include_fixtures: bool) -> Self {
        self.include_fixtures = include_fixtures;
        self
    }

    /// Adds a directory to exclude.
    #[must_use]
    pub fn exclude_dir(mut self, dir: impl Into<String>) -> Self {
        self.exclude_dirs.push(dir.into());
        self
    }
}

/// A discovered test case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredTest {
    /// Source file containing the test.
    pub file: PathBuf,
    /// Package name of the test.
    pub package: String,
    /// Rule name (e.g., `test_admin_allowed`).
    pub name: String,
    /// Fully qualified name for evaluation (e.g., `data.authz_test.test_admin_allowed`).
    pub qualified_name: String,
    /// Associated policy file (if found).
    pub policy_file: Option<PathBuf>,
    /// Test description (from comments).
    pub description: Option<String>,
}

/// A discovered fixture file.
#[derive(Debug, Clone)]
pub struct DiscoveredFixture {
    /// Path to the fixture file.
    pub file: PathBuf,
    /// Format of the fixture (json, yaml).
    pub format: FixtureFormat,
    /// Associated test file (if any).
    pub test_file: Option<PathBuf>,
}

/// Format of a fixture file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixtureFormat {
    /// JSON format.
    Json,
    /// YAML format.
    Yaml,
}

/// A discovered test suite.
#[derive(Debug, Default)]
pub struct TestSuite {
    /// Discovered test cases.
    tests: Vec<DiscoveredTest>,
    /// Discovered fixture files.
    fixtures: Vec<DiscoveredFixture>,
    /// Policy files required by tests.
    policy_files: HashMap<PathBuf, String>,
    /// Root directory of the discovery.
    root: PathBuf,
}

impl TestSuite {
    /// Creates a new empty test suite.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            tests: Vec::new(),
            fixtures: Vec::new(),
            policy_files: HashMap::new(),
            root: root.into(),
        }
    }

    /// Returns the discovered tests.
    #[must_use]
    pub fn tests(&self) -> &[DiscoveredTest] {
        &self.tests
    }

    /// Returns the discovered fixtures.
    #[must_use]
    pub fn fixtures(&self) -> &[DiscoveredFixture] {
        &self.fixtures
    }

    /// Returns the policy files needed by tests.
    #[must_use]
    pub const fn policy_files(&self) -> &HashMap<PathBuf, String> {
        &self.policy_files
    }

    /// Returns the number of tests.
    #[must_use]
    pub const fn test_count(&self) -> usize {
        self.tests.len()
    }

    /// Returns the root directory.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Adds a test to the suite.
    pub fn add_test(&mut self, test: DiscoveredTest) {
        self.tests.push(test);
    }

    /// Adds a fixture to the suite.
    pub fn add_fixture(&mut self, fixture: DiscoveredFixture) {
        self.fixtures.push(fixture);
    }

    /// Adds a policy file to the suite.
    pub fn add_policy_file(&mut self, path: PathBuf, content: String) {
        self.policy_files.insert(path, content);
    }

    /// Returns tests grouped by file.
    #[must_use]
    pub fn tests_by_file(&self) -> HashMap<PathBuf, Vec<&DiscoveredTest>> {
        let mut by_file: HashMap<PathBuf, Vec<&DiscoveredTest>> = HashMap::new();
        for test in &self.tests {
            by_file.entry(test.file.clone()).or_default().push(test);
        }
        by_file
    }

    /// Returns tests grouped by package.
    #[must_use]
    pub fn tests_by_package(&self) -> HashMap<String, Vec<&DiscoveredTest>> {
        let mut by_package: HashMap<String, Vec<&DiscoveredTest>> = HashMap::new();
        for test in &self.tests {
            by_package
                .entry(test.package.clone())
                .or_default()
                .push(test);
        }
        by_package
    }
}

/// Test discovery engine.
#[derive(Debug)]
pub struct TestDiscovery {
    config: DiscoveryConfig,
}

impl TestDiscovery {
    /// Creates a new test discovery instance.
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: DiscoveryConfig::default(),
        }
    }

    /// Creates a test discovery instance with custom configuration.
    #[must_use]
    pub const fn with_config(config: DiscoveryConfig) -> Self {
        Self { config }
    }

    /// Discovers tests in a directory.
    ///
    /// # Arguments
    ///
    /// * `path` - Root directory to search for tests
    ///
    /// # Errors
    ///
    /// Returns an error if the directory cannot be read or parsed.
    pub fn discover(&self, path: impl AsRef<Path>) -> Result<TestSuite> {
        let root = path.as_ref();

        if !root.exists() {
            return Err(TestError::Discovery(format!(
                "Directory does not exist: {}",
                root.display()
            )));
        }

        if !root.is_dir() {
            return Err(TestError::Discovery(format!(
                "Path is not a directory: {}",
                root.display()
            )));
        }

        info!(path = %root.display(), "Starting test discovery");

        let mut suite = TestSuite::new(root);
        self.scan_directory(root, &mut suite)?;

        info!(
            tests = suite.test_count(),
            fixtures = suite.fixtures.len(),
            "Discovery complete"
        );

        Ok(suite)
    }

    /// Recursively scans a directory for test files.
    fn scan_directory(&self, dir: &Path, suite: &mut TestSuite) -> Result<()> {
        let entries = fs::read_dir(dir).map_err(|e| TestError::Io {
            path: dir.to_path_buf(),
            source: e,
        })?;

        for entry in entries {
            let entry = entry.map_err(|e| TestError::Io {
                path: dir.to_path_buf(),
                source: e,
            })?;

            let path = entry.path();

            if path.is_dir() {
                let dir_name = path.file_name().and_then(OsStr::to_str).unwrap_or("");

                // Skip excluded directories
                if self.config.exclude_dirs.contains(&dir_name.to_string()) {
                    debug!(dir = %path.display(), "Skipping excluded directory");
                    continue;
                }

                // Recurse if configured
                if self.config.recursive {
                    self.scan_directory(&path, suite)?;
                }
            } else if path.is_file() {
                self.process_file(&path, suite)?;
            }
        }

        Ok(())
    }

    /// Processes a single file.
    fn process_file(&self, path: &Path, suite: &mut TestSuite) -> Result<()> {
        let file_name = path.file_name().and_then(OsStr::to_str).unwrap_or("");
        let extension = path.extension().and_then(OsStr::to_str).unwrap_or("");

        // Check for test files
        if file_name.ends_with(&self.config.test_file_pattern) {
            Self::process_test_file(path, suite)?;
        }
        // Check for fixture files
        else if self.config.include_fixtures {
            let is_json = extension.eq_ignore_ascii_case("json");
            let is_yaml =
                extension.eq_ignore_ascii_case("yaml") || extension.eq_ignore_ascii_case("yml");

            if is_json && file_name.contains("fixture") {
                Self::process_fixture_file(path, FixtureFormat::Json, suite);
            } else if is_yaml && file_name.contains("fixture") {
                Self::process_fixture_file(path, FixtureFormat::Yaml, suite);
            }
        }
        // Check for policy files (to load for testing)
        else if extension.eq_ignore_ascii_case("rego") && !file_name.ends_with("_test.rego") {
            Self::process_policy_file(path, suite)?;
        }

        Ok(())
    }

    /// Processes a test file.
    fn process_test_file(path: &Path, suite: &mut TestSuite) -> Result<()> {
        debug!(file = %path.display(), "Processing test file");

        let source = fs::read_to_string(path).map_err(|e| TestError::Io {
            path: path.to_path_buf(),
            source: e,
        })?;

        // Extract package name
        let package = extract_package(&source).ok_or_else(|| {
            TestError::Parse(format!(
                "Missing package declaration in {}",
                path.display()
            ))
        })?;

        // Find corresponding policy file
        let policy_file = find_policy_file(path);

        // Load the policy file if it exists
        if let Some(ref policy_path) = policy_file {
            if policy_path.exists() {
                let policy_source = fs::read_to_string(policy_path).map_err(|e| TestError::Io {
                    path: policy_path.clone(),
                    source: e,
                })?;
                suite.add_policy_file(policy_path.clone(), policy_source);
            }
        }

        // Add test file itself
        suite.add_policy_file(path.to_path_buf(), source.clone());

        // Extract test rules
        let tests = extract_test_rules(&source, path, &package, policy_file.as_ref());
        for test in tests {
            suite.add_test(test);
        }

        Ok(())
    }

    /// Processes a fixture file.
    fn process_fixture_file(path: &Path, format: FixtureFormat, suite: &mut TestSuite) {
        debug!(file = %path.display(), "Processing fixture file");

        // Find associated test file
        let test_file = find_test_file_for_fixture(path);

        suite.add_fixture(DiscoveredFixture {
            file: path.to_path_buf(),
            format,
            test_file,
        });
    }

    /// Processes a policy file for loading.
    fn process_policy_file(path: &Path, suite: &mut TestSuite) -> Result<()> {
        debug!(file = %path.display(), "Processing policy file");

        let source = fs::read_to_string(path).map_err(|e| TestError::Io {
            path: path.to_path_buf(),
            source: e,
        })?;

        suite.add_policy_file(path.to_path_buf(), source);

        Ok(())
    }
}

impl Default for TestDiscovery {
    fn default() -> Self {
        Self::new()
    }
}

/// Extracts the package name from source.
fn extract_package(source: &str) -> Option<String> {
    for line in source.lines() {
        let trimmed = line.trim();

        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("package") {
            let package = rest.trim().trim_end_matches(';');
            if !package.is_empty() {
                return Some(package.to_string());
            }
        }

        // Stop at first non-comment, non-empty line that's not a package
        break;
    }

    None
}

/// Extracts test rules from source.
fn extract_test_rules(
    source: &str,
    file: &Path,
    package: &str,
    policy_file: Option<&PathBuf>,
) -> Vec<DiscoveredTest> {
    let mut tests = Vec::new();
    let mut current_description: Option<String> = None;

    for line in source.lines() {
        let trimmed = line.trim();

        // Capture comments for test descriptions
        if trimmed.starts_with('#') {
            // Look for description comments like: # Test that admin users can access
            let comment = trimmed.trim_start_matches('#').trim();
            if !comment.is_empty()
                && !comment.starts_with("METADATA")
                && !comment.contains(':')
            {
                current_description = Some(comment.to_string());
            }
            continue;
        }

        // Look for test rules
        if let Some(rule_name) = extract_test_rule_name(trimmed) {
            tests.push(DiscoveredTest {
                file: file.to_path_buf(),
                package: package.to_string(),
                name: rule_name.clone(),
                qualified_name: format!("data.{package}.{rule_name}"),
                policy_file: policy_file.cloned(),
                description: current_description.take(),
            });
        } else if !trimmed.is_empty() {
            // Reset description if we hit a non-test line
            current_description = None;
        }
    }

    tests
}

/// Extracts a test rule name from a line.
fn extract_test_rule_name(line: &str) -> Option<String> {
    // Skip imports, package declarations, etc.
    if line.starts_with("import")
        || line.starts_with("package")
        || line.starts_with("default")
    {
        return None;
    }

    // Find the rule name before any operator
    let name_end = line
        .find(":=")
        .or_else(|| line.find(" = "))
        .or_else(|| line.find(" if"))
        .or_else(|| line.find('{'))?;

    let name = line[..name_end].trim();

    // Only return if it's a test rule
    if name.starts_with("test_") {
        Some(name.to_string())
    } else {
        None
    }
}

/// Finds the corresponding policy file for a test file.
fn find_policy_file(test_file: &Path) -> Option<PathBuf> {
    let file_name = test_file.file_name()?.to_str()?;

    // authz_test.rego -> authz.rego
    if let Some(base) = file_name.strip_suffix("_test.rego") {
        let policy_name = format!("{base}.rego");
        let policy_path = test_file.parent()?.join(&policy_name);

        if policy_path.exists() {
            return Some(policy_path);
        }
    }

    None
}

/// Finds the test file associated with a fixture.
fn find_test_file_for_fixture(fixture: &Path) -> Option<PathBuf> {
    let parent = fixture.parent()?;
    // TODO: Use fixture stem to find more specific test file
    let _file_name = fixture.file_stem()?.to_str()?;

    // fixtures_admin.json -> *_test.rego in same directory
    // Or look for any _test.rego file
    for entry in fs::read_dir(parent).ok()? {
        let entry = entry.ok()?;
        let path = entry.path();
        if let Some(name) = path.file_name().and_then(OsStr::to_str) {
            if name.ends_with("_test.rego") {
                return Some(path);
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_policy(dir: &Path, name: &str, content: &str) -> PathBuf {
        let path = dir.join(name);
        fs::write(&path, content).unwrap();
        path
    }

    #[test]
    fn test_extract_package() {
        let source = r#"
            # Comment
            package authz_test

            import future.keywords
        "#;
        assert_eq!(extract_package(source), Some("authz_test".to_string()));
    }

    #[test]
    fn test_extract_package_no_package() {
        let source = r#"
            # Just comments
            # No package
        "#;
        assert_eq!(extract_package(source), None);
    }

    #[test]
    fn test_extract_test_rule_name() {
        assert_eq!(
            extract_test_rule_name("test_admin_allowed if {"),
            Some("test_admin_allowed".to_string())
        );
        assert_eq!(
            extract_test_rule_name("test_basic := true"),
            Some("test_basic".to_string())
        );
        assert_eq!(extract_test_rule_name("allow if {"), None);
        assert_eq!(extract_test_rule_name("import future.keywords"), None);
    }

    #[test]
    fn test_discovery_config_defaults() {
        let config = DiscoveryConfig::default();
        assert!(config.recursive);
        assert!(config.include_fixtures);
        assert!(config.exclude_dirs.contains(&".git".to_string()));
    }

    #[test]
    fn test_discovery_config_builder() {
        let config = DiscoveryConfig::new()
            .with_recursive(false)
            .with_fixtures(false)
            .exclude_dir("vendor");

        assert!(!config.recursive);
        assert!(!config.include_fixtures);
        assert!(config.exclude_dirs.contains(&"vendor".to_string()));
    }

    #[test]
    fn test_test_suite_operations() {
        let mut suite = TestSuite::new("/test");

        suite.add_test(DiscoveredTest {
            file: PathBuf::from("test.rego"),
            package: "test".to_string(),
            name: "test_one".to_string(),
            qualified_name: "data.test.test_one".to_string(),
            policy_file: None,
            description: None,
        });

        assert_eq!(suite.test_count(), 1);
        assert_eq!(suite.tests()[0].name, "test_one");
    }

    #[test]
    fn test_discovery_nonexistent_dir() {
        let discovery = TestDiscovery::new();
        let result = discovery.discover("/nonexistent/path");
        assert!(result.is_err());
    }

    #[test]
    fn test_discovery_basic() {
        let temp_dir = TempDir::new().unwrap();

        // Create a test file
        let test_content = r#"
package authz_test

import future.keywords.if

# Test admin access
test_admin_allowed if {
    true
}

# Test guest denied
test_guest_denied if {
    false
}
"#;
        create_test_policy(temp_dir.path(), "authz_test.rego", test_content);

        let discovery = TestDiscovery::new();
        let suite = discovery.discover(temp_dir.path()).unwrap();

        assert_eq!(suite.test_count(), 2);

        let names: Vec<_> = suite.tests().iter().map(|t| &t.name).collect();
        assert!(names.contains(&&"test_admin_allowed".to_string()));
        assert!(names.contains(&&"test_guest_denied".to_string()));
    }

    #[test]
    fn test_discovery_with_policy_file() {
        let temp_dir = TempDir::new().unwrap();

        // Create policy file
        let policy_content = r#"
package authz

default allow := false

allow if {
    input.caller.type == "admin"
}
"#;
        create_test_policy(temp_dir.path(), "authz.rego", policy_content);

        // Create test file
        let test_content = r#"
package authz_test

import data.authz

test_admin if {
    authz.allow with input as {"caller": {"type": "admin"}}
}
"#;
        create_test_policy(temp_dir.path(), "authz_test.rego", test_content);

        let discovery = TestDiscovery::new();
        let suite = discovery.discover(temp_dir.path()).unwrap();

        assert_eq!(suite.test_count(), 1);

        // Should have loaded both files
        assert!(suite.policy_files().len() >= 2);
    }

    #[test]
    fn test_tests_by_package() {
        let mut suite = TestSuite::new("/test");

        suite.add_test(DiscoveredTest {
            file: PathBuf::from("a_test.rego"),
            package: "pkg_a".to_string(),
            name: "test_one".to_string(),
            qualified_name: "data.pkg_a.test_one".to_string(),
            policy_file: None,
            description: None,
        });

        suite.add_test(DiscoveredTest {
            file: PathBuf::from("b_test.rego"),
            package: "pkg_b".to_string(),
            name: "test_two".to_string(),
            qualified_name: "data.pkg_b.test_two".to_string(),
            policy_file: None,
            description: None,
        });

        let by_package = suite.tests_by_package();
        assert_eq!(by_package.len(), 2);
        assert!(by_package.contains_key("pkg_a"));
        assert!(by_package.contains_key("pkg_b"));
    }
}
