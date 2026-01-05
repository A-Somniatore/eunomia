//! Caller identity types for authorization.
//!
//! This module defines the different types of caller identities that can be
//! used in authorization decisions:
//!
//! - [`CallerIdentity::Spiffe`] - Internal service with SPIFFE ID
//! - [`CallerIdentity::User`] - External user with roles
//! - [`CallerIdentity::ApiKey`] - API key with scopes
//! - [`CallerIdentity::Anonymous`] - Unauthenticated caller
//!
//! **Note:** This module is deprecated. Use [`CallerIdentity`](themis_platform_types::CallerIdentity)
//! from the `themis-platform-types` crate instead.

use serde::{Deserialize, Serialize};

/// Represents the identity of a caller making an authorization request.
///
/// The identity type determines which authorization rules apply and what
/// attributes are available for policy evaluation.
///
/// **Deprecated:** Use [`themis_platform_types::CallerIdentity`] instead.
///
/// # Examples
///
/// ```rust,ignore
/// use eunomia_core::identity::CallerIdentity;
///
/// // Internal service identity
/// let service = CallerIdentity::spiffe(
///     "spiffe://somniatore.com/ns/production/sa/orders-service",
///     "orders-service",
///     "somniatore.com",
/// );
///
/// // User identity
/// let user = CallerIdentity::user("user-123", vec!["admin".to_string()]);
///
/// // API key identity
/// let api_key = CallerIdentity::api_key("key-456", vec!["read:users".to_string()]);
///
/// // Anonymous identity
/// let anonymous = CallerIdentity::anonymous();
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CallerIdentity {
    /// Internal service identity using SPIFFE.
    ///
    /// Used for service-to-service communication within the Themis ecosystem.
    #[serde(rename = "spiffe")]
    Spiffe {
        /// Full SPIFFE ID (e.g., `spiffe://domain/ns/namespace/sa/service`).
        spiffe_id: String,
        /// Parsed service name from the SPIFFE ID.
        service_name: String,
        /// Trust domain from the SPIFFE ID.
        trust_domain: String,
    },

    /// External user identity.
    ///
    /// Represents an authenticated user, typically from an identity provider.
    #[serde(rename = "user")]
    User {
        /// Unique user identifier.
        user_id: String,
        /// Roles assigned to the user.
        roles: Vec<String>,
        /// Optional tenant identifier (for multi-tenant systems).
        #[serde(skip_serializing_if = "Option::is_none")]
        tenant_id: Option<String>,
    },

    /// API key identity.
    ///
    /// Used for programmatic access with scoped permissions.
    #[serde(rename = "api_key")]
    ApiKey {
        /// Unique key identifier.
        key_id: String,
        /// Scopes granted to the API key.
        scopes: Vec<String>,
    },

    /// Anonymous/unauthenticated identity.
    ///
    /// Used when no authentication is provided.
    #[serde(rename = "anonymous")]
    #[default]
    Anonymous,
}

impl CallerIdentity {
    /// Creates a new SPIFFE identity.
    ///
    /// # Arguments
    ///
    /// * `spiffe_id` - Full SPIFFE ID URI
    /// * `service_name` - Parsed service name
    /// * `trust_domain` - Trust domain
    ///
    /// **Deprecated:** Use [`themis_platform_types::CallerIdentity::spiffe_full`] instead.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use eunomia_core::identity::CallerIdentity;
    ///
    /// let identity = CallerIdentity::spiffe(
    ///     "spiffe://somniatore.com/ns/prod/sa/orders",
    ///     "orders",
    ///     "somniatore.com",
    /// );
    /// ```
    #[must_use]
    pub fn spiffe(
        spiffe_id: impl Into<String>,
        service_name: impl Into<String>,
        trust_domain: impl Into<String>,
    ) -> Self {
        Self::Spiffe {
            spiffe_id: spiffe_id.into(),
            service_name: service_name.into(),
            trust_domain: trust_domain.into(),
        }
    }

    /// Creates a new user identity.
    ///
    /// # Arguments
    ///
    /// * `user_id` - Unique user identifier
    /// * `roles` - Roles assigned to the user
    ///
    /// **Deprecated:** Use [`themis_platform_types::CallerIdentity::user`] instead (takes email, not roles).
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use eunomia_core::identity::CallerIdentity;
    ///
    /// let user = CallerIdentity::user("user-123", vec!["admin".to_string()]);
    /// ```
    #[must_use]
    pub fn user(user_id: impl Into<String>, roles: Vec<String>) -> Self {
        Self::User {
            user_id: user_id.into(),
            roles,
            tenant_id: None,
        }
    }

    /// Creates a new user identity with a tenant.
    ///
    /// # Arguments
    ///
    /// * `user_id` - Unique user identifier
    /// * `roles` - Roles assigned to the user
    /// * `tenant_id` - Tenant identifier (for multi-tenant systems)
    ///
    /// **Deprecated:** This method is not available in `themis-platform-types`.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use eunomia_core::identity::CallerIdentity;
    ///
    /// let user = CallerIdentity::user_with_tenant(
    ///     "user-123",
    ///     vec!["editor".to_string()],
    ///     "tenant-456",
    /// );
    /// ```
    #[must_use]
    pub fn user_with_tenant(
        user_id: impl Into<String>,
        roles: Vec<String>,
        tenant_id: impl Into<String>,
    ) -> Self {
        Self::User {
            user_id: user_id.into(),
            roles,
            tenant_id: Some(tenant_id.into()),
        }
    }

    /// Creates a new API key identity.
    ///
    /// # Arguments
    ///
    /// * `key_id` - Unique key identifier
    /// * `scopes` - Scopes granted to the API key
    ///
    /// **Deprecated:** Use [`themis_platform_types::CallerIdentity::api_key`] instead (takes name, not scopes).
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use eunomia_core::identity::CallerIdentity;
    ///
    /// let api_key = CallerIdentity::api_key(
    ///     "key-789",
    ///     vec!["read:users".to_string(), "write:orders".to_string()],
    /// );
    /// ```
    #[must_use]
    pub fn api_key(key_id: impl Into<String>, scopes: Vec<String>) -> Self {
        Self::ApiKey {
            key_id: key_id.into(),
            scopes,
        }
    }

    /// Creates an anonymous identity.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use eunomia_core::CallerIdentity;
    ///
    /// let anonymous = CallerIdentity::anonymous();
    /// ```
    #[must_use]
    pub const fn anonymous() -> Self {
        Self::Anonymous
    }

    /// Returns true if this is a SPIFFE identity.
    #[must_use]
    pub const fn is_spiffe(&self) -> bool {
        matches!(self, Self::Spiffe { .. })
    }

    /// Returns true if this is a user identity.
    #[must_use]
    pub const fn is_user(&self) -> bool {
        matches!(self, Self::User { .. })
    }

    /// Returns true if this is an API key identity.
    #[must_use]
    pub const fn is_api_key(&self) -> bool {
        matches!(self, Self::ApiKey { .. })
    }

    /// Returns true if this is an anonymous identity.
    #[must_use]
    pub const fn is_anonymous(&self) -> bool {
        matches!(self, Self::Anonymous)
    }

    /// Returns the identity type as a string.
    #[must_use]
    pub const fn identity_type(&self) -> &'static str {
        match self {
            Self::Spiffe { .. } => "spiffe",
            Self::User { .. } => "user",
            Self::ApiKey { .. } => "api_key",
            Self::Anonymous => "anonymous",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spiffe_identity_creation() {
        let identity = CallerIdentity::spiffe(
            "spiffe://somniatore.com/ns/prod/sa/orders",
            "orders",
            "somniatore.com",
        );

        match identity {
            CallerIdentity::Spiffe {
                spiffe_id,
                service_name,
                trust_domain,
            } => {
                assert_eq!(spiffe_id, "spiffe://somniatore.com/ns/prod/sa/orders");
                assert_eq!(service_name, "orders");
                assert_eq!(trust_domain, "somniatore.com");
            }
            _ => panic!("Expected Spiffe variant"),
        }
    }

    #[test]
    fn test_user_identity_creation() {
        let identity = CallerIdentity::user("user-123", vec!["admin".to_string()]);

        match identity {
            CallerIdentity::User {
                user_id,
                roles,
                tenant_id,
            } => {
                assert_eq!(user_id, "user-123");
                assert_eq!(roles, vec!["admin"]);
                assert!(tenant_id.is_none());
            }
            _ => panic!("Expected User variant"),
        }
    }

    #[test]
    fn test_user_with_tenant_identity_creation() {
        let identity =
            CallerIdentity::user_with_tenant("user-123", vec!["editor".to_string()], "tenant-456");

        match identity {
            CallerIdentity::User {
                user_id,
                roles,
                tenant_id,
            } => {
                assert_eq!(user_id, "user-123");
                assert_eq!(roles, vec!["editor"]);
                assert_eq!(tenant_id, Some("tenant-456".to_string()));
            }
            _ => panic!("Expected User variant"),
        }
    }

    #[test]
    fn test_api_key_identity_creation() {
        let identity = CallerIdentity::api_key("key-789", vec!["read:users".to_string()]);

        match identity {
            CallerIdentity::ApiKey { key_id, scopes } => {
                assert_eq!(key_id, "key-789");
                assert_eq!(scopes, vec!["read:users"]);
            }
            _ => panic!("Expected ApiKey variant"),
        }
    }

    #[test]
    fn test_anonymous_identity() {
        let identity = CallerIdentity::anonymous();
        assert!(identity.is_anonymous());
        assert_eq!(identity.identity_type(), "anonymous");
    }

    #[test]
    fn test_identity_type_checks() {
        let spiffe = CallerIdentity::spiffe("id", "name", "domain");
        assert!(spiffe.is_spiffe());
        assert!(!spiffe.is_user());
        assert!(!spiffe.is_api_key());
        assert!(!spiffe.is_anonymous());

        let user = CallerIdentity::user("id", vec![]);
        assert!(!user.is_spiffe());
        assert!(user.is_user());
        assert!(!user.is_api_key());
        assert!(!user.is_anonymous());
    }

    #[test]
    fn test_serialization_spiffe() {
        let identity = CallerIdentity::spiffe(
            "spiffe://somniatore.com/ns/prod/sa/orders",
            "orders",
            "somniatore.com",
        );

        let json = serde_json::to_string(&identity).unwrap();
        assert!(json.contains(r#""type":"spiffe""#));
        assert!(json.contains(r#""spiffe_id":"spiffe://somniatore.com/ns/prod/sa/orders""#));

        let deserialized: CallerIdentity = serde_json::from_str(&json).unwrap();
        assert_eq!(identity, deserialized);
    }

    #[test]
    fn test_serialization_user() {
        let identity = CallerIdentity::user("user-123", vec!["admin".to_string()]);

        let json = serde_json::to_string(&identity).unwrap();
        assert!(json.contains(r#""type":"user""#));
        assert!(json.contains(r#""user_id":"user-123""#));

        let deserialized: CallerIdentity = serde_json::from_str(&json).unwrap();
        assert_eq!(identity, deserialized);
    }

    #[test]
    fn test_serialization_anonymous() {
        let identity = CallerIdentity::anonymous();

        let json = serde_json::to_string(&identity).unwrap();
        assert_eq!(json, r#"{"type":"anonymous"}"#);

        let deserialized: CallerIdentity = serde_json::from_str(&json).unwrap();
        assert_eq!(identity, deserialized);
    }

    #[test]
    fn test_default_is_anonymous() {
        let identity = CallerIdentity::default();
        assert!(identity.is_anonymous());
    }
}
