//! Configuration types for registry client.

use std::path::PathBuf;
use std::time::Duration;

/// Configuration for the registry client.
#[derive(Debug, Clone)]
pub struct RegistryConfig {
    /// Registry URL (e.g., "<https://registry.example.com>").
    pub url: String,

    /// Namespace prefix for bundle repositories (e.g., "policies").
    pub namespace: String,

    /// Authentication configuration.
    pub auth: RegistryAuth,

    /// Request timeout.
    pub timeout: Duration,

    /// TLS configuration for mTLS.
    pub tls: Option<TlsConfig>,

    /// User agent string.
    pub user_agent: String,
}

impl RegistryConfig {
    /// Creates a new registry configuration with the given URL.
    ///
    /// # Examples
    ///
    /// ```
    /// use eunomia_registry::RegistryConfig;
    ///
    /// let config = RegistryConfig::new("https://registry.example.com");
    /// assert_eq!(config.url, "https://registry.example.com");
    /// ```
    #[must_use]
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            namespace: String::new(),
            auth: RegistryAuth::None,
            timeout: Duration::from_secs(30),
            tls: None,
            user_agent: format!("eunomia-registry/{}", env!("CARGO_PKG_VERSION")),
        }
    }

    /// Sets the namespace prefix.
    ///
    /// # Examples
    ///
    /// ```
    /// use eunomia_registry::RegistryConfig;
    ///
    /// let config = RegistryConfig::new("https://registry.example.com")
    ///     .with_namespace("policies");
    /// assert_eq!(config.namespace, "policies");
    /// ```
    #[must_use]
    pub fn with_namespace(mut self, namespace: impl Into<String>) -> Self {
        self.namespace = namespace.into();
        self
    }

    /// Sets the authentication method.
    #[must_use]
    pub fn with_auth(mut self, auth: RegistryAuth) -> Self {
        self.auth = auth;
        self
    }

    /// Sets the request timeout.
    #[must_use]
    pub const fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Sets the TLS configuration.
    #[must_use]
    pub fn with_tls(mut self, tls: TlsConfig) -> Self {
        self.tls = Some(tls);
        self
    }

    /// Returns the full repository name for a service.
    ///
    /// # Examples
    ///
    /// ```
    /// use eunomia_registry::RegistryConfig;
    ///
    /// let config = RegistryConfig::new("https://registry.example.com")
    ///     .with_namespace("policies");
    /// assert_eq!(config.repository_name("users-service"), "policies/users-service");
    ///
    /// let config_no_ns = RegistryConfig::new("https://registry.example.com");
    /// assert_eq!(config_no_ns.repository_name("users-service"), "users-service");
    /// ```
    #[must_use]
    pub fn repository_name(&self, service: &str) -> String {
        if self.namespace.is_empty() {
            service.to_string()
        } else {
            format!("{}/{}", self.namespace, service)
        }
    }
}

/// Authentication methods for registry access.
#[derive(Debug, Clone)]
pub enum RegistryAuth {
    /// No authentication (for local development).
    None,

    /// Basic authentication (username/password or username/token).
    Basic {
        /// Username.
        username: String,
        /// Password or token.
        password: String,
    },

    /// Bearer token authentication (`OAuth2` / service account).
    Bearer {
        /// Token value.
        token: String,
    },

    /// AWS ECR authentication (uses IAM credentials).
    AwsEcr {
        /// AWS region.
        region: String,
    },

    /// GCP Artifact Registry authentication (uses ADC).
    GcpArtifact {
        /// GCP project ID.
        project: String,
        /// Artifact Registry location.
        location: String,
    },
}

impl RegistryAuth {
    /// Creates basic authentication.
    ///
    /// # Examples
    ///
    /// ```
    /// use eunomia_registry::RegistryAuth;
    ///
    /// let auth = RegistryAuth::basic("user", "pass");
    /// ```
    #[must_use]
    pub fn basic(username: impl Into<String>, password: impl Into<String>) -> Self {
        Self::Basic {
            username: username.into(),
            password: password.into(),
        }
    }

    /// Creates bearer token authentication.
    ///
    /// # Examples
    ///
    /// ```
    /// use eunomia_registry::RegistryAuth;
    ///
    /// let auth = RegistryAuth::bearer("my-token");
    /// ```
    #[must_use]
    pub fn bearer(token: impl Into<String>) -> Self {
        Self::Bearer {
            token: token.into(),
        }
    }

    /// Creates AWS ECR authentication.
    #[must_use]
    pub fn aws_ecr(region: impl Into<String>) -> Self {
        Self::AwsEcr {
            region: region.into(),
        }
    }

    /// Creates GCP Artifact Registry authentication.
    #[must_use]
    pub fn gcp_artifact(project: impl Into<String>, location: impl Into<String>) -> Self {
        Self::GcpArtifact {
            project: project.into(),
            location: location.into(),
        }
    }
}

/// TLS configuration for mTLS connections.
#[derive(Debug, Clone)]
pub struct TlsConfig {
    /// Path to CA certificate file.
    pub ca_cert: Option<PathBuf>,

    /// Path to client certificate file.
    pub client_cert: Option<PathBuf>,

    /// Path to client private key file.
    pub client_key: Option<PathBuf>,

    /// Whether to skip certificate verification (NOT recommended for production).
    pub insecure_skip_verify: bool,
}

impl Default for TlsConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl TlsConfig {
    /// Creates a new TLS configuration with default settings.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            ca_cert: None,
            client_cert: None,
            client_key: None,
            insecure_skip_verify: false,
        }
    }

    /// Sets the CA certificate path.
    #[must_use]
    pub fn with_ca_cert(mut self, path: impl Into<PathBuf>) -> Self {
        self.ca_cert = Some(path.into());
        self
    }

    /// Sets client certificate and key paths for mTLS.
    #[must_use]
    pub fn with_client_cert(
        mut self,
        cert: impl Into<PathBuf>,
        key: impl Into<PathBuf>,
    ) -> Self {
        self.client_cert = Some(cert.into());
        self.client_key = Some(key.into());
        self
    }

    /// Enables insecure mode (skips certificate verification).
    ///
    /// # Warning
    ///
    /// This should only be used for testing. Never use in production.
    #[must_use]
    pub const fn insecure(mut self) -> Self {
        self.insecure_skip_verify = true;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_new() {
        let config = RegistryConfig::new("https://example.com");
        assert_eq!(config.url, "https://example.com");
        assert!(config.namespace.is_empty());
        assert_eq!(config.timeout, Duration::from_secs(30));
    }

    #[test]
    fn test_config_with_namespace() {
        let config = RegistryConfig::new("https://example.com")
            .with_namespace("policies");
        assert_eq!(config.namespace, "policies");
    }

    #[test]
    fn test_repository_name_with_namespace() {
        let config = RegistryConfig::new("https://example.com")
            .with_namespace("eunomia");
        assert_eq!(
            config.repository_name("users-service"),
            "eunomia/users-service"
        );
    }

    #[test]
    fn test_repository_name_without_namespace() {
        let config = RegistryConfig::new("https://example.com");
        assert_eq!(config.repository_name("users-service"), "users-service");
    }

    #[test]
    fn test_basic_auth() {
        let auth = RegistryAuth::basic("user", "pass");
        assert!(matches!(
            auth,
            RegistryAuth::Basic { username, password }
            if username == "user" && password == "pass"
        ));
    }

    #[test]
    fn test_bearer_auth() {
        let auth = RegistryAuth::bearer("token123");
        assert!(matches!(
            auth,
            RegistryAuth::Bearer { token } if token == "token123"
        ));
    }

    #[test]
    fn test_tls_config() {
        let tls = TlsConfig::new()
            .with_ca_cert("/path/to/ca.crt")
            .with_client_cert("/path/to/client.crt", "/path/to/client.key");
        
        assert_eq!(tls.ca_cert, Some(PathBuf::from("/path/to/ca.crt")));
        assert_eq!(tls.client_cert, Some(PathBuf::from("/path/to/client.crt")));
        assert_eq!(tls.client_key, Some(PathBuf::from("/path/to/client.key")));
        assert!(!tls.insecure_skip_verify);
    }
}
