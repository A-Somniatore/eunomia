//! Audit event definitions.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::{Timestamp, Uuid};

/// Generates a new v7 UUID for audit events.
fn new_event_id() -> Uuid {
    let ts = Timestamp::now(uuid::NoContext);
    Uuid::new_v7(ts)
}

/// Severity level for audit events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum EventSeverity {
    /// Informational event
    #[default]
    Info,
    /// Warning event
    Warning,
    /// Error event
    Error,
    /// Critical event requiring immediate attention
    Critical,
}

/// Outcome of an audited operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EventOutcome {
    /// Operation succeeded
    Success,
    /// Operation failed
    Failure,
    /// Operation was denied
    Denied,
    /// Operation is in progress
    InProgress,
}

/// Base trait for all audit events.
pub trait AuditEvent: Serialize {
    /// Returns the event type identifier.
    fn event_type(&self) -> &'static str;

    /// Returns the event severity.
    fn severity(&self) -> EventSeverity;

    /// Returns the event timestamp.
    fn timestamp(&self) -> DateTime<Utc>;

    /// Returns the correlation ID for request tracing.
    fn correlation_id(&self) -> Option<&str>;
}

/// Policy lifecycle events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyEvent {
    /// Unique event ID
    pub id: Uuid,

    /// Event timestamp
    pub timestamp: DateTime<Utc>,

    /// Type of policy event
    pub event_type: PolicyEventType,

    /// Service name the policy belongs to
    pub service: String,

    /// Policy version
    pub version: String,

    /// Git commit SHA
    pub git_commit: Option<String>,

    /// Actor who triggered the event
    pub actor: String,

    /// Event outcome
    pub outcome: EventOutcome,

    /// Additional details
    pub details: Option<String>,

    /// Correlation ID for tracing
    pub correlation_id: Option<String>,
}

/// Types of policy events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyEventType {
    /// Policy was created
    Created,
    /// Policy was updated
    Updated,
    /// Policy was deleted
    Deleted,
    /// Policy validation occurred
    Validated,
    /// Policy tests were run
    Tested,
}

impl PolicyEvent {
    /// Creates a new policy created event.
    #[must_use]
    pub fn created(service: &str, version: &str, actor: &str) -> Self {
        Self {
            id: new_event_id(),
            timestamp: Utc::now(),
            event_type: PolicyEventType::Created,
            service: service.to_string(),
            version: version.to_string(),
            git_commit: None,
            actor: actor.to_string(),
            outcome: EventOutcome::Success,
            details: None,
            correlation_id: None,
        }
    }

    /// Creates a new policy updated event.
    #[must_use]
    pub fn updated(service: &str, version: &str, actor: &str) -> Self {
        Self {
            id: new_event_id(),
            timestamp: Utc::now(),
            event_type: PolicyEventType::Updated,
            service: service.to_string(),
            version: version.to_string(),
            git_commit: None,
            actor: actor.to_string(),
            outcome: EventOutcome::Success,
            details: None,
            correlation_id: None,
        }
    }

    /// Creates a new policy deleted event.
    #[must_use]
    pub fn deleted(service: &str, version: &str, actor: &str) -> Self {
        Self {
            id: new_event_id(),
            timestamp: Utc::now(),
            event_type: PolicyEventType::Deleted,
            service: service.to_string(),
            version: version.to_string(),
            git_commit: None,
            actor: actor.to_string(),
            outcome: EventOutcome::Success,
            details: None,
            correlation_id: None,
        }
    }

    /// Creates a new policy validated event.
    #[must_use]
    pub fn validated(service: &str, version: &str, outcome: EventOutcome) -> Self {
        Self {
            id: new_event_id(),
            timestamp: Utc::now(),
            event_type: PolicyEventType::Validated,
            service: service.to_string(),
            version: version.to_string(),
            git_commit: None,
            actor: "system".to_string(),
            outcome,
            details: None,
            correlation_id: None,
        }
    }

    /// Creates a new policy tested event.
    #[must_use]
    pub fn tested(service: &str, version: &str, passed: usize, failed: usize) -> Self {
        let outcome = if failed == 0 {
            EventOutcome::Success
        } else {
            EventOutcome::Failure
        };

        Self {
            id: new_event_id(),
            timestamp: Utc::now(),
            event_type: PolicyEventType::Tested,
            service: service.to_string(),
            version: version.to_string(),
            git_commit: None,
            actor: "system".to_string(),
            outcome,
            details: Some(format!("{passed} passed, {failed} failed")),
            correlation_id: None,
        }
    }

    /// Sets the git commit SHA.
    #[must_use]
    pub fn with_git_commit(mut self, commit: &str) -> Self {
        self.git_commit = Some(commit.to_string());
        self
    }

    /// Sets the correlation ID.
    #[must_use]
    pub fn with_correlation_id(mut self, id: &str) -> Self {
        self.correlation_id = Some(id.to_string());
        self
    }

    /// Sets additional details.
    #[must_use]
    pub fn with_details(mut self, details: &str) -> Self {
        self.details = Some(details.to_string());
        self
    }
}

impl AuditEvent for PolicyEvent {
    fn event_type(&self) -> &'static str {
        match self.event_type {
            PolicyEventType::Created => "policy.created",
            PolicyEventType::Updated => "policy.updated",
            PolicyEventType::Deleted => "policy.deleted",
            PolicyEventType::Validated => "policy.validated",
            PolicyEventType::Tested => "policy.tested",
        }
    }

    fn severity(&self) -> EventSeverity {
        match self.outcome {
            EventOutcome::Success | EventOutcome::InProgress => EventSeverity::Info,
            EventOutcome::Failure | EventOutcome::Denied => EventSeverity::Warning,
        }
    }

    fn timestamp(&self) -> DateTime<Utc> {
        self.timestamp
    }

    fn correlation_id(&self) -> Option<&str> {
        self.correlation_id.as_deref()
    }
}

/// Bundle operation events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleEvent {
    /// Unique event ID
    pub id: Uuid,

    /// Event timestamp
    pub timestamp: DateTime<Utc>,

    /// Type of bundle event
    pub event_type: BundleEventType,

    /// Service name
    pub service: String,

    /// Bundle version
    pub version: String,

    /// Bundle checksum
    pub checksum: Option<String>,

    /// Bundle size in bytes
    pub size_bytes: Option<u64>,

    /// Actor who triggered the event
    pub actor: String,

    /// Event outcome
    pub outcome: EventOutcome,

    /// Additional details
    pub details: Option<String>,

    /// Correlation ID for tracing
    pub correlation_id: Option<String>,
}

/// Types of bundle events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BundleEventType {
    /// Bundle was compiled
    Compiled,
    /// Bundle was signed
    Signed,
    /// Bundle was published to registry
    Published,
    /// Bundle was fetched from registry
    Fetched,
    /// Bundle signature was verified
    Verified,
}

impl BundleEvent {
    /// Creates a new bundle compiled event.
    #[must_use]
    pub fn compiled(service: &str, version: &str) -> Self {
        Self {
            id: new_event_id(),
            timestamp: Utc::now(),
            event_type: BundleEventType::Compiled,
            service: service.to_string(),
            version: version.to_string(),
            checksum: None,
            size_bytes: None,
            actor: "system".to_string(),
            outcome: EventOutcome::Success,
            details: None,
            correlation_id: None,
        }
    }

    /// Creates a new bundle signed event.
    #[must_use]
    pub fn signed(service: &str, version: &str, key_id: &str) -> Self {
        Self {
            id: new_event_id(),
            timestamp: Utc::now(),
            event_type: BundleEventType::Signed,
            service: service.to_string(),
            version: version.to_string(),
            checksum: None,
            size_bytes: None,
            actor: "system".to_string(),
            outcome: EventOutcome::Success,
            details: Some(format!("key_id={key_id}")),
            correlation_id: None,
        }
    }

    /// Creates a new bundle published event.
    #[must_use]
    pub fn published(service: &str, version: &str, registry: &str) -> Self {
        Self {
            id: new_event_id(),
            timestamp: Utc::now(),
            event_type: BundleEventType::Published,
            service: service.to_string(),
            version: version.to_string(),
            checksum: None,
            size_bytes: None,
            actor: "system".to_string(),
            outcome: EventOutcome::Success,
            details: Some(format!("registry={registry}")),
            correlation_id: None,
        }
    }

    /// Creates a new bundle fetched event.
    #[must_use]
    pub fn fetched(service: &str, version: &str, registry: &str) -> Self {
        Self {
            id: new_event_id(),
            timestamp: Utc::now(),
            event_type: BundleEventType::Fetched,
            service: service.to_string(),
            version: version.to_string(),
            checksum: None,
            size_bytes: None,
            actor: "system".to_string(),
            outcome: EventOutcome::Success,
            details: Some(format!("registry={registry}")),
            correlation_id: None,
        }
    }

    /// Sets the bundle checksum.
    #[must_use]
    pub fn with_checksum(mut self, checksum: &str) -> Self {
        self.checksum = Some(checksum.to_string());
        self
    }

    /// Sets the bundle size.
    #[must_use]
    pub const fn with_size(mut self, size_bytes: u64) -> Self {
        self.size_bytes = Some(size_bytes);
        self
    }

    /// Sets the correlation ID.
    #[must_use]
    pub fn with_correlation_id(mut self, id: &str) -> Self {
        self.correlation_id = Some(id.to_string());
        self
    }
}

impl AuditEvent for BundleEvent {
    fn event_type(&self) -> &'static str {
        match self.event_type {
            BundleEventType::Compiled => "bundle.compiled",
            BundleEventType::Signed => "bundle.signed",
            BundleEventType::Published => "bundle.published",
            BundleEventType::Fetched => "bundle.fetched",
            BundleEventType::Verified => "bundle.verified",
        }
    }

    fn severity(&self) -> EventSeverity {
        EventSeverity::Info
    }

    fn timestamp(&self) -> DateTime<Utc> {
        self.timestamp
    }

    fn correlation_id(&self) -> Option<&str> {
        self.correlation_id.as_deref()
    }
}

/// Distribution events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributionEvent {
    /// Unique event ID
    pub id: Uuid,

    /// Event timestamp
    pub timestamp: DateTime<Utc>,

    /// Type of distribution event
    pub event_type: DistributionEventType,

    /// Service name
    pub service: String,

    /// Bundle version being distributed
    pub version: String,

    /// Target instance endpoint
    pub instance: Option<String>,

    /// Number of instances affected
    pub instance_count: Option<usize>,

    /// Deployment strategy used
    pub strategy: Option<String>,

    /// Event outcome
    pub outcome: EventOutcome,

    /// Additional details
    pub details: Option<String>,

    /// Correlation ID for tracing
    pub correlation_id: Option<String>,
}

/// Types of distribution events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DistributionEventType {
    /// Deployment started
    DeploymentStarted,
    /// Deployment completed
    DeploymentCompleted,
    /// Deployment failed
    DeploymentFailed,
    /// Rollback initiated
    RollbackStarted,
    /// Rollback completed
    RollbackCompleted,
    /// Instance health check
    HealthCheck,
}

impl DistributionEvent {
    /// Creates a new deployment started event.
    #[must_use]
    pub fn deployment_started(
        service: &str,
        version: &str,
        instance_count: usize,
        strategy: &str,
    ) -> Self {
        Self {
            id: new_event_id(),
            timestamp: Utc::now(),
            event_type: DistributionEventType::DeploymentStarted,
            service: service.to_string(),
            version: version.to_string(),
            instance: None,
            instance_count: Some(instance_count),
            strategy: Some(strategy.to_string()),
            outcome: EventOutcome::InProgress,
            details: None,
            correlation_id: None,
        }
    }

    /// Creates a new deployment completed event.
    #[must_use]
    pub fn deployment_completed(
        service: &str,
        version: &str,
        successful: usize,
        failed: usize,
    ) -> Self {
        let outcome = if failed == 0 {
            EventOutcome::Success
        } else {
            EventOutcome::Failure
        };

        Self {
            id: new_event_id(),
            timestamp: Utc::now(),
            event_type: DistributionEventType::DeploymentCompleted,
            service: service.to_string(),
            version: version.to_string(),
            instance: None,
            instance_count: Some(successful + failed),
            strategy: None,
            outcome,
            details: Some(format!("{successful} successful, {failed} failed")),
            correlation_id: None,
        }
    }

    /// Creates a new rollback started event.
    #[must_use]
    pub fn rollback_started(service: &str, from_version: &str, to_version: &str) -> Self {
        Self {
            id: new_event_id(),
            timestamp: Utc::now(),
            event_type: DistributionEventType::RollbackStarted,
            service: service.to_string(),
            version: to_version.to_string(),
            instance: None,
            instance_count: None,
            strategy: None,
            outcome: EventOutcome::InProgress,
            details: Some(format!("from={from_version}")),
            correlation_id: None,
        }
    }

    /// Creates a new rollback completed event.
    #[must_use]
    pub fn rollback_completed(service: &str, version: &str, success: bool) -> Self {
        Self {
            id: new_event_id(),
            timestamp: Utc::now(),
            event_type: DistributionEventType::RollbackCompleted,
            service: service.to_string(),
            version: version.to_string(),
            instance: None,
            instance_count: None,
            strategy: None,
            outcome: if success {
                EventOutcome::Success
            } else {
                EventOutcome::Failure
            },
            details: None,
            correlation_id: None,
        }
    }

    /// Sets the correlation ID.
    #[must_use]
    pub fn with_correlation_id(mut self, id: &str) -> Self {
        self.correlation_id = Some(id.to_string());
        self
    }
}

impl AuditEvent for DistributionEvent {
    fn event_type(&self) -> &'static str {
        match self.event_type {
            DistributionEventType::DeploymentStarted => "distribution.deployment_started",
            DistributionEventType::DeploymentCompleted => "distribution.deployment_completed",
            DistributionEventType::DeploymentFailed => "distribution.deployment_failed",
            DistributionEventType::RollbackStarted => "distribution.rollback_started",
            DistributionEventType::RollbackCompleted => "distribution.rollback_completed",
            DistributionEventType::HealthCheck => "distribution.health_check",
        }
    }

    fn severity(&self) -> EventSeverity {
        match self.event_type {
            DistributionEventType::DeploymentFailed => EventSeverity::Error,
            DistributionEventType::RollbackStarted => EventSeverity::Warning,
            _ => EventSeverity::Info,
        }
    }

    fn timestamp(&self) -> DateTime<Utc> {
        self.timestamp
    }

    fn correlation_id(&self) -> Option<&str> {
        self.correlation_id.as_deref()
    }
}

/// Authorization decision events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorizationEvent {
    /// Unique event ID
    pub id: Uuid,

    /// Event timestamp
    pub timestamp: DateTime<Utc>,

    /// Service where the decision was made
    pub service: String,

    /// Operation being authorized
    pub operation_id: String,

    /// Caller type
    pub caller_type: String,

    /// Caller identifier (`user_id`, `service_name`, `key_id`)
    pub caller_id: Option<String>,

    /// Authorization decision
    pub allowed: bool,

    /// Reason for the decision
    pub reason: Option<String>,

    /// Policy version used
    pub policy_version: Option<String>,

    /// Evaluation time in nanoseconds
    pub evaluation_time_ns: Option<u64>,

    /// Correlation ID for tracing
    pub correlation_id: Option<String>,
}

impl AuthorizationEvent {
    /// Creates a new authorization allowed event.
    #[must_use]
    pub fn allowed(service: &str, operation_id: &str, caller_type: &str) -> Self {
        Self {
            id: new_event_id(),
            timestamp: Utc::now(),
            service: service.to_string(),
            operation_id: operation_id.to_string(),
            caller_type: caller_type.to_string(),
            caller_id: None,
            allowed: true,
            reason: None,
            policy_version: None,
            evaluation_time_ns: None,
            correlation_id: None,
        }
    }

    /// Creates a new authorization denied event.
    #[must_use]
    pub fn denied(service: &str, operation_id: &str, caller_type: &str, reason: &str) -> Self {
        Self {
            id: new_event_id(),
            timestamp: Utc::now(),
            service: service.to_string(),
            operation_id: operation_id.to_string(),
            caller_type: caller_type.to_string(),
            caller_id: None,
            allowed: false,
            reason: Some(reason.to_string()),
            policy_version: None,
            evaluation_time_ns: None,
            correlation_id: None,
        }
    }

    /// Sets the caller ID.
    #[must_use]
    pub fn with_caller_id(mut self, id: &str) -> Self {
        self.caller_id = Some(id.to_string());
        self
    }

    /// Sets the policy version.
    #[must_use]
    pub fn with_policy_version(mut self, version: &str) -> Self {
        self.policy_version = Some(version.to_string());
        self
    }

    /// Sets the evaluation time.
    #[must_use]
    pub const fn with_evaluation_time(mut self, time_ns: u64) -> Self {
        self.evaluation_time_ns = Some(time_ns);
        self
    }

    /// Sets the correlation ID.
    #[must_use]
    pub fn with_correlation_id(mut self, id: &str) -> Self {
        self.correlation_id = Some(id.to_string());
        self
    }
}

impl AuditEvent for AuthorizationEvent {
    fn event_type(&self) -> &'static str {
        if self.allowed {
            "authorization.allowed"
        } else {
            "authorization.denied"
        }
    }

    fn severity(&self) -> EventSeverity {
        if self.allowed {
            EventSeverity::Info
        } else {
            EventSeverity::Warning
        }
    }

    fn timestamp(&self) -> DateTime<Utc> {
        self.timestamp
    }

    fn correlation_id(&self) -> Option<&str> {
        self.correlation_id.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_policy_event_created() {
        let event = PolicyEvent::created("users-service", "1.0.0", "user@example.com");

        assert_eq!(event.event_type, PolicyEventType::Created);
        assert_eq!(event.service, "users-service");
        assert_eq!(event.version, "1.0.0");
        assert_eq!(event.actor, "user@example.com");
        assert_eq!(event.outcome, EventOutcome::Success);
    }

    #[test]
    fn test_policy_event_with_git_commit() {
        let event = PolicyEvent::created("users-service", "1.0.0", "user@example.com")
            .with_git_commit("abc123");

        assert_eq!(event.git_commit, Some("abc123".to_string()));
    }

    #[test]
    fn test_policy_event_tested() {
        let event = PolicyEvent::tested("users-service", "1.0.0", 10, 0);
        assert_eq!(event.outcome, EventOutcome::Success);

        let event = PolicyEvent::tested("users-service", "1.0.0", 8, 2);
        assert_eq!(event.outcome, EventOutcome::Failure);
    }

    #[test]
    fn test_bundle_event_compiled() {
        let event = BundleEvent::compiled("users-service", "1.0.0")
            .with_checksum("sha256:abc123")
            .with_size(1024);

        assert_eq!(event.event_type, BundleEventType::Compiled);
        assert_eq!(event.checksum, Some("sha256:abc123".to_string()));
        assert_eq!(event.size_bytes, Some(1024));
    }

    #[test]
    fn test_distribution_event_deployment() {
        let event =
            DistributionEvent::deployment_started("users-service", "1.0.0", 3, "immediate");

        assert_eq!(event.event_type, DistributionEventType::DeploymentStarted);
        assert_eq!(event.instance_count, Some(3));
        assert_eq!(event.strategy, Some("immediate".to_string()));
    }

    #[test]
    fn test_authorization_event_allowed() {
        let event = AuthorizationEvent::allowed("users-service", "getUser", "user")
            .with_caller_id("user-123")
            .with_policy_version("1.0.0")
            .with_evaluation_time(1500);

        assert!(event.allowed);
        assert_eq!(event.caller_id, Some("user-123".to_string()));
        assert_eq!(event.policy_version, Some("1.0.0".to_string()));
        assert_eq!(event.evaluation_time_ns, Some(1500));
    }

    #[test]
    fn test_authorization_event_denied() {
        let event = AuthorizationEvent::denied(
            "users-service",
            "deleteUser",
            "user",
            "insufficient permissions",
        );

        assert!(!event.allowed);
        assert_eq!(
            event.reason,
            Some("insufficient permissions".to_string())
        );
    }

    #[test]
    fn test_event_serialization() {
        let event = PolicyEvent::created("users-service", "1.0.0", "user@example.com");
        let json = serde_json::to_string(&event).unwrap();

        assert!(json.contains("\"event_type\":\"created\""));
        assert!(json.contains("\"service\":\"users-service\""));
        assert!(json.contains("\"version\":\"1.0.0\""));
    }
}
