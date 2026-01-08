//! Compiler metrics for policy compilation operations.

use prometheus::{Counter, CounterVec, HistogramOpts, HistogramVec, Opts, Registry};
use tracing::warn;

/// Metrics for the policy compiler.
///
/// Tracks compilation success/failure rates and duration.
#[derive(Clone)]
pub struct CompilerMetrics {
    /// Total compilations counter (labeled by service and status).
    compilations_total: CounterVec,

    /// Compilation duration histogram (labeled by service).
    compilation_duration_ms: HistogramVec,

    /// Total policy files processed.
    policies_processed_total: Counter,

    /// Bundle size histogram (bytes).
    bundle_size_bytes: HistogramVec,
}

impl CompilerMetrics {
    /// Creates new compiler metrics and registers them with the given registry.
    pub(crate) fn new(registry: &Registry) -> Self {
        let compilations_total = CounterVec::new(
            Opts::new(
                "eunomia_compiler_compilations_total",
                "Total number of policy compilations",
            ),
            &["service", "status"],
        )
        .expect("metric can be created");

        let compilation_duration_ms = HistogramVec::new(
            HistogramOpts::new(
                "eunomia_compiler_compilation_duration_milliseconds",
                "Policy compilation duration in milliseconds",
            )
            .buckets(vec![10.0, 25.0, 50.0, 100.0, 250.0, 500.0, 1000.0, 2500.0]),
            &["service"],
        )
        .expect("metric can be created");

        let policies_processed_total = Counter::new(
            "eunomia_compiler_policies_processed_total",
            "Total number of policy files processed",
        )
        .expect("metric can be created");

        let bundle_size_bytes = HistogramVec::new(
            HistogramOpts::new(
                "eunomia_compiler_bundle_size_bytes",
                "Compiled bundle size in bytes",
            )
            .buckets(vec![
                1024.0,
                10240.0,
                102400.0,
                1048576.0,
                10485760.0,
                104857600.0,
            ]),
            &["service"],
        )
        .expect("metric can be created");

        // Register metrics
        if let Err(e) = registry.register(Box::new(compilations_total.clone())) {
            warn!("Failed to register compilations_total: {e}");
        }
        if let Err(e) = registry.register(Box::new(compilation_duration_ms.clone())) {
            warn!("Failed to register compilation_duration_ms: {e}");
        }
        if let Err(e) = registry.register(Box::new(policies_processed_total.clone())) {
            warn!("Failed to register policies_processed_total: {e}");
        }
        if let Err(e) = registry.register(Box::new(bundle_size_bytes.clone())) {
            warn!("Failed to register bundle_size_bytes: {e}");
        }

        Self {
            compilations_total,
            compilation_duration_ms,
            policies_processed_total,
            bundle_size_bytes,
        }
    }

    /// Records a compilation attempt.
    ///
    /// # Arguments
    ///
    /// * `service` - Service name being compiled
    /// * `success` - Whether the compilation succeeded
    /// * `duration_ms` - Compilation duration in milliseconds
    pub fn record_compilation(&self, service: &str, success: bool, duration_ms: u64) {
        let status = if success { "success" } else { "failure" };
        self.compilations_total
            .with_label_values(&[service, status])
            .inc();
        self.compilation_duration_ms
            .with_label_values(&[service])
            .observe(duration_ms as f64);
    }

    /// Records policy files processed.
    pub fn record_policies_processed(&self, count: u64) {
        self.policies_processed_total.inc_by(count as f64);
    }

    /// Records a bundle size.
    pub fn record_bundle_size(&self, service: &str, size_bytes: u64) {
        self.bundle_size_bytes
            .with_label_values(&[service])
            .observe(size_bytes as f64);
    }

    /// Returns the total compilation count for a service and status.
    #[must_use]
    pub fn get_compilation_count(&self, service: &str, success: bool) -> f64 {
        let status = if success { "success" } else { "failure" };
        self.compilations_total
            .with_label_values(&[service, status])
            .get()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_registry() -> (Registry, CompilerMetrics) {
        let registry = Registry::new();
        let metrics = CompilerMetrics::new(&registry);
        (registry, metrics)
    }

    #[test]
    fn test_record_successful_compilation() {
        let (_, metrics) = create_test_registry();

        metrics.record_compilation("test-service", true, 150);

        assert_eq!(metrics.get_compilation_count("test-service", true), 1.0);
        assert_eq!(metrics.get_compilation_count("test-service", false), 0.0);
    }

    #[test]
    fn test_record_failed_compilation() {
        let (_, metrics) = create_test_registry();

        metrics.record_compilation("test-service", false, 50);

        assert_eq!(metrics.get_compilation_count("test-service", true), 0.0);
        assert_eq!(metrics.get_compilation_count("test-service", false), 1.0);
    }

    #[test]
    fn test_multiple_compilations() {
        let (_, metrics) = create_test_registry();

        metrics.record_compilation("svc1", true, 100);
        metrics.record_compilation("svc1", true, 110);
        metrics.record_compilation("svc1", false, 50);
        metrics.record_compilation("svc2", true, 200);

        assert_eq!(metrics.get_compilation_count("svc1", true), 2.0);
        assert_eq!(metrics.get_compilation_count("svc1", false), 1.0);
        assert_eq!(metrics.get_compilation_count("svc2", true), 1.0);
    }

    #[test]
    fn test_record_policies_processed() {
        let (_, metrics) = create_test_registry();

        metrics.record_policies_processed(5);
        metrics.record_policies_processed(3);

        // Total should be 8 (counter only goes up)
        assert_eq!(metrics.policies_processed_total.get(), 8.0);
    }

    #[test]
    fn test_record_bundle_size() {
        let (_, metrics) = create_test_registry();

        metrics.record_bundle_size("test-service", 1024);
        metrics.record_bundle_size("test-service", 2048);

        // Histogram records observations, we can verify it doesn't panic
    }
}
