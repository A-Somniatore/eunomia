//! Version resolution for bundle references.
//!
//! Supports various version query formats:
//! - `latest` → Most recent semantic version
//! - `v1.2.3` → Exact version match
//! - `v1.2` → Latest patch in minor version
//! - `v1` → Latest minor/patch in major version
//! - `sha256:abc...` → Exact digest match

use crate::error::RegistryError;

/// A version query that can be resolved to a specific version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VersionQuery {
    /// Latest available version.
    Latest,

    /// Exact semantic version (e.g., "v1.2.3").
    Exact(String),

    /// Major version constraint (e.g., "v1" matches v1.x.y).
    Major(u64),

    /// Minor version constraint (e.g., "v1.2" matches v1.2.x).
    Minor(u64, u64),

    /// Content digest (e.g., "sha256:abc123...").
    Digest(String),
}

impl VersionQuery {
    /// Parses a version query string.
    ///
    /// # Examples
    ///
    /// ```
    /// use eunomia_registry::VersionQuery;
    ///
    /// let query = VersionQuery::parse("v1.2.3").unwrap();
    /// assert!(matches!(query, VersionQuery::Exact(_)));
    ///
    /// let query = VersionQuery::parse("latest").unwrap();
    /// assert!(matches!(query, VersionQuery::Latest));
    ///
    /// let query = VersionQuery::parse("v1").unwrap();
    /// assert!(matches!(query, VersionQuery::Major(1)));
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the version string cannot be parsed.
    pub fn parse(input: &str) -> Result<Self, RegistryError> {
        let input = input.trim();

        if input.eq_ignore_ascii_case("latest") {
            return Ok(Self::Latest);
        }

        // Check for digest reference
        if input.contains(':') && !input.starts_with('v') {
            return Ok(Self::Digest(input.to_string()));
        }

        // Remove optional 'v' prefix
        let version_str = input.strip_prefix('v').unwrap_or(input);

        // Parse version components
        let parts: Vec<&str> = version_str.split('.').collect();

        match parts.len() {
            1 => {
                let major =
                    parts[0]
                        .parse::<u64>()
                        .map_err(|_| RegistryError::InvalidReference {
                            reference: input.to_string(),
                        })?;
                Ok(Self::Major(major))
            }
            2 => {
                let major =
                    parts[0]
                        .parse::<u64>()
                        .map_err(|_| RegistryError::InvalidReference {
                            reference: input.to_string(),
                        })?;
                let minor =
                    parts[1]
                        .parse::<u64>()
                        .map_err(|_| RegistryError::InvalidReference {
                            reference: input.to_string(),
                        })?;
                Ok(Self::Minor(major, minor))
            }
            3 => Ok(Self::Exact(format!("v{version_str}"))),
            _ => Err(RegistryError::InvalidReference {
                reference: input.to_string(),
            }),
        }
    }

    /// Returns true if this query matches a digest.
    #[must_use]
    pub const fn is_digest(&self) -> bool {
        matches!(self, Self::Digest(_))
    }

    /// Returns true if this query matches latest.
    #[must_use]
    pub const fn is_latest(&self) -> bool {
        matches!(self, Self::Latest)
    }
}

impl std::fmt::Display for VersionQuery {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Latest => write!(f, "latest"),
            Self::Exact(v) => write!(f, "{v}"),
            Self::Major(m) => write!(f, "v{m}"),
            Self::Minor(m, n) => write!(f, "v{m}.{n}"),
            Self::Digest(d) => write!(f, "{d}"),
        }
    }
}

/// Resolves version queries to specific versions.
#[derive(Debug, Clone)]
pub struct VersionResolver;

impl VersionResolver {
    /// Creates a new version resolver.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Resolves a version query against a list of available tags.
    ///
    /// # Arguments
    ///
    /// * `query` - The version query to resolve.
    /// * `available_tags` - List of available version tags.
    ///
    /// # Returns
    ///
    /// The resolved version string, or an error if no match is found.
    ///
    /// # Examples
    ///
    /// ```
    /// use eunomia_registry::{VersionQuery, VersionResolver};
    ///
    /// let resolver = VersionResolver::new();
    /// let tags = vec!["v1.0.0", "v1.1.0", "v1.2.0", "v2.0.0"];
    ///
    /// // Exact match
    /// let query = VersionQuery::Exact("v1.1.0".to_string());
    /// let resolved = resolver.resolve(&query, &tags, "test").unwrap();
    /// assert_eq!(resolved, "v1.1.0");
    ///
    /// // Latest
    /// let query = VersionQuery::Latest;
    /// let resolved = resolver.resolve(&query, &tags, "test").unwrap();
    /// assert_eq!(resolved, "v2.0.0");
    ///
    /// // Major constraint
    /// let query = VersionQuery::Major(1);
    /// let resolved = resolver.resolve(&query, &tags, "test").unwrap();
    /// assert_eq!(resolved, "v1.2.0");
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the version query cannot be resolved.
    pub fn resolve(
        &self,
        query: &VersionQuery,
        available_tags: &[impl AsRef<str>],
        service: &str,
    ) -> Result<String, RegistryError> {
        match query {
            VersionQuery::Digest(d) => Ok(d.clone()),
            VersionQuery::Exact(v) => {
                if available_tags.iter().any(|t| t.as_ref() == v) {
                    Ok(v.clone())
                } else {
                    Err(RegistryError::NotFound {
                        service: service.to_string(),
                        version: v.clone(),
                    })
                }
            }
            VersionQuery::Latest => Self::find_latest(available_tags, service),
            VersionQuery::Major(major) => {
                Self::find_latest_in_major(*major, available_tags, service)
            }
            VersionQuery::Minor(major, minor) => {
                Self::find_latest_in_minor(*major, *minor, available_tags, service)
            }
        }
    }

    /// Finds the latest version from available tags.
    fn find_latest(tags: &[impl AsRef<str>], service: &str) -> Result<String, RegistryError> {
        let mut semver_tags: Vec<(u64, u64, u64, &str)> = tags
            .iter()
            .filter_map(|t| {
                let tag = t.as_ref();
                Self::parse_semver(tag).map(|(major, minor, patch)| (major, minor, patch, tag))
            })
            .collect();

        semver_tags.sort_by(|a, b| {
            a.0.cmp(&b.0)
                .then_with(|| a.1.cmp(&b.1))
                .then_with(|| a.2.cmp(&b.2))
                .reverse()
        });

        semver_tags
            .first()
            .map(|(_, _, _, tag)| (*tag).to_string())
            .ok_or_else(|| RegistryError::VersionResolutionFailed {
                service: service.to_string(),
                query: "latest".to_string(),
                message: "No valid semantic versions found".to_string(),
            })
    }

    /// Finds the latest version within a major version.
    fn find_latest_in_major(
        major: u64,
        tags: &[impl AsRef<str>],
        service: &str,
    ) -> Result<String, RegistryError> {
        let mut matching: Vec<(u64, u64, &str)> = tags
            .iter()
            .filter_map(|t| {
                let tag = t.as_ref();
                let (m, minor, patch) = Self::parse_semver(tag)?;
                if m == major {
                    Some((minor, patch, tag))
                } else {
                    None
                }
            })
            .collect();

        matching.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)).reverse());

        matching
            .first()
            .map(|(_, _, tag)| (*tag).to_string())
            .ok_or_else(|| RegistryError::VersionResolutionFailed {
                service: service.to_string(),
                query: format!("v{major}"),
                message: format!("No versions found for major version {major}"),
            })
    }

    /// Finds the latest version within a minor version.
    fn find_latest_in_minor(
        major: u64,
        minor: u64,
        tags: &[impl AsRef<str>],
        service: &str,
    ) -> Result<String, RegistryError> {
        let mut matching: Vec<(u64, &str)> = tags
            .iter()
            .filter_map(|t| {
                let tag = t.as_ref();
                let (m, n, patch) = Self::parse_semver(tag)?;
                if m == major && n == minor {
                    Some((patch, tag))
                } else {
                    None
                }
            })
            .collect();

        matching.sort_by(|a, b| a.0.cmp(&b.0).reverse());

        matching
            .first()
            .map(|(_, tag)| (*tag).to_string())
            .ok_or_else(|| RegistryError::VersionResolutionFailed {
                service: service.to_string(),
                query: format!("v{major}.{minor}"),
                message: format!("No versions found for v{major}.{minor}.x"),
            })
    }

    /// Parses a semantic version string into (major, minor, patch).
    fn parse_semver(tag: &str) -> Option<(u64, u64, u64)> {
        let version = tag.strip_prefix('v').unwrap_or(tag);
        let parts: Vec<&str> = version.split('.').collect();

        if parts.len() != 3 {
            return None;
        }

        let major = parts[0].parse().ok()?;
        let minor = parts[1].parse().ok()?;
        let patch = parts[2].split('-').next()?.parse().ok()?;

        Some((major, minor, patch))
    }
}

impl Default for VersionResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_latest() {
        let query = VersionQuery::parse("latest").unwrap();
        assert!(matches!(query, VersionQuery::Latest));

        let query = VersionQuery::parse("LATEST").unwrap();
        assert!(matches!(query, VersionQuery::Latest));
    }

    #[test]
    fn test_parse_exact() {
        let query = VersionQuery::parse("v1.2.3").unwrap();
        assert!(matches!(query, VersionQuery::Exact(v) if v == "v1.2.3"));

        let query = VersionQuery::parse("1.2.3").unwrap();
        assert!(matches!(query, VersionQuery::Exact(v) if v == "v1.2.3"));
    }

    #[test]
    fn test_parse_major() {
        let query = VersionQuery::parse("v1").unwrap();
        assert!(matches!(query, VersionQuery::Major(1)));

        let query = VersionQuery::parse("2").unwrap();
        assert!(matches!(query, VersionQuery::Major(2)));
    }

    #[test]
    fn test_parse_minor() {
        let query = VersionQuery::parse("v1.2").unwrap();
        assert!(matches!(query, VersionQuery::Minor(1, 2)));
    }

    #[test]
    fn test_parse_digest() {
        let query = VersionQuery::parse("sha256:abc123def456").unwrap();
        assert!(matches!(query, VersionQuery::Digest(d) if d == "sha256:abc123def456"));
    }

    #[test]
    fn test_resolve_exact() {
        let resolver = VersionResolver::new();
        let tags = vec!["v1.0.0", "v1.1.0", "v1.2.0"];
        let query = VersionQuery::Exact("v1.1.0".to_string());

        let result = resolver.resolve(&query, &tags, "test").unwrap();
        assert_eq!(result, "v1.1.0");
    }

    #[test]
    fn test_resolve_exact_not_found() {
        let resolver = VersionResolver::new();
        let tags = vec!["v1.0.0", "v1.1.0"];
        let query = VersionQuery::Exact("v2.0.0".to_string());

        let result = resolver.resolve(&query, &tags, "test");
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_latest() {
        let resolver = VersionResolver::new();
        let tags = vec!["v1.0.0", "v2.1.0", "v1.5.0", "v2.0.0"];
        let query = VersionQuery::Latest;

        let result = resolver.resolve(&query, &tags, "test").unwrap();
        assert_eq!(result, "v2.1.0");
    }

    #[test]
    fn test_resolve_major() {
        let resolver = VersionResolver::new();
        let tags = vec!["v1.0.0", "v1.2.0", "v1.1.5", "v2.0.0"];
        let query = VersionQuery::Major(1);

        let result = resolver.resolve(&query, &tags, "test").unwrap();
        assert_eq!(result, "v1.2.0");
    }

    #[test]
    fn test_resolve_minor() {
        let resolver = VersionResolver::new();
        let tags = vec!["v1.2.0", "v1.2.1", "v1.2.5", "v1.3.0"];
        let query = VersionQuery::Minor(1, 2);

        let result = resolver.resolve(&query, &tags, "test").unwrap();
        assert_eq!(result, "v1.2.5");
    }

    #[test]
    fn test_resolve_digest() {
        let resolver = VersionResolver::new();
        let tags: Vec<&str> = vec![];
        let query = VersionQuery::Digest("sha256:abc123".to_string());

        let result = resolver.resolve(&query, &tags, "test").unwrap();
        assert_eq!(result, "sha256:abc123");
    }

    #[test]
    fn test_version_query_display() {
        assert_eq!(VersionQuery::Latest.to_string(), "latest");
        assert_eq!(
            VersionQuery::Exact("v1.2.3".to_string()).to_string(),
            "v1.2.3"
        );
        assert_eq!(VersionQuery::Major(1).to_string(), "v1");
        assert_eq!(VersionQuery::Minor(1, 2).to_string(), "v1.2");
        assert_eq!(
            VersionQuery::Digest("sha256:abc".to_string()).to_string(),
            "sha256:abc"
        );
    }
}
