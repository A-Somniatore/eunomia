//! Audit logging for the Eunomia authorization platform.
//!
//! This crate provides audit logging capabilities for tracking:
//! - Policy lifecycle events (creation, update, deletion)
//! - Bundle operations (compilation, signing, publishing)
//! - Distribution events (push, rollback)
//! - Authorization decisions (allow/deny with reasons)
//!
//! # Features
//!
//! - Structured audit events with consistent schema
//! - Multiple output backends (stdout, file, custom)
//! - Correlation IDs for request tracing
//! - Tamper-evident event signing (optional)
//!
//! # Example
//!
//! ```rust
//! use eunomia_audit::{AuditLogger, PolicyEvent, TracingBackend};
//! use std::sync::Arc;
//!
//! // Create a logger with tracing backend
//! let logger = AuditLogger::builder()
//!     .with_backend(Arc::new(TracingBackend::new()))
//!     .build();
//!
//! // Log a policy event
//! let event = PolicyEvent::created("users-service", "1.0.0", "user@example.com");
//! logger.log(&event).unwrap();
//! ```

mod event;
mod logger;
mod schema;

pub use event::{
    AuditEvent, AuthorizationEvent, BundleEvent, DistributionEvent, EventOutcome, EventSeverity,
    PolicyEvent,
};
pub use logger::{AuditLogger, InMemoryBackend, LoggerBackend, LoggerError, TracingBackend};
pub use schema::{
    authorization_event_schema, bundle_event_schema, distribution_event_schema,
    policy_event_schema, AuditMetadata, EventSchema, FieldDefinition, FieldType,
    CURRENT_SCHEMA_VERSION,
};
