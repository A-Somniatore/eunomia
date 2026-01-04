//! Authorization decision types.
//!
//! This module defines the [`AuthorizationDecision`] structure that represents
//! the result of evaluating a policy.

use serde::{Deserialize, Serialize};

/// The result of evaluating an authorization policy.
///
/// This structure contains the decision (allow/deny) along with metadata
/// about the evaluation for auditing and debugging purposes.
///
/// # Examples
///
/// ```rust
/// use eunomia_core::AuthorizationDecision;
///
/// let decision = AuthorizationDecision::allow("admin access granted", "users_service.authz");
/// assert!(decision.allowed);
///
/// let denied = AuthorizationDecision::deny("insufficient permissions", "users_service.authz");
/// assert!(!denied.allowed);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthorizationDecision {
    /// Whether the request is allowed.
    pub allowed: bool,

    /// Reason for the decision (for auditing and debugging).
    pub reason: String,

    /// Identifier of the policy that made the decision.
    pub policy_id: String,

    /// Version of the policy bundle.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policy_version: Option<String>,

    /// Time taken to evaluate the policy, in nanoseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evaluation_time_ns: Option<u64>,
}

impl AuthorizationDecision {
    /// Creates a new allow decision.
    ///
    /// # Arguments
    ///
    /// * `reason` - Reason for allowing the request
    /// * `policy_id` - Identifier of the policy that made the decision
    ///
    /// # Examples
    ///
    /// ```rust
    /// use eunomia_core::AuthorizationDecision;
    ///
    /// let decision = AuthorizationDecision::allow(
    ///     "user has admin role",
    ///     "users_service.authz",
    /// );
    /// assert!(decision.allowed);
    /// ```
    #[must_use]
    pub fn allow(reason: impl Into<String>, policy_id: impl Into<String>) -> Self {
        Self {
            allowed: true,
            reason: reason.into(),
            policy_id: policy_id.into(),
            policy_version: None,
            evaluation_time_ns: None,
        }
    }

    /// Creates a new deny decision.
    ///
    /// # Arguments
    ///
    /// * `reason` - Reason for denying the request
    /// * `policy_id` - Identifier of the policy that made the decision
    ///
    /// # Examples
    ///
    /// ```rust
    /// use eunomia_core::AuthorizationDecision;
    ///
    /// let decision = AuthorizationDecision::deny(
    ///     "no matching allow rule",
    ///     "users_service.authz",
    /// );
    /// assert!(!decision.allowed);
    /// ```
    #[must_use]
    pub fn deny(reason: impl Into<String>, policy_id: impl Into<String>) -> Self {
        Self {
            allowed: false,
            reason: reason.into(),
            policy_id: policy_id.into(),
            policy_version: None,
            evaluation_time_ns: None,
        }
    }

    /// Sets the policy version for this decision.
    #[must_use]
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.policy_version = Some(version.into());
        self
    }

    /// Sets the evaluation time for this decision.
    #[must_use]
    pub const fn with_evaluation_time(mut self, time_ns: u64) -> Self {
        self.evaluation_time_ns = Some(time_ns);
        self
    }

    /// Returns true if the request was allowed.
    #[must_use]
    pub const fn is_allowed(&self) -> bool {
        self.allowed
    }

    /// Returns true if the request was denied.
    #[must_use]
    pub const fn is_denied(&self) -> bool {
        !self.allowed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allow_decision() {
        let decision = AuthorizationDecision::allow("access granted", "test.authz");

        assert!(decision.allowed);
        assert!(decision.is_allowed());
        assert!(!decision.is_denied());
        assert_eq!(decision.reason, "access granted");
        assert_eq!(decision.policy_id, "test.authz");
        assert!(decision.policy_version.is_none());
        assert!(decision.evaluation_time_ns.is_none());
    }

    #[test]
    fn test_deny_decision() {
        let decision = AuthorizationDecision::deny("access denied", "test.authz");

        assert!(!decision.allowed);
        assert!(!decision.is_allowed());
        assert!(decision.is_denied());
        assert_eq!(decision.reason, "access denied");
        assert_eq!(decision.policy_id, "test.authz");
    }

    #[test]
    fn test_decision_with_version() {
        let decision = AuthorizationDecision::allow("granted", "test").with_version("1.2.3");

        assert_eq!(decision.policy_version, Some("1.2.3".to_string()));
    }

    #[test]
    fn test_decision_with_evaluation_time() {
        let decision =
            AuthorizationDecision::allow("granted", "test").with_evaluation_time(1_000_000);

        assert_eq!(decision.evaluation_time_ns, Some(1_000_000));
    }

    #[test]
    fn test_decision_serialization() {
        let decision = AuthorizationDecision::allow("admin access", "users_service.authz")
            .with_version("1.0.0")
            .with_evaluation_time(500_000);

        let json = serde_json::to_string(&decision).unwrap();
        assert!(json.contains(r#""allowed":true"#));
        assert!(json.contains(r#""reason":"admin access""#));
        assert!(json.contains(r#""policy_id":"users_service.authz""#));
        assert!(json.contains(r#""policy_version":"1.0.0""#));
        assert!(json.contains(r#""evaluation_time_ns":500000"#));

        let deserialized: AuthorizationDecision = serde_json::from_str(&json).unwrap();
        assert_eq!(decision, deserialized);
    }

    #[test]
    fn test_decision_serialization_without_optional_fields() {
        let decision = AuthorizationDecision::deny("no access", "test");

        let json = serde_json::to_string(&decision).unwrap();
        assert!(!json.contains("policy_version"));
        assert!(!json.contains("evaluation_time_ns"));
    }
}
