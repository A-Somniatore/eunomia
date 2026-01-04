//! Policy optimization.
//!
//! This module provides optimization passes for compiled policies.

use eunomia_core::Policy;

/// Optimizer for Rego policies.
///
/// The optimizer applies transformations to improve policy evaluation performance.
#[derive(Debug, Default)]
pub struct Optimizer {
    /// Whether to remove comments.
    strip_comments: bool,
    /// Whether to minimize whitespace.
    minimize_whitespace: bool,
}

impl Optimizer {
    /// Creates a new optimizer with default settings.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            strip_comments: false,
            minimize_whitespace: false,
        }
    }

    /// Sets whether to strip comments from the source.
    #[must_use]
    pub const fn with_strip_comments(mut self, strip: bool) -> Self {
        self.strip_comments = strip;
        self
    }

    /// Sets whether to minimize whitespace.
    #[must_use]
    pub const fn with_minimize_whitespace(mut self, minimize: bool) -> Self {
        self.minimize_whitespace = minimize;
        self
    }

    /// Optimizes a policy.
    ///
    /// # Arguments
    ///
    /// * `policy` - The policy to optimize
    ///
    /// # Returns
    ///
    /// A new policy with optimizations applied.
    #[must_use]
    pub fn optimize(&self, policy: &Policy) -> Policy {
        let mut source = policy.source.clone();

        if self.strip_comments {
            source = Self::strip_comments_from_source(&source);
        }

        if self.minimize_whitespace {
            source = Self::minimize_whitespace_in_source(&source);
        }

        Policy {
            package_name: policy.package_name.clone(),
            source,
            file_path: policy.file_path.clone(),
            created_at: policy.created_at,
            description: policy.description.clone(),
            authors: policy.authors.clone(),
        }
    }

    /// Removes comments from source code.
    fn strip_comments_from_source(source: &str) -> String {
        let mut result = Vec::new();
        let mut in_metadata = false;

        for line in source.lines() {
            let trimmed = line.trim();

            // Keep METADATA comments as they may be required
            if trimmed == "# METADATA" {
                in_metadata = true;
                result.push(line.to_string());
                continue;
            }

            if in_metadata {
                if trimmed.starts_with('#') {
                    result.push(line.to_string());
                    continue;
                }
                in_metadata = false;
            }

            // Skip pure comment lines
            if trimmed.starts_with('#') {
                continue;
            }

            // Remove inline comments
            if let Some(idx) = line.find(" #") {
                result.push(line[..idx].to_string());
            } else {
                result.push(line.to_string());
            }
        }

        result.join("\n")
    }

    /// Minimizes whitespace in source code.
    fn minimize_whitespace_in_source(source: &str) -> String {
        let mut result = Vec::new();
        let mut prev_empty = false;

        for line in source.lines() {
            let trimmed = line.trim();

            // Skip multiple consecutive empty lines
            if trimmed.is_empty() {
                if !prev_empty {
                    result.push(String::new());
                }
                prev_empty = true;
                continue;
            }

            prev_empty = false;
            result.push(trimmed.to_string());
        }

        result.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_optimize_strip_comments() {
        let policy = Policy::new(
            "test.authz",
            r#"
# This is a comment
package test.authz

# Another comment
default allow := false  # inline comment
"#,
        );

        let optimizer = Optimizer::new().with_strip_comments(true);
        let optimized = optimizer.optimize(&policy);

        assert!(!optimized.source.contains("This is a comment"));
        assert!(!optimized.source.contains("Another comment"));
        assert!(!optimized.source.contains("inline comment"));
        assert!(optimized.source.contains("default allow := false"));
    }

    #[test]
    fn test_optimize_preserve_metadata() {
        let policy = Policy::new(
            "test.authz",
            r#"
# METADATA
# description: Important policy
package test.authz
"#,
        );

        let optimizer = Optimizer::new().with_strip_comments(true);
        let optimized = optimizer.optimize(&policy);

        assert!(optimized.source.contains("# METADATA"));
        assert!(optimized.source.contains("# description: Important policy"));
    }

    #[test]
    fn test_optimize_minimize_whitespace() {
        let policy = Policy::new(
            "test.authz",
            r#"
package test.authz



default allow := false


allow if {
    true
}
"#,
        );

        let optimizer = Optimizer::new().with_minimize_whitespace(true);
        let optimized = optimizer.optimize(&policy);

        // Should not have multiple consecutive empty lines
        assert!(!optimized.source.contains("\n\n\n"));
    }

    #[test]
    fn test_optimize_noop() {
        let policy = Policy::new("test.authz", "package test.authz\ndefault allow := false");

        let optimizer = Optimizer::new();
        let optimized = optimizer.optimize(&policy);

        assert_eq!(policy.source, optimized.source);
    }
}
