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
use themis_platform_types::identity::{ApiKeyIdentity, SpiffeIdentity, UserIdentity};

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
    email: Option<String>,
    name: Option<String>,
    roles: Vec<String>,
    groups: Vec<String>,
    tenant_id: Option<String>,
}

impl MockUser {
    /// Creates a new mock user builder with the given user ID.
    #[must_use]
    pub fn new(user_id: impl Into<String>) -> Self {
        let id = user_id.into();
        Self {
            email: Some(format!("{id}@mock.test")),
            user_id: id,
            name: None,
            roles: Vec::new(),
            groups: Vec::new(),
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

    /// Sets the email for this user.
    #[must_use]
    pub fn with_email(mut self, email: impl Into<String>) -> Self {
        self.email = Some(email.into());
        self
    }

    /// Sets the display name for this user.
    #[must_use]
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
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

    /// Adds a single group to the user.
    #[must_use]
    pub fn with_group(mut self, group: impl Into<String>) -> Self {
        self.groups.push(group.into());
        self
    }

    /// Sets the groups for this user, replacing any existing groups.
    #[must_use]
    pub fn with_groups<I, S>(mut self, groups: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.groups = groups.into_iter().map(Into::into).collect();
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
        CallerIdentity::User(UserIdentity {
            user_id: self.user_id,
            email: self.email,
            name: self.name,
            roles: self.roles,
            groups: self.groups,
            tenant_id: self.tenant_id,
        })
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
        CallerIdentity::Spiffe(SpiffeIdentity {
            spiffe_id,
            trust_domain: Some(self.trust_domain),
            service_name: Some(self.service_name),
        })
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
    name: String,
    scopes: Vec<String>,
    owner_id: Option<String>,
}

impl MockApiKey {
    /// Creates a new mock API key builder.
    #[must_use]
    pub fn new(key_id: impl Into<String>) -> Self {
        let id = key_id.into();
        Self {
            name: format!("Mock Key {id}"),
            key_id: id,
            scopes: Vec::new(),
            owner_id: None,
        }
    }

    /// Creates a read-only API key with "read:*" scope.
    #[must_use]
    pub fn read_only() -> CallerIdentity {
        Self::new("mock-read-key")
            .with_name("Mock Read-Only Key")
            .with_scope("read:*")
            .build()
    }

    /// Creates a full-access API key with "admin:*" scope.
    #[must_use]
    pub fn full_access() -> CallerIdentity {
        Self::new("mock-admin-key")
            .with_name("Mock Admin Key")
            .with_scope("admin:*")
            .build()
    }

    /// Creates an API key with specific service read access.
    #[must_use]
    pub fn read_service(service: &str) -> CallerIdentity {
        Self::new(format!("mock-{service}-read-key"))
            .with_name(format!("{service} Read Key"))
            .with_scope(format!("read:{service}"))
            .build()
    }

    /// Creates an API key with specific service write access.
    #[must_use]
    pub fn write_service(service: &str) -> CallerIdentity {
        Self::new(format!("mock-{service}-write-key"))
            .with_name(format!("{service} Write Key"))
            .with_scopes([format!("read:{service}"), format!("write:{service}")])
            .build()
    }

    /// Sets the human-readable name for this API key.
    #[must_use]
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Sets the owner ID for this API key.
    #[must_use]
    pub fn with_owner(mut self, owner_id: impl Into<String>) -> Self {
        self.owner_id = Some(owner_id.into());
        self
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
        CallerIdentity::ApiKey(ApiKeyIdentity {
            key_id: self.key_id,
            name: self.name,
            scopes: self.scopes,
            owner_id: self.owner_id,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_user_admin() {
        let admin = MockUser::admin();
        match admin {
            CallerIdentity::User(user) => {
                assert_eq!(user.user_id, "mock-admin");
                assert!(user.roles.contains(&"admin".to_string()));
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
            CallerIdentity::User(u) => {
                assert_eq!(u.user_id, "user-123");
                assert!(u.roles.contains(&"viewer".to_string()));
                assert_eq!(u.tenant_id, Some("tenant-abc".to_string()));
            }
            _ => panic!("Expected User identity"),
        }
    }

    #[test]
    fn test_mock_user_guest() {
        let guest = MockUser::guest();
        match guest {
            CallerIdentity::User(u) => {
                assert!(u.roles.is_empty());
            }
            _ => panic!("Expected User identity"),
        }
    }

    #[test]
    fn test_mock_spiffe_default() {
        let service = MockSpiffe::new("my-service").build();
        match service {
            CallerIdentity::Spiffe(s) => {
                assert_eq!(s.service_name, Some("my-service".to_string()));
                assert_eq!(s.trust_domain, Some("test.local".to_string()));
                assert!(s.spiffe_id.contains("my-service"));
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
            CallerIdentity::Spiffe(s) => {
                assert_eq!(s.trust_domain, Some("prod.example.com".to_string()));
                assert!(s.spiffe_id.contains("production"));
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
            CallerIdentity::Spiffe(s) => {
                assert_eq!(s.service_name, Some("users-service".to_string()));
            }
            _ => panic!("Expected Spiffe"),
        }

        match orders {
            CallerIdentity::Spiffe(s) => {
                assert_eq!(s.service_name, Some("orders-service".to_string()));
            }
            _ => panic!("Expected Spiffe"),
        }

        match gateway {
            CallerIdentity::Spiffe(s) => {
                assert_eq!(s.service_name, Some("gateway".to_string()));
            }
            _ => panic!("Expected Spiffe"),
        }
    }

    #[test]
    fn test_mock_api_key_read_only() {
        let key = MockApiKey::read_only();
        match key {
            CallerIdentity::ApiKey(k) => {
                assert!(k.scopes.contains(&"read:*".to_string()));
            }
            _ => panic!("Expected ApiKey identity"),
        }
    }

    #[test]
    fn test_mock_api_key_service_access() {
        let read_key = MockApiKey::read_service("users");
        let write_key = MockApiKey::write_service("orders");

        match read_key {
            CallerIdentity::ApiKey(k) => {
                assert!(k.key_id.contains("users"));
                assert!(k.scopes.contains(&"read:users".to_string()));
            }
            _ => panic!("Expected ApiKey identity"),
        }

        match write_key {
            CallerIdentity::ApiKey(k) => {
                assert!(k.key_id.contains("orders"));
                assert!(k.scopes.contains(&"read:orders".to_string()));
                assert!(k.scopes.contains(&"write:orders".to_string()));
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
            CallerIdentity::ApiKey(k) => {
                assert_eq!(k.key_id, "custom-key");
                assert_eq!(k.scopes.len(), 2);
            }
            _ => panic!("Expected ApiKey identity"),
        }
    }
}
