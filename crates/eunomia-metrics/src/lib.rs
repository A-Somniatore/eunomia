//! OpenTelemetry metrics for the Eunomia authorization platform.
//!
//! This crate provides metrics collection and export for monitoring:
//! - Policy compilation (success/failure counts, duration)
//! - Bundle operations (publish count, size, duration)
//! - Distribution events (push success/failure, latency)
//! - Cache operations (hits, misses, evictions)
//!
//! # Metrics Naming Convention
//!
//! All metrics follow the format: `eunomia_<component>_<metric>_<unit>`
//!
//! Components:
//! - `compiler` - Policy compilation metrics
//! - `bundle` - Bundle creation and publishing metrics
//! - `distributor` - Policy distribution metrics
//! - `registry` - Registry operations metrics
//! - `cache` - Caching metrics
//!
//! # Example
//!
//! ```rust,no_run
//! use eunomia_metrics::{MetricsRegistry, CompilerMetrics};
//!
//! // Initialize global metrics registry
//! let registry = MetricsRegistry::global();
//!
//! // Record a compilation
//! registry.compiler().record_compilation("users-service", true, 150);
//!
//! // Get Prometheus metrics
//! let metrics_output = registry.prometheus_output();
//! ```
//!
//! # Prometheus Integration
//!
//! The metrics can be exposed via HTTP endpoint:
//!
//! ```rust,no_run
//! use eunomia_metrics::{serve_metrics, MetricsServerConfig};
//!
//! #[tokio::main]
//! async fn main() {
//!     // Start metrics server with default config (port 9090)
//!     let config = MetricsServerConfig::default();
//!     let handle = serve_metrics(config).await.unwrap();
//!     
//!     // Later, shut down
//!     handle.shutdown();
//! }
//! ```

mod compiler;
mod distributor;
mod registry;
mod server;

pub use compiler::CompilerMetrics;
pub use distributor::DistributorMetrics;
pub use registry::RegistryMetrics;
pub use server::{serve_metrics, MetricsServerConfig, MetricsServerHandle};

use once_cell::sync::Lazy;
use parking_lot::RwLock;
use prometheus::{Encoder, Registry, TextEncoder};
use std::sync::Arc;

/// Global metrics registry instance.
static GLOBAL_REGISTRY: Lazy<MetricsRegistry> = Lazy::new(MetricsRegistry::new);

/// Central metrics registry for all Eunomia metrics.
///
/// This registry manages all metric collectors and provides access to
/// component-specific metrics.
#[derive(Clone)]
pub struct MetricsRegistry {
    prometheus_registry: Arc<RwLock<Registry>>,
    compiler: CompilerMetrics,
    distributor: DistributorMetrics,
    registry: RegistryMetrics,
}

impl MetricsRegistry {
    /// Creates a new metrics registry.
    #[must_use]
    pub fn new() -> Self {
        let prometheus_registry = Arc::new(RwLock::new(Registry::new()));

        let compiler = CompilerMetrics::new(&prometheus_registry.read());
        let distributor = DistributorMetrics::new(&prometheus_registry.read());
        let registry = RegistryMetrics::new(&prometheus_registry.read());

        Self {
            prometheus_registry,
            compiler,
            distributor,
            registry,
        }
    }

    /// Returns the global metrics registry instance.
    #[must_use]
    pub fn global() -> &'static Self {
        &GLOBAL_REGISTRY
    }

    /// Returns compiler metrics collector.
    #[must_use]
    pub fn compiler(&self) -> &CompilerMetrics {
        &self.compiler
    }

    /// Returns distributor metrics collector.
    #[must_use]
    pub fn distributor(&self) -> &DistributorMetrics {
        &self.distributor
    }

    /// Returns registry metrics collector.
    #[must_use]
    pub fn registry(&self) -> &RegistryMetrics {
        &self.registry
    }

    /// Generates Prometheus-format metrics output.
    ///
    /// # Errors
    ///
    /// Returns an error if encoding fails.
    pub fn prometheus_output(&self) -> Result<String, MetricsError> {
        let encoder = TextEncoder::new();
        let mut buffer = Vec::new();

        let metric_families = self.prometheus_registry.read().gather();
        encoder.encode(&metric_families, &mut buffer)?;

        String::from_utf8(buffer).map_err(|e| MetricsError::Encoding(e.to_string()))
    }
}

impl Default for MetricsRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Errors that can occur during metrics operations.
#[derive(Debug, thiserror::Error)]
pub enum MetricsError {
    /// Prometheus encoding error.
    #[error("Failed to encode metrics: {0}")]
    Encoding(String),

    /// Server binding error.
    #[error("Failed to bind metrics server: {0}")]
    ServerBind(String),

    /// Metric registration error.
    #[error("Failed to register metric: {0}")]
    Registration(String),
}

impl From<prometheus::Error> for MetricsError {
    fn from(err: prometheus::Error) -> Self {
        Self::Registration(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_registry_creation() {
        let registry = MetricsRegistry::new();

        // Verify we can access all metric collectors
        let _ = registry.compiler();
        let _ = registry.distributor();
        let _ = registry.registry();
    }

    #[test]
    fn test_global_registry() {
        let registry = MetricsRegistry::global();

        // Record a metric
        registry
            .compiler()
            .record_compilation("test-service", true, 100);

        // Verify output can be generated
        let output = registry.prometheus_output().unwrap();
        assert!(output.contains("eunomia_compiler"));
    }

    #[test]
    fn test_prometheus_output_format() {
        let registry = MetricsRegistry::new();

        // Record some metrics
        registry.compiler().record_compilation("svc1", true, 50);
        registry.compiler().record_compilation("svc1", false, 100);
        registry
            .distributor()
            .record_push("svc1", "v1.0.0", true, 200);

        let output = registry.prometheus_output().unwrap();

        // Check for expected metric names
        assert!(output.contains("eunomia_compiler_compilations_total"));
        assert!(output.contains("eunomia_distributor_pushes_total"));
    }
}
