//! Distributor metrics for policy push operations.

use prometheus::{CounterVec, Histogram, HistogramOpts, HistogramVec, Opts, Registry};
use tracing::warn;

/// Metrics for the policy distributor.
///
/// Tracks push success/failure rates, latency, and instance health.
#[derive(Clone)]
pub struct DistributorMetrics {
    /// Total pushes counter (labeled by service, version, status).
    pushes_total: CounterVec,

    /// Push duration histogram (labeled by service).
    push_duration_ms: HistogramVec,

    /// Push batch size histogram.
    push_batch_size: Histogram,

    /// Rollbacks counter (labeled by service, status).
    rollbacks_total: CounterVec,

    /// Rollback duration histogram.
    rollback_duration_ms: HistogramVec,

    /// Active deployments gauge (labeled by service).
    active_deployments: CounterVec,

    /// Instance health check results (labeled by instance, status).
    health_checks_total: CounterVec,
}

impl DistributorMetrics {
    /// Creates new distributor metrics and registers them with the given registry.
    pub(crate) fn new(registry: &Registry) -> Self {
        let pushes_total = CounterVec::new(
            Opts::new(
                "eunomia_distributor_pushes_total",
                "Total number of policy pushes",
            ),
            &["service", "version", "status"],
        )
        .expect("metric can be created");

        let push_duration_ms = HistogramVec::new(
            HistogramOpts::new(
                "eunomia_distributor_push_duration_milliseconds",
                "Policy push duration in milliseconds",
            )
            .buckets(vec![
                50.0, 100.0, 250.0, 500.0, 1000.0, 2500.0, 5000.0, 10000.0,
            ]),
            &["service"],
        )
        .expect("metric can be created");

        let push_batch_size = Histogram::with_opts(
            HistogramOpts::new(
                "eunomia_distributor_push_batch_size",
                "Number of instances per push batch",
            )
            .buckets(vec![1.0, 5.0, 10.0, 25.0, 50.0, 100.0]),
        )
        .expect("metric can be created");

        let rollbacks_total = CounterVec::new(
            Opts::new(
                "eunomia_distributor_rollbacks_total",
                "Total number of policy rollbacks",
            ),
            &["service", "status"],
        )
        .expect("metric can be created");

        let rollback_duration_ms = HistogramVec::new(
            HistogramOpts::new(
                "eunomia_distributor_rollback_duration_milliseconds",
                "Policy rollback duration in milliseconds",
            )
            .buckets(vec![
                50.0, 100.0, 250.0, 500.0, 1000.0, 2500.0, 5000.0, 10000.0,
            ]),
            &["service"],
        )
        .expect("metric can be created");

        let active_deployments = CounterVec::new(
            Opts::new(
                "eunomia_distributor_deployments_total",
                "Total number of deployment operations",
            ),
            &["service", "strategy"],
        )
        .expect("metric can be created");

        let health_checks_total = CounterVec::new(
            Opts::new(
                "eunomia_distributor_health_checks_total",
                "Total number of instance health checks",
            ),
            &["instance", "status"],
        )
        .expect("metric can be created");

        // Register metrics
        if let Err(e) = registry.register(Box::new(pushes_total.clone())) {
            warn!("Failed to register pushes_total: {e}");
        }
        if let Err(e) = registry.register(Box::new(push_duration_ms.clone())) {
            warn!("Failed to register push_duration_ms: {e}");
        }
        if let Err(e) = registry.register(Box::new(push_batch_size.clone())) {
            warn!("Failed to register push_batch_size: {e}");
        }
        if let Err(e) = registry.register(Box::new(rollbacks_total.clone())) {
            warn!("Failed to register rollbacks_total: {e}");
        }
        if let Err(e) = registry.register(Box::new(rollback_duration_ms.clone())) {
            warn!("Failed to register rollback_duration_ms: {e}");
        }
        if let Err(e) = registry.register(Box::new(active_deployments.clone())) {
            warn!("Failed to register active_deployments: {e}");
        }
        if let Err(e) = registry.register(Box::new(health_checks_total.clone())) {
            warn!("Failed to register health_checks_total: {e}");
        }

        Self {
            pushes_total,
            push_duration_ms,
            push_batch_size,
            rollbacks_total,
            rollback_duration_ms,
            active_deployments,
            health_checks_total,
        }
    }

    /// Records a policy push attempt.
    ///
    /// # Arguments
    ///
    /// * `service` - Service name
    /// * `version` - Policy version
    /// * `success` - Whether the push succeeded
    /// * `duration_ms` - Push duration in milliseconds
    pub fn record_push(&self, service: &str, version: &str, success: bool, duration_ms: u64) {
        let status = if success { "success" } else { "failure" };
        self.pushes_total
            .with_label_values(&[service, version, status])
            .inc();
        self.push_duration_ms
            .with_label_values(&[service])
            .observe(duration_ms as f64);
    }

    /// Records a batch push size.
    pub fn record_batch_size(&self, size: usize) {
        self.push_batch_size.observe(size as f64);
    }

    /// Records a rollback attempt.
    pub fn record_rollback(&self, service: &str, success: bool, duration_ms: u64) {
        let status = if success { "success" } else { "failure" };
        self.rollbacks_total
            .with_label_values(&[service, status])
            .inc();
        self.rollback_duration_ms
            .with_label_values(&[service])
            .observe(duration_ms as f64);
    }

    /// Records a deployment operation.
    pub fn record_deployment(&self, service: &str, strategy: &str) {
        self.active_deployments
            .with_label_values(&[service, strategy])
            .inc();
    }

    /// Records a health check result.
    pub fn record_health_check(&self, instance: &str, healthy: bool) {
        let status = if healthy { "healthy" } else { "unhealthy" };
        self.health_checks_total
            .with_label_values(&[instance, status])
            .inc();
    }

    /// Returns the total push count for a service, version, and status.
    #[must_use]
    pub fn get_push_count(&self, service: &str, version: &str, success: bool) -> f64 {
        let status = if success { "success" } else { "failure" };
        self.pushes_total
            .with_label_values(&[service, version, status])
            .get()
    }

    /// Returns the total rollback count for a service and status.
    #[must_use]
    pub fn get_rollback_count(&self, service: &str, success: bool) -> f64 {
        let status = if success { "success" } else { "failure" };
        self.rollbacks_total
            .with_label_values(&[service, status])
            .get()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_registry() -> (Registry, DistributorMetrics) {
        let registry = Registry::new();
        let metrics = DistributorMetrics::new(&registry);
        (registry, metrics)
    }

    #[test]
    fn test_record_successful_push() {
        let (_, metrics) = create_test_registry();

        metrics.record_push("test-service", "v1.0.0", true, 150);

        assert_eq!(
            metrics.get_push_count("test-service", "v1.0.0", true),
            1.0
        );
        assert_eq!(
            metrics.get_push_count("test-service", "v1.0.0", false),
            0.0
        );
    }

    #[test]
    fn test_record_failed_push() {
        let (_, metrics) = create_test_registry();

        metrics.record_push("test-service", "v1.0.0", false, 50);

        assert_eq!(
            metrics.get_push_count("test-service", "v1.0.0", false),
            1.0
        );
    }

    #[test]
    fn test_record_rollback() {
        let (_, metrics) = create_test_registry();

        metrics.record_rollback("test-service", true, 200);
        metrics.record_rollback("test-service", false, 100);

        assert_eq!(metrics.get_rollback_count("test-service", true), 1.0);
        assert_eq!(metrics.get_rollback_count("test-service", false), 1.0);
    }

    #[test]
    fn test_record_batch_size() {
        let (_, metrics) = create_test_registry();

        metrics.record_batch_size(5);
        metrics.record_batch_size(10);
        // Histogram records observations, verify it doesn't panic
    }

    #[test]
    fn test_record_health_check() {
        let (_, metrics) = create_test_registry();

        metrics.record_health_check("instance-1", true);
        metrics.record_health_check("instance-1", false);
        metrics.record_health_check("instance-2", true);
        // Counter increments, verify it doesn't panic
    }

    #[test]
    fn test_record_deployment() {
        let (_, metrics) = create_test_registry();

        metrics.record_deployment("test-service", "rolling");
        metrics.record_deployment("test-service", "canary");
        // Counter increments, verify it doesn't panic
    }
}
