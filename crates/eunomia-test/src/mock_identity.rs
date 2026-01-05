//! Mock identity builders for policy testing.
//!
//! This module provides convenient builders for creating [`CallerIdentity`] values
//! in tests. These builders offer sensible defaults and fluent APIs for common
//! testing scenarios.
//!
//! # Overview
//!
//! The module provides three main builders:
//! - [`MockUser`] - For user identities with roles and optional tenant
//! - [`MockSpiffe`] - For service identities with SPIFFE IDs
//! - [`MockApiKey`] - For API key identities with scopes
//!
//! # Examples
//!
//! ```rust
//! use eunomia_test::{MockUser, MockSpiffe, MockApiKey};
//!
//! // Create an admin user
//! let admin = MockUser::admin();
//!
//! // Create a user with custom roles
//! let user = MockUser::new("user-123")
//!     .with_roles(["viewer", "editor"])
//!     .with_tenant("tenant-abc")
//!     .build();
//!
//! // Create a service identity
//! let service = MockSpiffe::new("orders-service")
//!     .with_trust_domain("example.com")
//!     .build();
//!
//! // Create an API key with scopes
//! let api_key = MockApiKey::new("key-456")
//!     .with_scopes(["read:users", "write:orders"])
//!     .build();
//! ```

use eunomia_core::CallerIdentity;

/// Builder for mock user identities.
///
/// Provides a fluent API for creating user identities with roles and optional
/// tenant information. Includes factory methods for common user types.
///
/// # Examples
///
/// ```rust
/// use eunomia_test::MockUser;
///
/// // Quick admin user
/// let admin = MockUser::admin();
///
/// // Custom user
/// let user = MockUser::new("user-123")
///     .with_roles(["viewer"])
///     .build();
/// ```
#[derive(Debug, Clone)]
pub struct MockUser {
    user_id: String,
    roles: Vec<String>,
    tenant_id: Option<String>,
}

impl MockUser {
    /// Creates a new mock user builder with the given user ID.
    #[must_use]
    pub fn new(user_id: impl Into<String>) -> Self {
        Self {
            user_id: user_id.into(),
            roles: Vec::new(),
            tenant_id: None,
        }
    }

    /// Creates an admin user with the "admin" role.
    ///
    /// User ID defaults to "mock-admin".
    #[must_use]
    pub fn admin() -> CallerIdentity {
        Self::new("mock-admin").with_role("admin").build()
    }

    /// Creates a viewer user with the "viewer" role.
    ///
    /// User ID defaults to "mock-viewer".
    #[must_use]
    pub fn viewer() -> CallerIdentity {
        Self::new("mock-viewer").with_role("viewer").build()
    }

    /// Creates an editor user with the "editor" role.
    ///
    /// User ID defaults to "mock-editor".
    #[must_use]
    pub fn editor() -> CallerIdentity {
        Self::new("mock-editor").with_role("editor").build()
    }

    /// Creates a guest user with no roles.
    ///
    /// User ID defaults to "mock-guest".
    #[must_use]
    pub fn guest() -> CallerIdentity {
        Self::new("mock-guest").build()
    }

    /// Creates a super admin user with `super_admin` and `admin` roles.
    ///
    /// User ID defaults to "mock-super-admin".
    #[must_use]
    pub fn super_admin() -> CallerIdentity {
        Self::new("mock-super-admin")
            .with_roles(["super_admin", "admin"])
            .build()
    }

    /// Adds a single role to the user.
    #[must_use]
    pub fn with_role(mut self, role: impl Into<String>) -> Self {
        self.roles.push(role.into());
        self
    }

    /// Sets the roles for this user, replacing any existing roles.
    #[must_use]
    pub fn with_roles<I, S>(mut self, roles: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.roles = roles.into_iter().map(Into::into).collect();
        self
    }

    /// Sets the tenant ID for this user (multi-tenant scenarios).
    #[must_use]
    pub fn with_tenant(mut self, tenant_id: impl Into<String>) -> Self {
        self.tenant_id = Some(tenant_id.into());
        self
    }

    /// Builds the [`CallerIdentity`].
    #[must_use]
    pub fn build(self) -> CallerIdentity {
        if let Some(tenant_id) = self.tenant_id {
            CallerIdentity::user_with_tenant(self.user_id, self.roles, tenant_id)
        } else {
            CallerIdentity::user(self.user_id, self.roles)
        }
    }
}

/// Builder for mock SPIFFE service identities.
///
/// Provides a fluent API for creating service identities with SPIFFE IDs.
/// Useful for testing service-to-service authorization.
///
/// # Examples
///
/// ```rust
/// use eunomia_test::MockSpiffe;
///
/// // Simple service identity
/// let service = MockSpiffe::new("orders-service").build();
///
/// // Full SPIFFE ID
/// let service = MockSpiffe::new("orders-service")
///     .with_trust_domain("prod.example.com")
///     .with_namespace("production")
///     .build();
/// ```
#[derive(Debug, Clone)]
pub struct MockSpiffe {
    service_name: String,
    trust_domain: String,
    namespace: String,
}

impl MockSpiffe {
    /// Default trust domain for mock identities.
    pub const DEFAULT_TRUST_DOMAIN: &'static str = "test.local";

    /// Default namespace for mock identities.
    pub const DEFAULT_NAMESPACE: &'static str = "default";

    /// Creates a new mock SPIFFE identity builder.
    #[must_use]
    pub fn new(service_name: impl Into<String>) -> Self {
        Self {
            service_name: service_name.into(),
            trust_domain: Self::DEFAULT_TRUST_DOMAIN.to_string(),
            namespace: Self::DEFAULT_NAMESPACE.to_string(),
        }
    }

    /// Creates a service identity for the "users" service.
    #[must_use]
    pub fn users_service() -> CallerIdentity {
        Self::new("users-service").build()
    }

    /// Creates a service identity for the "orders" service.
    #[must_use]
    pub fn orders_service() -> CallerIdentity {
        Self::new("orders-service").build()
    }

    /// Creates a service identity for the "gateway" service.
    #[must_use]
    pub fn gateway() -> CallerIdentity {
        Self::new("gateway").build()
    }

    /// Sets the trust domain.
    #[must_use]
    pub fn with_trust_domain(mut self, domain: impl Into<String>) -> Self {
        self.trust_domain = domain.into();
        self
    }

    /// Sets the namespace.
    #[must_use]
    pub fn with_namespace(mut self, namespace: impl Into<String>) -> Self {
        self.namespace = namespace.into();
        self
    }

    /// Builds the [`CallerIdentity`].
    #[must_use]
    pub fn build(self) -> CallerIdentity {
        let spiffe_id = format!(
            "spiffe://{}/ns/{}/sa/{}",
            self.trust_domain, self.namespace, self.service_name
        );
        CallerIdentity::spiffe(spiffe_id, self.service_name, self.trust_domain)
    }
}

/// Builder for mock API key identities.
///
/// Provides a fluent API for creating API key identities with scopes.
///
/// # Examples
///
/// ```rust
/// use eunomia_test::MockApiKey;
///
/// // Read-only API key
/// let key = MockApiKey::read_only();
///
/// // Custom API key with scopes
/// let key = MockApiKey::new("key-123")
///     .with_scopes(["read:users", "write:orders"])
///     .build();
/// ```
#[derive(Debug, Clone)]
pub struct MockApiKey {
    key_id: String,
    scopes: Vec<String>,
}

impl MockApiKey {
    /// Creates a new mock API key builder.
    #[must_use]
    pub fn new(key_id: impl Into<String>) -> Self {
        Self {
            key_id: key_id.into(),
            scopes: Vec::new(),
        }
    }

    /// Creates a read-only API key with "read:*" scope.
    #[must_use]
    pub fn read_only() -> CallerIdentity {
        Self::new("mock-read-key").with_scope("read:*").build()
    }

    /// Creates a full-access API key with "admin:*" scope.
    #[must_use]
    pub fn full_access() -> CallerIdentity {
        Self::new("mock-admin-key").with_scope("admin:*").build()
    }

    /// Creates an API key with specific service read access.
    #[must_use]
    pub fn read_service(service: &str) -> CallerIdentity {
        Self::new(format!("mock-{service}-read-key"))
            .with_scope(format!("read:{service}"))
            .build()
    }

    /// Creates an API key with specific service write access.
    #[must_use]
    pub fn write_service(service: &str) -> CallerIdentity {
        Self::new(format!("mock-{service}-write-key"))
            .with_scopes([format!("read:{service}"), format!("write:{service}")])
            .build()
    }

    /// Adds a single scope to the API key.
    #[must_use]
    pub fn with_scope(mut self, scope: impl Into<String>) -> Self {
        self.scopes.push(scope.into());
        self
    }

    /// Sets the scopes for this API key, replacing any existing scopes.
    #[must_use]
    pub fn with_scopes<I, S>(mut self, scopes: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.scopes = scopes.into_iter().map(Into::into).collect();
        self
    }

    /// Builds the [`CallerIdentity`].
    #[must_use]
    pub fn build(self) -> CallerIdentity {
        CallerIdentity::api_key(self.key_id, self.scopes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_user_admin() {
        let admin = MockUser::admin();
        match admin {
            CallerIdentity::User { user_id, roles, .. } => {
                assert_eq!(user_id, "mock-admin");
                assert!(roles.contains(&"admin".to_string()));
            }
            _ => panic!("Expected User identity"),
        }
    }

    #[test]
    fn test_mock_user_with_tenant() {
        let user = MockUser::new("user-123")
            .with_roles(["viewer"])
            .with_tenant("tenant-abc")
            .build();

        match user {
            CallerIdentity::User {
                user_id,
                roles,
                tenant_id,
            } => {
                assert_eq!(user_id, "user-123");
                assert!(roles.contains(&"viewer".to_string()));
                assert_eq!(tenant_id, Some("tenant-abc".to_string()));
            }
            _ => panic!("Expected User identity"),
        }
    }

    #[test]
    fn test_mock_user_guest() {
        let guest = MockUser::guest();
        match guest {
            CallerIdentity::User { roles, .. } => {
                assert!(roles.is_empty());
            }
            _ => panic!("Expected User identity"),
        }
    }

    #[test]
    fn test_mock_spiffe_default() {
        let service = MockSpiffe::new("my-service").build();
        match service {
            CallerIdentity::Spiffe {
                spiffe_id,
                service_name,
                trust_domain,
            } => {
                assert_eq!(service_name, "my-service");
                assert_eq!(trust_domain, "test.local");
                assert!(spiffe_id.contains("my-service"));
            }
            _ => panic!("Expected Spiffe identity"),
        }
    }

    #[test]
    fn test_mock_spiffe_custom_domain() {
        let service = MockSpiffe::new("orders")
            .with_trust_domain("prod.example.com")
            .with_namespace("production")
            .build();

        match service {
            CallerIdentity::Spiffe {
                spiffe_id,
                trust_domain,
                ..
            } => {
                assert_eq!(trust_domain, "prod.example.com");
                assert!(spiffe_id.contains("production"));
            }
            _ => panic!("Expected Spiffe identity"),
        }
    }

    #[test]
    fn test_mock_spiffe_factories() {
        let users = MockSpiffe::users_service();
        let orders = MockSpiffe::orders_service();
        let gateway = MockSpiffe::gateway();

        match users {
            CallerIdentity::Spiffe { service_name, .. } => {
                assert_eq!(service_name, "users-service");
            }
            _ => panic!("Expected Spiffe"),
        }

        match orders {
            CallerIdentity::Spiffe { service_name, .. } => {
                assert_eq!(service_name, "orders-service");
            }
            _ => panic!("Expected Spiffe"),
        }

        match gateway {
            CallerIdentity::Spiffe { service_name, .. } => {
                assert_eq!(service_name, "gateway");
            }
            _ => panic!("Expected Spiffe"),
        }
    }

    #[test]
    fn test_mock_api_key_read_only() {
        let key = MockApiKey::read_only();
        match key {
            CallerIdentity::ApiKey { scopes, .. } => {
                assert!(scopes.contains(&"read:*".to_string()));
            }
            _ => panic!("Expected ApiKey identity"),
        }
    }

    #[test]
    fn test_mock_api_key_service_access() {
        let read_key = MockApiKey::read_service("users");
        let write_key = MockApiKey::write_service("orders");

        match read_key {
            CallerIdentity::ApiKey { scopes, key_id } => {
                assert!(key_id.contains("users"));
                assert!(scopes.contains(&"read:users".to_string()));
            }
            _ => panic!("Expected ApiKey identity"),
        }

        match write_key {
            CallerIdentity::ApiKey { scopes, key_id } => {
                assert!(key_id.contains("orders"));
                assert!(scopes.contains(&"read:orders".to_string()));
                assert!(scopes.contains(&"write:orders".to_string()));
            }
            _ => panic!("Expected ApiKey identity"),
        }
    }

    #[test]
    fn test_mock_api_key_custom_scopes() {
        let key = MockApiKey::new("custom-key")
            .with_scopes(["scope1", "scope2"])
            .build();

        match key {
            CallerIdentity::ApiKey { key_id, scopes } => {
                assert_eq!(key_id, "custom-key");
                assert_eq!(scopes.len(), 2);
            }
            _ => panic!("Expected ApiKey identity"),
        }
    }
}
