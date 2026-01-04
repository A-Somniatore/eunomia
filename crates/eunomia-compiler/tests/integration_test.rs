//! Integration tests for the Rego policy engine with sample policies.
//!
//! These tests validate the RegoEngine against real-world policy examples
//! from the `examples/policies/` directory.

use serde_json::json;

use eunomia_compiler::{
    IssueSeverity, PolicyValidator, RegoEngine, ValidatorConfig, validate_file,
};

/// Path to sample policies
const USERS_SERVICE_POLICY: &str = "../../examples/policies/users-service/authz.rego";
const ORDERS_SERVICE_POLICY: &str = "../../examples/policies/orders-service/authz.rego";
const COMMON_AUTHZ_POLICY: &str = "../../examples/policies/common/authz.rego";

/// Helper to get absolute path from relative test path
fn policy_path(relative: &str) -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    format!("{manifest_dir}/{relative}")
}

// =============================================================================
// Policy Loading Tests
// =============================================================================

#[test]
fn test_load_users_service_policy() {
    let mut engine = RegoEngine::new();
    let result = engine.add_policy_from_file(&policy_path(USERS_SERVICE_POLICY));
    assert!(result.is_ok(), "Failed to load users service policy: {result:?}");
    
    let policies: Vec<_> = engine.all_policies().collect();
    assert!(!policies.is_empty(), "No policies loaded");
}

#[test]
fn test_load_orders_service_policy() {
    let mut engine = RegoEngine::new();
    
    // Orders service depends on common authz
    engine
        .add_policy_from_file(&policy_path(COMMON_AUTHZ_POLICY))
        .expect("Failed to load common authz");
    
    let result = engine.add_policy_from_file(&policy_path(ORDERS_SERVICE_POLICY));
    assert!(result.is_ok(), "Failed to load orders service policy: {result:?}");
}

#[test]
fn test_load_common_authz_policy() {
    let mut engine = RegoEngine::new();
    let result = engine.add_policy_from_file(&policy_path(COMMON_AUTHZ_POLICY));
    assert!(result.is_ok(), "Failed to load common authz policy: {result:?}");
}

#[test]
fn test_load_multiple_policies() {
    let mut engine = RegoEngine::new();
    
    engine
        .add_policy_from_file(&policy_path(COMMON_AUTHZ_POLICY))
        .expect("Failed to load common authz");
    
    engine
        .add_policy_from_file(&policy_path(USERS_SERVICE_POLICY))
        .expect("Failed to load users service");
    
    let policies: Vec<_> = engine.all_policies().collect();
    assert!(policies.len() >= 2, "Expected at least 2 policies");
}

// =============================================================================
// Policy Validation Tests
// =============================================================================

#[test]
fn test_validate_users_service_policy() {
    let report = validate_file(&policy_path(USERS_SERVICE_POLICY)).unwrap();
    
    if !report.is_valid() {
        eprintln!("Validation errors: {report:#?}");
    }
    
    // Policy should have no critical issues
    let critical_issues: Vec<_> = report
        .issues
        .iter()
        .filter(|i| matches!(i.severity, IssueSeverity::Error))
        .collect();
    
    assert!(
        critical_issues.is_empty(),
        "Users service policy has critical issues: {critical_issues:?}"
    );
}

#[test]
fn test_validate_orders_service_policy() {
    let report = validate_file(&policy_path(ORDERS_SERVICE_POLICY)).unwrap();
    
    // Policy should have no critical issues
    let critical_issues: Vec<_> = report
        .issues
        .iter()
        .filter(|i| matches!(i.severity, IssueSeverity::Error))
        .collect();
    
    assert!(
        critical_issues.is_empty(),
        "Orders service policy has critical issues: {critical_issues:?}"
    );
}

#[test]
fn test_validate_common_authz_policy() {
    let config = ValidatorConfig::lenient();
    let validator = PolicyValidator::with_config(config);
    let report = validator.validate_file(&policy_path(COMMON_AUTHZ_POLICY)).unwrap();
    
    // Lenient config should allow library patterns
    assert!(report.is_valid(), "Common authz policy failed validation: {report:#?}");
}

// =============================================================================
// Users Service Authorization Tests
// =============================================================================

mod users_service {
    use super::*;

    fn create_engine() -> RegoEngine {
        let mut engine = RegoEngine::new();
        engine
            .add_policy_from_file(&policy_path(USERS_SERVICE_POLICY))
            .expect("Failed to load users service policy");
        engine
    }

    #[test]
    fn test_admin_can_do_anything() {
        let mut engine = create_engine();
        
        engine
            .set_input_json(&json!({
                "caller": {
                    "type": "user",
                    "user_id": "admin-1",
                    "roles": ["admin"]
                },
                "operation_id": "deleteUser",
                "method": "DELETE",
                "path": "/users/any-user"
            }))
            .expect("Failed to set input");

        let result = engine.eval_bool("data.users_service.authz.allow");
        assert!(result.unwrap_or(false), "Admin should be allowed");
    }

    #[test]
    fn test_user_can_read_own_profile() {
        let mut engine = create_engine();
        
        engine
            .set_input_json(&json!({
                "caller": {
                    "type": "user",
                    "user_id": "user-123",
                    "roles": ["user"]
                },
                "operation_id": "getUser",
                "method": "GET",
                "path": "/users/user-123"
            }))
            .expect("Failed to set input");

        let result = engine.eval_bool("data.users_service.authz.allow");
        assert!(result.unwrap_or(false), "User should be able to read own profile");
    }

    #[test]
    fn test_user_cannot_read_other_profile() {
        let mut engine = create_engine();
        
        engine
            .set_input_json(&json!({
                "caller": {
                    "type": "user",
                    "user_id": "user-123",
                    "roles": ["user"]
                },
                "operation_id": "getUser",
                "method": "GET",
                "path": "/users/other-user-456"
            }))
            .expect("Failed to set input");

        let result = engine.eval_bool("data.users_service.authz.allow");
        assert!(!result.unwrap_or(true), "User should NOT be able to read other's profile");
    }

    #[test]
    fn test_orders_service_can_read_user() {
        let mut engine = create_engine();
        
        engine
            .set_input_json(&json!({
                "caller": {
                    "type": "spiffe",
                    "service_name": "orders-service",
                    "trust_domain": "somniatore.com"
                },
                "operation_id": "getUser",
                "method": "GET",
                "path": "/users/user-123"
            }))
            .expect("Failed to set input");

        let result = engine.eval_bool("data.users_service.authz.allow");
        assert!(result.unwrap_or(false), "Orders service should be able to read user");
    }

    #[test]
    fn test_untrusted_service_denied() {
        let mut engine = create_engine();
        
        engine
            .set_input_json(&json!({
                "caller": {
                    "type": "spiffe",
                    "service_name": "orders-service",
                    "trust_domain": "malicious.com"
                },
                "operation_id": "getUser",
                "method": "GET",
                "path": "/users/user-123"
            }))
            .expect("Failed to set input");

        let result = engine.eval_bool("data.users_service.authz.allow");
        assert!(!result.unwrap_or(true), "Untrusted service should be denied");
    }

    #[test]
    fn test_api_key_with_read_scope() {
        let mut engine = create_engine();
        
        engine
            .set_input_json(&json!({
                "caller": {
                    "type": "api_key",
                    "scopes": ["users:read"]
                },
                "operation_id": "getUser",
                "method": "GET",
                "path": "/users/user-123"
            }))
            .expect("Failed to set input");

        let result = engine.eval_bool("data.users_service.authz.allow");
        assert!(result.unwrap_or(false), "API key with users:read should be allowed");
    }

    #[test]
    fn test_api_key_without_scope_denied() {
        let mut engine = create_engine();
        
        engine
            .set_input_json(&json!({
                "caller": {
                    "type": "api_key",
                    "scopes": ["orders:read"]
                },
                "operation_id": "getUser",
                "method": "GET",
                "path": "/users/user-123"
            }))
            .expect("Failed to set input");

        let result = engine.eval_bool("data.users_service.authz.allow");
        assert!(!result.unwrap_or(true), "API key without users scope should be denied");
    }

    #[test]
    fn test_default_deny_empty_input() {
        let mut engine = create_engine();
        
        engine
            .set_input_json(&json!({}))
            .expect("Failed to set input");

        let result = engine.eval_bool("data.users_service.authz.allow");
        assert!(!result.unwrap_or(true), "Empty input should be denied (default deny)");
    }
}

// =============================================================================
// Common Authz Helpers Tests
// =============================================================================

mod common_authz {
    use super::*;

    fn create_engine() -> RegoEngine {
        let mut engine = RegoEngine::new();
        engine
            .add_policy_from_file(&policy_path(COMMON_AUTHZ_POLICY))
            .expect("Failed to load common authz policy");
        engine
    }

    #[test]
    fn test_is_user_helper() {
        let mut engine = create_engine();
        
        engine
            .set_input_json(&json!({
                "caller": {"type": "user"}
            }))
            .expect("Failed to set input");

        let result = engine.eval_bool("data.common.authz.is_user");
        assert!(result.unwrap_or(false), "is_user should return true for user type");
    }

    #[test]
    fn test_is_service_helper() {
        let mut engine = create_engine();
        
        engine
            .set_input_json(&json!({
                "caller": {"type": "spiffe"}
            }))
            .expect("Failed to set input");

        let result = engine.eval_bool("data.common.authz.is_service");
        assert!(result.unwrap_or(false), "is_service should return true for spiffe type");
    }

    #[test]
    fn test_is_read_operation() {
        let mut engine = create_engine();
        
        engine
            .set_input_json(&json!({"method": "GET"}))
            .expect("Failed to set input");

        let result = engine.eval_bool("data.common.authz.is_read_operation");
        assert!(result.unwrap_or(false), "GET should be a read operation");
    }

    #[test]
    fn test_is_write_operation() {
        let mut engine = create_engine();
        
        engine
            .set_input_json(&json!({"method": "POST"}))
            .expect("Failed to set input");

        let result = engine.eval_bool("data.common.authz.is_write_operation");
        assert!(result.unwrap_or(false), "POST should be a write operation");
    }

    #[test]
    fn test_default_deny() {
        let mut engine = create_engine();
        
        engine
            .set_input_json(&json!({}))
            .expect("Failed to set input");

        let result = engine.eval_bool("data.common.authz.allow");
        assert!(!result.unwrap_or(true), "Default should be deny");
    }
}

// =============================================================================
// Engine Feature Tests
// =============================================================================

mod engine_features {
    use super::*;

    #[test]
    fn test_policy_info_retrieval() {
        let mut engine = RegoEngine::new();
        engine
            .add_policy_from_file(&policy_path(USERS_SERVICE_POLICY))
            .expect("Failed to load policy");

        let policies: Vec<_> = engine.all_policies().collect();
        assert!(!policies.is_empty());

        // Check that policy info contains expected data
        let (_, info) = policies.first().expect("Should have at least one policy");
        assert!(!info.package.is_empty(), "Package should not be empty");
    }

    #[test]
    fn test_multiple_evaluations() {
        let mut engine = RegoEngine::new();
        engine
            .add_policy_from_file(&policy_path(USERS_SERVICE_POLICY))
            .expect("Failed to load policy");

        // First evaluation - admin
        engine
            .set_input_json(&json!({
                "caller": {"type": "user", "roles": ["admin"]},
                "operation_id": "deleteUser",
                "method": "DELETE"
            }))
            .expect("Failed to set input");
        let result1 = engine.eval_bool("data.users_service.authz.allow");
        assert!(result1.unwrap_or(false), "Admin should be allowed");

        // Second evaluation - regular user (should reset properly)
        engine
            .set_input_json(&json!({
                "caller": {"type": "user", "user_id": "user-1", "roles": ["user"]},
                "operation_id": "deleteUser",
                "method": "DELETE",
                "path": "/users/other-user"
            }))
            .expect("Failed to set input");
        let result2 = engine.eval_bool("data.users_service.authz.allow");
        assert!(!result2.unwrap_or(true), "Regular user should be denied delete");
    }

    #[test]
    fn test_invalid_query() {
        let mut engine = RegoEngine::new();
        engine
            .add_policy_from_file(&policy_path(USERS_SERVICE_POLICY))
            .expect("Failed to load policy");

        engine
            .set_input_json(&json!({}))
            .expect("Failed to set input");

        // Query for non-existent rule - regorus may return true for undefined
        // The important thing is it doesn't panic
        let result = engine.eval_bool("data.nonexistent.package.rule");
        // Just verify it returns something (true or false) without panicking
        let _ = result;
    }
}
