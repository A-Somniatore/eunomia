//! Property-based tests for eunomia-core types.
//!
//! These tests use proptest to verify invariants across many randomly generated inputs.

use proptest::prelude::*;

use crate::{AuthorizationDecision, Bundle, CallerIdentity, Policy, PolicyInput};

/// Strategy for generating valid SPIFFE IDs.
fn spiffe_id_strategy() -> impl Strategy<Value = String> {
    (
        "[a-z][a-z0-9-]{2,20}\\.[a-z]{2,6}",  // trust domain
        "[a-z][a-z0-9-]{2,20}",                // namespace
        "[a-z][a-z0-9-]{2,30}",                // service
    )
        .prop_map(|(domain, ns, svc)| format!("spiffe://{}/ns/{}/sa/{}", domain, ns, svc))
}

/// Strategy for generating service names.
fn service_name_strategy() -> impl Strategy<Value = String> {
    "[a-z][a-z0-9-]{2,30}-service"
}

/// Strategy for generating user IDs.
fn user_id_strategy() -> impl Strategy<Value = String> {
    "(user|usr|u)-[a-f0-9]{8,32}"
}

/// Strategy for generating role names.
fn role_strategy() -> impl Strategy<Value = String> {
    "(admin|editor|viewer|moderator|owner|member|guest)"
}

/// Strategy for generating API key IDs.
fn api_key_id_strategy() -> impl Strategy<Value = String> {
    "(key|api|ak)-[a-f0-9]{16,32}"
}

/// Strategy for generating scopes.
fn scope_strategy() -> impl Strategy<Value = String> {
    "(read|write|delete|admin):(users|orders|payments|products|all)"
}

/// Strategy for generating operation IDs.
fn operation_id_strategy() -> impl Strategy<Value = String> {
    "(get|list|create|update|delete|find)(User|Order|Payment|Product|Item)(ById|s|)"
}

/// Strategy for generating HTTP methods.
fn http_method_strategy() -> impl Strategy<Value = String> {
    "(GET|POST|PUT|PATCH|DELETE|HEAD|OPTIONS)"
}

/// Strategy for generating request paths.
fn path_strategy() -> impl Strategy<Value = String> {
    "/(users|orders|payments|products)(/[a-f0-9-]{8,36})?"
}

/// Strategy for generating environments.
fn environment_strategy() -> impl Strategy<Value = String> {
    "(production|staging|development|test)"
}

/// Strategy for generating CallerIdentity.
fn caller_identity_strategy() -> impl Strategy<Value = CallerIdentity> {
    prop_oneof![
        // SPIFFE identity
        (
            spiffe_id_strategy(),
            service_name_strategy(),
            "[a-z]+\\.[a-z]{2,6}"
        )
            .prop_map(|(id, name, domain)| CallerIdentity::spiffe(id, name, domain)),
        // User identity
        (
            user_id_strategy(),
            prop::collection::vec(role_strategy(), 0..5),
            prop::option::of("[a-z]+-[0-9]{3,6}")
        )
            .prop_map(|(id, roles, org)| {
                if let Some(org_id) = org {
                    CallerIdentity::user_with_tenant(id, roles, org_id)
                } else {
                    CallerIdentity::user(id, roles)
                }
            }),
        // API key identity
        (
            api_key_id_strategy(),
            prop::collection::vec(scope_strategy(), 1..5)
        )
            .prop_map(|(id, scopes)| CallerIdentity::api_key(id, scopes)),
        // Anonymous
        Just(CallerIdentity::anonymous()),
    ]
}

proptest! {
    /// Test that CallerIdentity serialization is reversible.
    #[test]
    fn caller_identity_roundtrip(identity in caller_identity_strategy()) {
        let json = serde_json::to_string(&identity).unwrap();
        let deserialized: CallerIdentity = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(identity, deserialized);
    }

    /// Test that CallerIdentity type checks are mutually exclusive.
    #[test]
    fn caller_identity_type_exclusive(identity in caller_identity_strategy()) {
        let checks = [
            identity.is_spiffe(),
            identity.is_user(),
            identity.is_api_key(),
            identity.is_anonymous(),
        ];
        let true_count = checks.iter().filter(|&&x| x).count();
        prop_assert_eq!(true_count, 1, "Exactly one type check should be true");
    }

    /// Test that CallerIdentity::identity_type matches the variant.
    #[test]
    fn caller_identity_type_string_matches(identity in caller_identity_strategy()) {
        let type_str = identity.identity_type();
        match &identity {
            CallerIdentity::Spiffe { .. } => prop_assert_eq!(type_str, "spiffe"),
            CallerIdentity::User { .. } => prop_assert_eq!(type_str, "user"),
            CallerIdentity::ApiKey { .. } => prop_assert_eq!(type_str, "api_key"),
            CallerIdentity::Anonymous => prop_assert_eq!(type_str, "anonymous"),
        }
    }

    /// Test that PolicyInput serialization is reversible.
    #[test]
    fn policy_input_roundtrip(
        caller in caller_identity_strategy(),
        service in service_name_strategy(),
        operation_id in operation_id_strategy(),
        method in http_method_strategy(),
        path in path_strategy(),
        environment in environment_strategy(),
    ) {
        let input = PolicyInput::builder()
            .caller(caller)
            .service(service)
            .operation_id(operation_id)
            .method(method)
            .path(path)
            .environment(environment)
            .build();

        let json = serde_json::to_string(&input).unwrap();
        let deserialized: PolicyInput = serde_json::from_str(&json).unwrap();

        prop_assert_eq!(input.service, deserialized.service);
        prop_assert_eq!(input.operation_id, deserialized.operation_id);
        prop_assert_eq!(input.method, deserialized.method);
        prop_assert_eq!(input.path, deserialized.path);
        prop_assert_eq!(input.environment, deserialized.environment);
    }

    /// Test that AuthorizationDecision serialization is reversible.
    #[test]
    fn authorization_decision_roundtrip(
        allowed in any::<bool>(),
        reason in "[a-zA-Z ]{5,50}",
        policy_id in "[a-z_]+\\.[a-z_]+",
    ) {
        let decision = if allowed {
            AuthorizationDecision::allow(&reason, &policy_id)
        } else {
            AuthorizationDecision::deny(&reason, &policy_id)
        };

        let json = serde_json::to_string(&decision).unwrap();
        let deserialized: AuthorizationDecision = serde_json::from_str(&json).unwrap();

        prop_assert_eq!(decision.allowed, deserialized.allowed);
        prop_assert_eq!(decision.reason, deserialized.reason);
        prop_assert_eq!(decision.policy_id, deserialized.policy_id);
    }

    /// Test that AuthorizationDecision is_allowed/is_denied are consistent.
    #[test]
    fn authorization_decision_allowed_consistency(
        allowed in any::<bool>(),
    ) {
        let decision = if allowed {
            AuthorizationDecision::allow("test", "test.policy")
        } else {
            AuthorizationDecision::deny("test", "test.policy")
        };

        prop_assert_eq!(decision.is_allowed(), allowed);
        prop_assert_eq!(decision.is_denied(), !allowed);
        prop_assert_eq!(decision.is_allowed(), !decision.is_denied());
    }

    /// Test that Bundle serialization is reversible.
    #[test]
    fn bundle_roundtrip(
        name in service_name_strategy(),
        version in "[0-9]+\\.[0-9]+\\.[0-9]+",
    ) {
        let bundle = Bundle::builder(&name)
            .version(&version)
            .add_policy("test.authz", "package test.authz\ndefault allow := false")
            .build();

        let json = serde_json::to_string(&bundle).unwrap();
        let deserialized: Bundle = serde_json::from_str(&json).unwrap();

        prop_assert_eq!(bundle.name, deserialized.name);
        prop_assert_eq!(bundle.version, deserialized.version);
        prop_assert_eq!(bundle.policies, deserialized.policies);
    }

    /// Test that Bundle file_name format is consistent.
    #[test]
    fn bundle_file_name_format(
        name in "[a-z][a-z0-9-]{2,20}",
        version in "[0-9]+\\.[0-9]+\\.[0-9]+",
    ) {
        let bundle = Bundle::builder(&name)
            .version(&version)
            .build();

        let file_name = bundle.file_name();
        let expected_version = format!("-v{}", version);
        prop_assert!(file_name.starts_with(&name));
        prop_assert!(file_name.contains(&expected_version));
        prop_assert!(file_name.ends_with(".bundle.tar.gz"));
    }

    /// Test that Policy is_test detection works correctly.
    #[test]
    fn policy_is_test_detection(
        service in "[a-z_]+",
        is_test in any::<bool>(),
    ) {
        let package_name = if is_test {
            format!("{}_test", service)
        } else {
            format!("{}.authz", service)
        };

        let policy = Policy::new(&package_name, "");

        if is_test {
            prop_assert!(policy.is_test(), "Policy with _test suffix should be detected as test");
        }
    }
}

#[cfg(test)]
mod additional_tests {
    use super::*;
    use std::collections::HashMap;

    /// Test deserializing CallerIdentity from external JSON format.
    #[test]
    fn test_deserialize_external_spiffe_format() {
        let json = r#"{
            "type": "spiffe",
            "spiffe_id": "spiffe://example.com/ns/prod/sa/api",
            "service_name": "api",
            "trust_domain": "example.com"
        }"#;

        let identity: CallerIdentity = serde_json::from_str(json).unwrap();
        assert!(identity.is_spiffe());
    }

    /// Test deserializing CallerIdentity from external JSON format.
    #[test]
    fn test_deserialize_external_user_format() {
        let json = r#"{
            "type": "user",
            "user_id": "user-123",
            "roles": ["admin", "editor"]
        }"#;

        let identity: CallerIdentity = serde_json::from_str(json).unwrap();
        assert!(identity.is_user());
    }

    /// Test deserializing PolicyInput from external JSON format.
    #[test]
    fn test_deserialize_external_policy_input() {
        let json = r#"{
            "caller": {"type": "anonymous"},
            "service": "users-service",
            "operation_id": "getUser",
            "method": "GET",
            "path": "/users/123",
            "timestamp": "2026-01-04T12:00:00Z",
            "environment": "production"
        }"#;

        let input: PolicyInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.service, "users-service");
        assert_eq!(input.operation_id, "getUser");
        assert!(input.caller.is_anonymous());
    }

    /// Test PolicyInput with headers.
    #[test]
    fn test_policy_input_headers_serialization() {
        let mut headers = HashMap::new();
        headers.insert("x-request-id".to_string(), "req-123".to_string());
        headers.insert("x-trace-id".to_string(), "trace-456".to_string());

        let input = PolicyInput::builder()
            .caller(CallerIdentity::anonymous())
            .service("test")
            .operation_id("test")
            .method("GET")
            .path("/test")
            .headers(headers.clone())
            .build();

        let json = serde_json::to_string(&input).unwrap();
        let deserialized: PolicyInput = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.headers, headers);
    }

    /// Test Bundle with multiple policies and data files.
    #[test]
    fn test_bundle_complex() {
        let bundle = Bundle::builder("complex-service")
            .version("2.0.0")
            .git_commit("abc123def456")
            .add_policy("complex.authz", "package complex.authz\ndefault allow := false")
            .add_policy("complex.roles", "package complex.roles\nadmin := true")
            .add_data_file("data/roles.json", r#"{"admin": ["read", "write"]}"#)
            .add_data_file("data/config.json", r#"{"enabled": true}"#)
            .revision(42)
            .add_root("complex")
            .opa_version("0.60.0")
            .metadata("team", "platform")
            .build();

        assert_eq!(bundle.policy_count(), 2);
        assert!(bundle.has_policy("complex.authz"));
        assert!(bundle.has_policy("complex.roles"));
        assert_eq!(bundle.data_files.len(), 2);
        assert_eq!(bundle.manifest.revision, 42);
        assert_eq!(bundle.git_commit, Some("abc123def456".to_string()));

        // Verify serialization roundtrip
        let json = serde_json::to_string(&bundle).unwrap();
        let deserialized: Bundle = serde_json::from_str(&json).unwrap();
        assert_eq!(bundle.name, deserialized.name);
        assert_eq!(bundle.policies.len(), deserialized.policies.len());
    }

    /// Test AuthorizationDecision with all optional fields.
    #[test]
    fn test_authorization_decision_full() {
        let decision = AuthorizationDecision::allow("admin access granted", "users_service.authz")
            .with_version("1.2.3")
            .with_evaluation_time(500_000);

        assert!(decision.is_allowed());
        assert_eq!(decision.policy_version, Some("1.2.3".to_string()));
        assert_eq!(decision.evaluation_time_ns, Some(500_000));

        let json = serde_json::to_string(&decision).unwrap();
        assert!(json.contains("policy_version"));
        assert!(json.contains("evaluation_time_ns"));
    }
}
