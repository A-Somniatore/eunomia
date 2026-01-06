//! Audit event schema and metadata definitions.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Metadata attached to all audit events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditMetadata {
    /// Schema version for this event format
    pub schema_version: String,

    /// Event source identifier
    pub source: String,

    /// Environment (production, staging, development)
    pub environment: Option<String>,

    /// Hostname where the event was generated
    pub hostname: Option<String>,

    /// Process ID
    pub pid: Option<u32>,

    /// Additional custom tags
    #[serde(default)]
    pub tags: Vec<String>,
}

impl Default for AuditMetadata {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_SCHEMA_VERSION.to_string(),
            source: "eunomia".to_string(),
            environment: None,
            hostname: None,
            pid: None,
            tags: Vec::new(),
        }
    }
}

impl AuditMetadata {
    /// Creates new metadata with default values.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates metadata with source identification.
    #[must_use]
    pub fn with_source(source: &str) -> Self {
        Self {
            source: source.to_string(),
            ..Default::default()
        }
    }

    /// Sets the environment.
    #[must_use]
    pub fn environment(mut self, env: &str) -> Self {
        self.environment = Some(env.to_string());
        self
    }

    /// Sets the hostname.
    #[must_use]
    pub fn hostname(mut self, hostname: &str) -> Self {
        self.hostname = Some(hostname.to_string());
        self
    }

    /// Sets the process ID.
    #[must_use]
    pub const fn pid(mut self, pid: u32) -> Self {
        self.pid = Some(pid);
        self
    }

    /// Adds a tag.
    #[must_use]
    pub fn tag(mut self, tag: &str) -> Self {
        self.tags.push(tag.to_string());
        self
    }
}

/// Current schema version.
pub const CURRENT_SCHEMA_VERSION: &str = "1.0.0";

/// Event schema definition for documentation and validation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventSchema {
    /// Schema name
    pub name: String,

    /// Schema version
    pub version: String,

    /// Event type pattern this schema applies to
    pub event_type_pattern: String,

    /// Schema description
    pub description: String,

    /// Required fields
    pub required_fields: Vec<FieldDefinition>,

    /// Optional fields
    pub optional_fields: Vec<FieldDefinition>,

    /// Schema creation timestamp
    pub created_at: DateTime<Utc>,
}

/// Field definition within a schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldDefinition {
    /// Field name
    pub name: String,

    /// Field type
    pub field_type: FieldType,

    /// Field description
    pub description: String,

    /// Example value
    pub example: Option<String>,
}

/// Supported field types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FieldType {
    /// String field
    String,
    /// Integer field
    Integer,
    /// Float field
    Float,
    /// Boolean field
    Boolean,
    /// Timestamp field
    Timestamp,
    /// UUID field
    Uuid,
    /// Array field
    Array,
    /// Object field
    Object,
}

impl EventSchema {
    /// Creates a new event schema.
    #[must_use]
    pub fn new(name: &str, version: &str, event_type_pattern: &str, description: &str) -> Self {
        Self {
            name: name.to_string(),
            version: version.to_string(),
            event_type_pattern: event_type_pattern.to_string(),
            description: description.to_string(),
            required_fields: Vec::new(),
            optional_fields: Vec::new(),
            created_at: Utc::now(),
        }
    }

    /// Adds a required field.
    #[must_use]
    pub fn required(mut self, name: &str, field_type: FieldType, description: &str) -> Self {
        self.required_fields.push(FieldDefinition {
            name: name.to_string(),
            field_type,
            description: description.to_string(),
            example: None,
        });
        self
    }

    /// Adds an optional field.
    #[must_use]
    pub fn optional(mut self, name: &str, field_type: FieldType, description: &str) -> Self {
        self.optional_fields.push(FieldDefinition {
            name: name.to_string(),
            field_type,
            description: description.to_string(),
            example: None,
        });
        self
    }
}

impl FieldDefinition {
    /// Creates a new field definition.
    #[must_use]
    pub fn new(name: &str, field_type: FieldType, description: &str) -> Self {
        Self {
            name: name.to_string(),
            field_type,
            description: description.to_string(),
            example: None,
        }
    }

    /// Sets an example value.
    #[must_use]
    pub fn with_example(mut self, example: &str) -> Self {
        self.example = Some(example.to_string());
        self
    }
}

/// Returns the schema for policy events.
#[must_use]
pub fn policy_event_schema() -> EventSchema {
    EventSchema::new(
        "PolicyEvent",
        "1.0.0",
        "policy.*",
        "Events related to policy lifecycle",
    )
    .required("id", FieldType::Uuid, "Unique event identifier")
    .required("timestamp", FieldType::Timestamp, "Event timestamp")
    .required("event_type", FieldType::String, "Type of policy event")
    .required("service", FieldType::String, "Service name")
    .required("version", FieldType::String, "Policy version")
    .required("actor", FieldType::String, "Actor who triggered the event")
    .required("outcome", FieldType::String, "Event outcome")
    .optional("git_commit", FieldType::String, "Git commit SHA")
    .optional("details", FieldType::String, "Additional details")
    .optional("correlation_id", FieldType::String, "Correlation ID")
}

/// Returns the schema for bundle events.
#[must_use]
pub fn bundle_event_schema() -> EventSchema {
    EventSchema::new(
        "BundleEvent",
        "1.0.0",
        "bundle.*",
        "Events related to bundle operations",
    )
    .required("id", FieldType::Uuid, "Unique event identifier")
    .required("timestamp", FieldType::Timestamp, "Event timestamp")
    .required("event_type", FieldType::String, "Type of bundle event")
    .required("service", FieldType::String, "Service name")
    .required("version", FieldType::String, "Bundle version")
    .required("actor", FieldType::String, "Actor who triggered the event")
    .required("outcome", FieldType::String, "Event outcome")
    .optional("checksum", FieldType::String, "Bundle checksum")
    .optional("size_bytes", FieldType::Integer, "Bundle size in bytes")
    .optional("details", FieldType::String, "Additional details")
    .optional("correlation_id", FieldType::String, "Correlation ID")
}

/// Returns the schema for distribution events.
#[must_use]
pub fn distribution_event_schema() -> EventSchema {
    EventSchema::new(
        "DistributionEvent",
        "1.0.0",
        "distribution.*",
        "Events related to bundle distribution",
    )
    .required("id", FieldType::Uuid, "Unique event identifier")
    .required("timestamp", FieldType::Timestamp, "Event timestamp")
    .required("event_type", FieldType::String, "Type of distribution event")
    .required("service", FieldType::String, "Service name")
    .required("version", FieldType::String, "Bundle version")
    .required("outcome", FieldType::String, "Event outcome")
    .optional("instance", FieldType::String, "Target instance endpoint")
    .optional("instance_count", FieldType::Integer, "Number of instances")
    .optional("strategy", FieldType::String, "Deployment strategy")
    .optional("details", FieldType::String, "Additional details")
    .optional("correlation_id", FieldType::String, "Correlation ID")
}

/// Returns the schema for authorization events.
#[must_use]
pub fn authorization_event_schema() -> EventSchema {
    EventSchema::new(
        "AuthorizationEvent",
        "1.0.0",
        "authorization.*",
        "Events related to authorization decisions",
    )
    .required("id", FieldType::Uuid, "Unique event identifier")
    .required("timestamp", FieldType::Timestamp, "Event timestamp")
    .required("service", FieldType::String, "Service name")
    .required("operation_id", FieldType::String, "Operation being authorized")
    .required("caller_type", FieldType::String, "Type of caller")
    .required("allowed", FieldType::Boolean, "Authorization decision")
    .optional("caller_id", FieldType::String, "Caller identifier")
    .optional("reason", FieldType::String, "Reason for decision")
    .optional("policy_version", FieldType::String, "Policy version used")
    .optional("evaluation_time_ns", FieldType::Integer, "Evaluation time")
    .optional("correlation_id", FieldType::String, "Correlation ID")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_metadata_default() {
        let metadata = AuditMetadata::new();

        assert_eq!(metadata.schema_version, CURRENT_SCHEMA_VERSION);
        assert_eq!(metadata.source, "eunomia");
        assert!(metadata.environment.is_none());
        assert!(metadata.tags.is_empty());
    }

    #[test]
    fn test_audit_metadata_builder() {
        let metadata = AuditMetadata::with_source("eunomia-cli")
            .environment("production")
            .hostname("host1.example.com")
            .pid(12345)
            .tag("team:platform");

        assert_eq!(metadata.source, "eunomia-cli");
        assert_eq!(metadata.environment, Some("production".to_string()));
        assert_eq!(metadata.hostname, Some("host1.example.com".to_string()));
        assert_eq!(metadata.pid, Some(12345));
        assert_eq!(metadata.tags, vec!["team:platform"]);
    }

    #[test]
    fn test_event_schema_creation() {
        let schema = EventSchema::new(
            "TestEvent",
            "1.0.0",
            "test.*",
            "Test events",
        )
        .required("id", FieldType::Uuid, "Event ID")
        .optional("details", FieldType::String, "Details");

        assert_eq!(schema.name, "TestEvent");
        assert_eq!(schema.version, "1.0.0");
        assert_eq!(schema.required_fields.len(), 1);
        assert_eq!(schema.optional_fields.len(), 1);
    }

    #[test]
    fn test_field_definition() {
        let field = FieldDefinition::new("test_field", FieldType::String, "A test field")
            .with_example("example_value");

        assert_eq!(field.name, "test_field");
        assert_eq!(field.field_type, FieldType::String);
        assert_eq!(field.example, Some("example_value".to_string()));
    }

    #[test]
    fn test_policy_event_schema() {
        let schema = policy_event_schema();

        assert_eq!(schema.name, "PolicyEvent");
        assert!(!schema.required_fields.is_empty());
        assert!(schema.required_fields.iter().any(|f| f.name == "id"));
        assert!(schema.required_fields.iter().any(|f| f.name == "service"));
    }

    #[test]
    fn test_bundle_event_schema() {
        let schema = bundle_event_schema();

        assert_eq!(schema.name, "BundleEvent");
        assert!(schema.optional_fields.iter().any(|f| f.name == "checksum"));
    }

    #[test]
    fn test_distribution_event_schema() {
        let schema = distribution_event_schema();

        assert_eq!(schema.name, "DistributionEvent");
        assert!(schema.optional_fields.iter().any(|f| f.name == "strategy"));
    }

    #[test]
    fn test_authorization_event_schema() {
        let schema = authorization_event_schema();

        assert_eq!(schema.name, "AuthorizationEvent");
        assert!(schema.required_fields.iter().any(|f| f.name == "allowed"));
    }

    #[test]
    fn test_metadata_serialization() {
        let metadata = AuditMetadata::with_source("test")
            .environment("dev");

        let json = serde_json::to_string(&metadata).unwrap();
        assert!(json.contains("\"source\":\"test\""));
        assert!(json.contains("\"environment\":\"dev\""));

        let deserialized: AuditMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.source, metadata.source);
    }
}
