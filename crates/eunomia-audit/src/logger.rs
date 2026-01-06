//! Audit logger implementation.

use crate::event::AuditEvent;
use std::fmt::Debug;
use std::sync::Arc;
use tracing::{debug, error, info, warn};

/// Backend trait for audit log storage.
pub trait LoggerBackend: Send + Sync + Debug {
    /// Logs an audit event.
    ///
    /// # Errors
    ///
    /// Returns an error if the event cannot be logged.
    fn log(&self, event_json: &str) -> Result<(), LoggerError>;

    /// Flushes any buffered events.
    ///
    /// # Errors
    ///
    /// Returns an error if the flush operation fails.
    fn flush(&self) -> Result<(), LoggerError>;

    /// Returns the backend name for identification.
    fn name(&self) -> &'static str;
}

/// Errors that can occur during audit logging.
#[derive(Debug, thiserror::Error)]
pub enum LoggerError {
    /// Serialization error
    #[error("Failed to serialize event: {0}")]
    Serialization(#[from] serde_json::Error),

    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Backend-specific error
    #[error("Backend error: {0}")]
    Backend(String),
}

/// Audit logger that sends events to configured backends.
#[derive(Debug)]
pub struct AuditLogger {
    /// Backends to send events to
    backends: Vec<Arc<dyn LoggerBackend>>,

    /// Whether logging is enabled
    enabled: bool,

    /// Log level threshold (events below this level are filtered)
    min_severity: crate::event::EventSeverity,
}

impl Default for AuditLogger {
    fn default() -> Self {
        Self::new()
    }
}

impl AuditLogger {
    /// Creates a new audit logger with no backends.
    #[must_use]
    pub fn new() -> Self {
        Self {
            backends: Vec::new(),
            enabled: true,
            min_severity: crate::event::EventSeverity::Info,
        }
    }

    /// Creates a builder for configuring the logger.
    #[must_use]
    pub fn builder() -> AuditLoggerBuilder {
        AuditLoggerBuilder::new()
    }

    /// Adds a backend to the logger.
    pub fn add_backend(&mut self, backend: Arc<dyn LoggerBackend>) {
        self.backends.push(backend);
    }

    /// Enables or disables the logger.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Sets the minimum severity level for logging.
    pub fn set_min_severity(&mut self, severity: crate::event::EventSeverity) {
        self.min_severity = severity;
    }

    /// Logs an audit event to all configured backends.
    ///
    /// # Errors
    ///
    /// Returns an error if the event cannot be serialized.
    /// Backend errors are logged but do not cause this method to fail.
    pub fn log<E: AuditEvent>(&self, event: &E) -> Result<(), LoggerError> {
        if !self.enabled {
            debug!("Audit logging disabled, skipping event");
            return Ok(());
        }

        if !self.should_log_severity(event.severity()) {
            debug!(
                "Event severity {:?} below threshold {:?}, skipping",
                event.severity(),
                self.min_severity
            );
            return Ok(());
        }

        let json = serde_json::to_string(event)?;

        for backend in &self.backends {
            if let Err(e) = backend.log(&json) {
                error!(
                    "Failed to log event to backend {}: {}",
                    backend.name(),
                    e
                );
            }
        }

        Ok(())
    }

    /// Flushes all backends.
    ///
    /// # Errors
    ///
    /// Returns an error if any backend fails to flush.
    pub fn flush(&self) -> Result<(), LoggerError> {
        for backend in &self.backends {
            backend.flush()?;
        }
        Ok(())
    }

    /// Returns the number of configured backends.
    #[must_use]
    pub fn backend_count(&self) -> usize {
        self.backends.len()
    }

    fn should_log_severity(&self, severity: crate::event::EventSeverity) -> bool {
        use crate::event::EventSeverity;

        let severity_level = match severity {
            EventSeverity::Info => 0,
            EventSeverity::Warning => 1,
            EventSeverity::Error => 2,
            EventSeverity::Critical => 3,
        };

        let min_level = match self.min_severity {
            EventSeverity::Info => 0,
            EventSeverity::Warning => 1,
            EventSeverity::Error => 2,
            EventSeverity::Critical => 3,
        };

        severity_level >= min_level
    }
}

/// Builder for configuring an audit logger.
#[derive(Debug, Default)]
pub struct AuditLoggerBuilder {
    backends: Vec<Arc<dyn LoggerBackend>>,
    enabled: bool,
    min_severity: crate::event::EventSeverity,
}

impl AuditLoggerBuilder {
    /// Creates a new builder.
    #[must_use]
    pub fn new() -> Self {
        Self {
            backends: Vec::new(),
            enabled: true,
            min_severity: crate::event::EventSeverity::Info,
        }
    }

    /// Adds a backend to the logger.
    #[must_use]
    pub fn with_backend(mut self, backend: Arc<dyn LoggerBackend>) -> Self {
        self.backends.push(backend);
        self
    }

    /// Enables or disables the logger.
    #[must_use]
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Sets the minimum severity level.
    #[must_use]
    pub fn min_severity(mut self, severity: crate::event::EventSeverity) -> Self {
        self.min_severity = severity;
        self
    }

    /// Builds the audit logger.
    #[must_use]
    pub fn build(self) -> AuditLogger {
        AuditLogger {
            backends: self.backends,
            enabled: self.enabled,
            min_severity: self.min_severity,
        }
    }
}

/// Tracing-based backend that logs events via tracing macros.
#[derive(Debug, Default)]
pub struct TracingBackend;

impl TracingBackend {
    /// Creates a new tracing backend.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl LoggerBackend for TracingBackend {
    fn log(&self, event_json: &str) -> Result<(), LoggerError> {
        // Parse to get severity for appropriate log level
        let value: serde_json::Value = serde_json::from_str(event_json)?;

        let outcome = value
            .get("outcome")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        match outcome {
            "failure" | "denied" => warn!(audit_event = %event_json, "Audit event"),
            _ => info!(audit_event = %event_json, "Audit event"),
        }

        Ok(())
    }

    fn flush(&self) -> Result<(), LoggerError> {
        // Tracing backend doesn't buffer
        Ok(())
    }

    fn name(&self) -> &'static str {
        "tracing"
    }
}

/// In-memory backend for testing.
#[derive(Debug)]
pub struct InMemoryBackend {
    events: std::sync::Mutex<Vec<String>>,
}

impl Default for InMemoryBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryBackend {
    /// Creates a new in-memory backend.
    #[must_use]
    pub fn new() -> Self {
        Self {
            events: std::sync::Mutex::new(Vec::new()),
        }
    }

    /// Returns all logged events.
    #[must_use]
    pub fn events(&self) -> Vec<String> {
        self.events.lock().unwrap().clone()
    }

    /// Clears all logged events.
    pub fn clear(&self) {
        self.events.lock().unwrap().clear();
    }
}

impl LoggerBackend for InMemoryBackend {
    fn log(&self, event_json: &str) -> Result<(), LoggerError> {
        self.events.lock().unwrap().push(event_json.to_string());
        Ok(())
    }

    fn flush(&self) -> Result<(), LoggerError> {
        Ok(())
    }

    fn name(&self) -> &'static str {
        "in_memory"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::PolicyEvent;

    #[test]
    fn test_logger_with_in_memory_backend() {
        let backend = Arc::new(InMemoryBackend::new());
        let logger = AuditLogger::builder()
            .with_backend(backend.clone())
            .build();

        let event = PolicyEvent::created("users-service", "1.0.0", "user@example.com");
        logger.log(&event).unwrap();

        let events = backend.events();
        assert_eq!(events.len(), 1);
        assert!(events[0].contains("users-service"));
    }

    #[test]
    fn test_logger_disabled() {
        let backend = Arc::new(InMemoryBackend::new());
        let logger = AuditLogger::builder()
            .with_backend(backend.clone())
            .enabled(false)
            .build();

        let event = PolicyEvent::created("users-service", "1.0.0", "user@example.com");
        logger.log(&event).unwrap();

        let events = backend.events();
        assert!(events.is_empty());
    }

    #[test]
    fn test_logger_severity_filtering() {
        let backend = Arc::new(InMemoryBackend::new());
        let logger = AuditLogger::builder()
            .with_backend(backend.clone())
            .min_severity(crate::event::EventSeverity::Warning)
            .build();

        // Info event should be filtered
        let info_event = PolicyEvent::created("users-service", "1.0.0", "user@example.com");
        logger.log(&info_event).unwrap();

        // Warning event should be logged
        let warning_event = PolicyEvent::tested("users-service", "1.0.0", 8, 2);
        logger.log(&warning_event).unwrap();

        let events = backend.events();
        assert_eq!(events.len(), 1);
        assert!(events[0].contains("\"outcome\":\"failure\""));
    }

    #[test]
    fn test_multiple_backends() {
        let backend1 = Arc::new(InMemoryBackend::new());
        let backend2 = Arc::new(InMemoryBackend::new());

        let logger = AuditLogger::builder()
            .with_backend(backend1.clone())
            .with_backend(backend2.clone())
            .build();

        let event = PolicyEvent::created("users-service", "1.0.0", "user@example.com");
        logger.log(&event).unwrap();

        assert_eq!(backend1.events().len(), 1);
        assert_eq!(backend2.events().len(), 1);
    }

    #[test]
    fn test_backend_count() {
        let backend1 = Arc::new(InMemoryBackend::new());
        let backend2 = Arc::new(InMemoryBackend::new());

        let logger = AuditLogger::builder()
            .with_backend(backend1)
            .with_backend(backend2)
            .build();

        assert_eq!(logger.backend_count(), 2);
    }

    #[test]
    fn test_tracing_backend() {
        let backend = TracingBackend::new();
        let json = r#"{"outcome":"success","event_type":"created"}"#;

        // Should not panic
        backend.log(json).unwrap();
        backend.flush().unwrap();
        assert_eq!(backend.name(), "tracing");
    }

    #[test]
    fn test_in_memory_backend_clear() {
        let backend = InMemoryBackend::new();
        backend.log(r#"{"event":"test"}"#).unwrap();
        assert_eq!(backend.events().len(), 1);

        backend.clear();
        assert!(backend.events().is_empty());
    }
}
