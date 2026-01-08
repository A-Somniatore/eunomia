//! Registry metrics for bundle operations.

use prometheus::{CounterVec, Histogram, HistogramOpts, HistogramVec, Opts, Registry};
use tracing::warn;

/// Metrics for the bundle registry client.
///
/// Tracks publish/fetch operations, cache performance, and bundle sizes.
#[derive(Clone)]
pub struct RegistryMetrics {
    /// Total publish operations (labeled by service, status).
    publishes_total: CounterVec,

    /// Publish duration histogram (labeled by service).
    publish_duration_ms: HistogramVec,

    /// Total fetch operations (labeled by service, status).
    fetches_total: CounterVec,

    /// Fetch duration histogram (labeled by service).
    fetch_duration_ms: HistogramVec,

    /// Cache hits/misses (labeled by operation type).
    cache_operations_total: CounterVec,

    /// Cache size bytes.
    cache_size_bytes: Histogram,

    /// Bundle download size bytes.
    bundle_download_bytes: HistogramVec,
}

impl RegistryMetrics {
    /// Creates new registry metrics and registers them with the given registry.
    pub(crate) fn new(registry: &Registry) -> Self {
        let publishes_total = CounterVec::new(
            Opts::new(
                "eunomia_registry_publishes_total",
                "Total number of bundle publish operations",
            ),
            &["service", "status"],
        )
        .expect("metric can be created");

        let publish_duration_ms = HistogramVec::new(
            HistogramOpts::new(
                "eunomia_registry_publish_duration_milliseconds",
                "Bundle publish duration in milliseconds",
            )
            .buckets(vec![
                100.0, 250.0, 500.0, 1000.0, 2500.0, 5000.0, 10000.0, 30000.0,
            ]),
            &["service"],
        )
        .expect("metric can be created");

        let fetches_total = CounterVec::new(
            Opts::new(
                "eunomia_registry_fetches_total",
                "Total number of bundle fetch operations",
            ),
            &["service", "status"],
        )
        .expect("metric can be created");

        let fetch_duration_ms = HistogramVec::new(
            HistogramOpts::new(
                "eunomia_registry_fetch_duration_milliseconds",
                "Bundle fetch duration in milliseconds",
            )
            .buckets(vec![
                50.0, 100.0, 250.0, 500.0, 1000.0, 2500.0, 5000.0, 10000.0,
            ]),
            &["service"],
        )
        .expect("metric can be created");

        let cache_operations_total = CounterVec::new(
            Opts::new(
                "eunomia_registry_cache_operations_total",
                "Total number of cache operations",
            ),
            &["operation"], // hit, miss, eviction
        )
        .expect("metric can be created");

        let cache_size_bytes = Histogram::with_opts(
            HistogramOpts::new(
                "eunomia_registry_cache_size_bytes",
                "Current cache size in bytes",
            )
            .buckets(vec![
                1048576.0,    // 1 MB
                10485760.0,   // 10 MB
                104857600.0,  // 100 MB
                524288000.0,  // 500 MB
                1073741824.0, // 1 GB
            ]),
        )
        .expect("metric can be created");

        let bundle_download_bytes = HistogramVec::new(
            HistogramOpts::new(
                "eunomia_registry_bundle_download_bytes",
                "Downloaded bundle size in bytes",
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
        if let Err(e) = registry.register(Box::new(publishes_total.clone())) {
            warn!("Failed to register publishes_total: {e}");
        }
        if let Err(e) = registry.register(Box::new(publish_duration_ms.clone())) {
            warn!("Failed to register publish_duration_ms: {e}");
        }
        if let Err(e) = registry.register(Box::new(fetches_total.clone())) {
            warn!("Failed to register fetches_total: {e}");
        }
        if let Err(e) = registry.register(Box::new(fetch_duration_ms.clone())) {
            warn!("Failed to register fetch_duration_ms: {e}");
        }
        if let Err(e) = registry.register(Box::new(cache_operations_total.clone())) {
            warn!("Failed to register cache_operations_total: {e}");
        }
        if let Err(e) = registry.register(Box::new(cache_size_bytes.clone())) {
            warn!("Failed to register cache_size_bytes: {e}");
        }
        if let Err(e) = registry.register(Box::new(bundle_download_bytes.clone())) {
            warn!("Failed to register bundle_download_bytes: {e}");
        }

        Self {
            publishes_total,
            publish_duration_ms,
            fetches_total,
            fetch_duration_ms,
            cache_operations_total,
            cache_size_bytes,
            bundle_download_bytes,
        }
    }

    /// Records a bundle publish operation.
    pub fn record_publish(&self, service: &str, success: bool, duration_ms: u64) {
        let status = if success { "success" } else { "failure" };
        self.publishes_total
            .with_label_values(&[service, status])
            .inc();
        self.publish_duration_ms
            .with_label_values(&[service])
            .observe(duration_ms as f64);
    }

    /// Records a bundle fetch operation.
    pub fn record_fetch(&self, service: &str, success: bool, duration_ms: u64) {
        let status = if success { "success" } else { "failure" };
        self.fetches_total
            .with_label_values(&[service, status])
            .inc();
        self.fetch_duration_ms
            .with_label_values(&[service])
            .observe(duration_ms as f64);
    }

    /// Records a cache hit.
    pub fn record_cache_hit(&self) {
        self.cache_operations_total
            .with_label_values(&["hit"])
            .inc();
    }

    /// Records a cache miss.
    pub fn record_cache_miss(&self) {
        self.cache_operations_total
            .with_label_values(&["miss"])
            .inc();
    }

    /// Records a cache eviction.
    pub fn record_cache_eviction(&self) {
        self.cache_operations_total
            .with_label_values(&["eviction"])
            .inc();
    }

    /// Records the current cache size.
    pub fn record_cache_size(&self, size_bytes: u64) {
        self.cache_size_bytes.observe(size_bytes as f64);
    }

    /// Records a bundle download size.
    pub fn record_bundle_download(&self, service: &str, size_bytes: u64) {
        self.bundle_download_bytes
            .with_label_values(&[service])
            .observe(size_bytes as f64);
    }

    /// Returns the total publish count for a service and status.
    #[must_use]
    pub fn get_publish_count(&self, service: &str, success: bool) -> f64 {
        let status = if success { "success" } else { "failure" };
        self.publishes_total
            .with_label_values(&[service, status])
            .get()
    }

    /// Returns the total fetch count for a service and status.
    #[must_use]
    pub fn get_fetch_count(&self, service: &str, success: bool) -> f64 {
        let status = if success { "success" } else { "failure" };
        self.fetches_total
            .with_label_values(&[service, status])
            .get()
    }

    /// Returns the cache operation count for a given operation type.
    #[must_use]
    pub fn get_cache_operation_count(&self, operation: &str) -> f64 {
        self.cache_operations_total
            .with_label_values(&[operation])
            .get()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_registry() -> (Registry, RegistryMetrics) {
        let registry = Registry::new();
        let metrics = RegistryMetrics::new(&registry);
        (registry, metrics)
    }

    #[test]
    fn test_record_publish() {
        let (_, metrics) = create_test_registry();

        metrics.record_publish("test-service", true, 500);
        metrics.record_publish("test-service", false, 100);

        assert_eq!(metrics.get_publish_count("test-service", true), 1.0);
        assert_eq!(metrics.get_publish_count("test-service", false), 1.0);
    }

    #[test]
    fn test_record_fetch() {
        let (_, metrics) = create_test_registry();

        metrics.record_fetch("test-service", true, 200);
        metrics.record_fetch("test-service", true, 150);
        metrics.record_fetch("test-service", false, 50);

        assert_eq!(metrics.get_fetch_count("test-service", true), 2.0);
        assert_eq!(metrics.get_fetch_count("test-service", false), 1.0);
    }

    #[test]
    fn test_cache_operations() {
        let (_, metrics) = create_test_registry();

        metrics.record_cache_hit();
        metrics.record_cache_hit();
        metrics.record_cache_miss();
        metrics.record_cache_eviction();

        assert_eq!(metrics.get_cache_operation_count("hit"), 2.0);
        assert_eq!(metrics.get_cache_operation_count("miss"), 1.0);
        assert_eq!(metrics.get_cache_operation_count("eviction"), 1.0);
    }

    #[test]
    fn test_record_cache_size() {
        let (_, metrics) = create_test_registry();

        metrics.record_cache_size(1048576); // 1 MB
        metrics.record_cache_size(10485760); // 10 MB
        // Histogram records observations, verify it doesn't panic
    }

    #[test]
    fn test_record_bundle_download() {
        let (_, metrics) = create_test_registry();

        metrics.record_bundle_download("test-service", 102400);
        // Histogram records observations, verify it doesn't panic
    }
}
