//! Instance discovery for Archimedes services.
//!
//! This module provides mechanisms for discovering Archimedes instances
//! across different environments (static configuration, Kubernetes, DNS).

use async_trait::async_trait;
use hickory_resolver::config::{ResolverConfig, ResolverOpts};
use hickory_resolver::TokioAsyncResolver;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::error::{DistributorError, Result};
use crate::instance::{Instance, InstanceMetadata};

/// Trait for instance discovery.
#[async_trait]
pub trait Discovery: Send + Sync {
    /// Discovers instances for a given service.
    async fn discover(&self, service: &str) -> Result<Vec<Instance>>;

    /// Returns all known instances across all services.
    async fn all_instances(&self) -> Result<Vec<Instance>>;

    /// Refreshes the instance cache (if applicable).
    async fn refresh(&self) -> Result<()>;
}

/// Discovery source configuration.
#[derive(Debug, Clone)]
pub enum DiscoverySource {
    /// Static list of endpoints.
    Static {
        /// List of endpoint addresses.
        endpoints: Vec<String>,
    },

    /// Kubernetes service discovery.
    Kubernetes {
        /// Namespace to discover in (None = all namespaces).
        namespace: Option<String>,

        /// Label selector for filtering pods.
        label_selector: Option<String>,

        /// Port name to use (default: "grpc").
        port_name: Option<String>,
    },

    /// DNS-based discovery.
    Dns {
        /// DNS hostnames to resolve.
        hosts: Vec<String>,

        /// Port to use for discovered IPs.
        port: u16,

        /// DNS resolver address (None = system default).
        resolver: Option<String>,
    },
}

/// Static instance discovery.
///
/// Uses a fixed list of endpoints provided at configuration time.
/// Useful for development, testing, and simple deployments.
#[derive(Debug)]
pub struct StaticDiscovery {
    #[allow(dead_code)]
    endpoints: Vec<String>,
    instances: Arc<RwLock<Vec<Instance>>>,
}

impl StaticDiscovery {
    /// Creates a new static discovery with the given endpoints.
    pub fn new(endpoints: Vec<String>) -> Self {
        let instances: Vec<Instance> = endpoints
            .iter()
            .enumerate()
            .map(|(i, ep)| {
                Instance::new(format!("static-{i}"), ep.clone())
                    .with_metadata(InstanceMetadata::default())
            })
            .collect();

        Self {
            endpoints,
            instances: Arc::new(RwLock::new(instances)),
        }
    }

    /// Adds an endpoint to the discovery.
    pub async fn add_endpoint(&self, endpoint: String) {
        let mut instances = self.instances.write().await;
        let id = format!("static-{}", instances.len());
        instances.push(Instance::new(id, endpoint));
    }

    /// Removes an endpoint by ID.
    pub async fn remove_endpoint(&self, instance_id: &str) {
        let mut instances = self.instances.write().await;
        instances.retain(|i| i.id != instance_id);
    }
}

#[async_trait]
impl Discovery for StaticDiscovery {
    async fn discover(&self, _service: &str) -> Result<Vec<Instance>> {
        // Static discovery returns all instances regardless of service
        // In a more sophisticated implementation, we'd filter by service
        let instances = self.instances.read().await;
        Ok(instances.clone())
    }

    async fn all_instances(&self) -> Result<Vec<Instance>> {
        let instances = self.instances.read().await;
        Ok(instances.clone())
    }

    async fn refresh(&self) -> Result<()> {
        // Static discovery doesn't need refresh
        Ok(())
    }
}

/// Combined discovery that aggregates multiple sources.
pub struct CombinedDiscovery {
    sources: Vec<Box<dyn Discovery>>,
}

impl CombinedDiscovery {
    /// Creates a new combined discovery with the given sources.
    pub fn new(sources: Vec<Box<dyn Discovery>>) -> Self {
        Self { sources }
    }
}

#[async_trait]
impl Discovery for CombinedDiscovery {
    async fn discover(&self, service: &str) -> Result<Vec<Instance>> {
        let mut all_instances = Vec::new();
        for source in &self.sources {
            let instances = source.discover(service).await?;
            all_instances.extend(instances);
        }
        Ok(all_instances)
    }

    async fn all_instances(&self) -> Result<Vec<Instance>> {
        let mut all_instances = Vec::new();
        for source in &self.sources {
            let instances = source.all_instances().await?;
            all_instances.extend(instances);
        }
        Ok(all_instances)
    }

    async fn refresh(&self) -> Result<()> {
        for source in &self.sources {
            source.refresh().await?;
        }
        Ok(())
    }
}

/// Caching wrapper for discovery sources.
///
/// Caches discovered instances for a configurable TTL to reduce
/// the load on external discovery services.
pub struct CachedDiscovery {
    inner: Box<dyn Discovery>,
    cache: Arc<RwLock<DiscoveryCache>>,
    ttl: std::time::Duration,
}

struct DiscoveryCache {
    instances: Vec<Instance>,
    last_refresh: Option<std::time::Instant>,
}

impl CachedDiscovery {
    /// Creates a new cached discovery wrapping the given source.
    pub fn new(inner: Box<dyn Discovery>, ttl: std::time::Duration) -> Self {
        Self {
            inner,
            cache: Arc::new(RwLock::new(DiscoveryCache {
                instances: Vec::new(),
                last_refresh: None,
            })),
            ttl,
        }
    }

    async fn is_cache_valid(&self) -> bool {
        let cache = self.cache.read().await;
        cache
            .last_refresh
            .is_some_and(|last_refresh| last_refresh.elapsed() < self.ttl)
    }

    async fn update_cache(&self, instances: Vec<Instance>) {
        let mut cache = self.cache.write().await;
        cache.instances = instances;
        cache.last_refresh = Some(std::time::Instant::now());
    }
}

#[async_trait]
impl Discovery for CachedDiscovery {
    async fn discover(&self, service: &str) -> Result<Vec<Instance>> {
        if !self.is_cache_valid().await {
            let instances = self.inner.all_instances().await?;
            self.update_cache(instances).await;
        }

        let cache = self.cache.read().await;
        let filtered: Vec<Instance> = cache
            .instances
            .iter()
            .filter(|i| i.service().is_none_or(|s| s == service))
            .cloned()
            .collect();

        Ok(filtered)
    }

    async fn all_instances(&self) -> Result<Vec<Instance>> {
        if !self.is_cache_valid().await {
            let instances = self.inner.all_instances().await?;
            self.update_cache(instances).await;
        }

        let cache = self.cache.read().await;
        Ok(cache.instances.clone())
    }

    async fn refresh(&self) -> Result<()> {
        let instances = self.inner.all_instances().await?;
        self.update_cache(instances).await;
        Ok(())
    }
}

/// DNS-based instance discovery.
///
/// Resolves hostnames to IP addresses using DNS lookups.
/// Supports both A (IPv4) and AAAA (IPv6) records.
///
/// # Example
///
/// ```rust,ignore
/// use eunomia_distributor::discovery::DnsDiscovery;
///
/// let discovery = DnsDiscovery::new(
///     vec!["archimedes.service.consul".to_string()],
///     8080,
/// );
/// let instances = discovery.all_instances().await?;
/// ```
pub struct DnsDiscovery {
    hosts: Vec<String>,
    port: u16,
    resolver: TokioAsyncResolver,
    instances: Arc<RwLock<Vec<Instance>>>,
}

impl DnsDiscovery {
    /// Creates a new DNS discovery with the given hosts and port.
    ///
    /// Uses the system DNS resolver by default.
    pub fn new(hosts: Vec<String>, port: u16) -> Self {
        let resolver =
            TokioAsyncResolver::tokio(ResolverConfig::default(), ResolverOpts::default());

        Self {
            hosts,
            port,
            resolver,
            instances: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Creates a DNS discovery with a custom resolver address.
    ///
    /// # Arguments
    ///
    /// * `hosts` - DNS hostnames to resolve
    /// * `port` - Port number to use for discovered instances
    /// * `resolver_addr` - Custom DNS resolver address (e.g., "8.8.8.8:53")
    pub fn with_resolver(hosts: Vec<String>, port: u16, resolver_addr: &str) -> Result<Self> {
        use hickory_resolver::config::NameServerConfig;
        use std::net::SocketAddr;

        let addr: SocketAddr = resolver_addr.parse().map_err(|e| {
            DistributorError::InvalidConfig {
                reason: format!("Invalid resolver address: {e}"),
            }
        })?;

        let mut config = ResolverConfig::new();
        config.add_name_server(NameServerConfig::new(
            addr,
            hickory_resolver::config::Protocol::Udp,
        ));

        let resolver = TokioAsyncResolver::tokio(config, ResolverOpts::default());

        Ok(Self {
            hosts,
            port,
            resolver,
            instances: Arc::new(RwLock::new(Vec::new())),
        })
    }

    /// Resolves all configured hosts and updates the instance list.
    async fn resolve_hosts(&self) -> Result<Vec<Instance>> {
        let mut instances = Vec::new();

        for host in &self.hosts {
            match self.resolver.lookup_ip(host.as_str()).await {
                Ok(lookup) => {
                    for ip in lookup.iter() {
                        let endpoint = format!("{}:{}", ip, self.port);
                        let instance_id = format!("dns-{host}-{ip}");

                        let mut metadata = InstanceMetadata::default();
                        metadata.annotations.insert("dns.host".to_string(), host.clone());
                        metadata.annotations.insert("dns.resolved_ip".to_string(), ip.to_string());

                        instances.push(
                            Instance::new(instance_id, endpoint).with_metadata(metadata),
                        );
                    }
                }
                Err(e) => {
                    tracing::warn!(host = %host, error = %e, "DNS lookup failed");
                    // Continue with other hosts, don't fail the entire discovery
                }
            }
        }

        Ok(instances)
    }
}

impl std::fmt::Debug for DnsDiscovery {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DnsDiscovery")
            .field("hosts", &self.hosts)
            .field("port", &self.port)
            .finish_non_exhaustive()
    }
}

#[async_trait]
impl Discovery for DnsDiscovery {
    async fn discover(&self, _service: &str) -> Result<Vec<Instance>> {
        // DNS discovery returns all resolved instances regardless of service
        // Service filtering would require additional DNS TXT records or SRV records
        self.all_instances().await
    }

    async fn all_instances(&self) -> Result<Vec<Instance>> {
        let instances = self.instances.read().await;
        if instances.is_empty() {
            // If cache is empty, do a refresh first
            drop(instances);
            self.refresh().await?;
            let instances = self.instances.read().await;
            Ok(instances.clone())
        } else {
            Ok(instances.clone())
        }
    }

    async fn refresh(&self) -> Result<()> {
        let resolved = self.resolve_hosts().await?;
        let mut instances = self.instances.write().await;
        *instances = resolved;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_static_discovery_creation() {
        let endpoints = vec!["localhost:8080".to_string(), "localhost:8081".to_string()];
        let discovery = StaticDiscovery::new(endpoints);

        let instances = discovery.all_instances().await.unwrap();
        assert_eq!(instances.len(), 2);
        assert_eq!(instances[0].id, "static-0");
        assert_eq!(instances[1].id, "static-1");
    }

    #[tokio::test]
    async fn test_static_discovery_discover() {
        let endpoints = vec!["localhost:8080".to_string()];
        let discovery = StaticDiscovery::new(endpoints);

        let instances = discovery.discover("any-service").await.unwrap();
        assert_eq!(instances.len(), 1);
    }

    #[tokio::test]
    async fn test_static_discovery_add_endpoint() {
        let discovery = StaticDiscovery::new(vec![]);
        assert_eq!(discovery.all_instances().await.unwrap().len(), 0);

        discovery.add_endpoint("localhost:8080".to_string()).await;
        assert_eq!(discovery.all_instances().await.unwrap().len(), 1);

        discovery.add_endpoint("localhost:8081".to_string()).await;
        assert_eq!(discovery.all_instances().await.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn test_static_discovery_remove_endpoint() {
        let endpoints = vec!["localhost:8080".to_string(), "localhost:8081".to_string()];
        let discovery = StaticDiscovery::new(endpoints);

        discovery.remove_endpoint("static-0").await;
        let instances = discovery.all_instances().await.unwrap();
        assert_eq!(instances.len(), 1);
        assert_eq!(instances[0].id, "static-1");
    }

    #[tokio::test]
    async fn test_static_discovery_refresh() {
        let discovery = StaticDiscovery::new(vec!["localhost:8080".to_string()]);
        // Refresh should succeed but do nothing
        discovery.refresh().await.unwrap();
        assert_eq!(discovery.all_instances().await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_combined_discovery() {
        let source1 = Box::new(StaticDiscovery::new(vec!["host1:8080".to_string()]));
        let source2 = Box::new(StaticDiscovery::new(vec!["host2:8080".to_string()]));

        let combined = CombinedDiscovery::new(vec![source1, source2]);
        let instances = combined.all_instances().await.unwrap();
        assert_eq!(instances.len(), 2);
    }

    #[tokio::test]
    async fn test_cached_discovery() {
        let inner = Box::new(StaticDiscovery::new(vec!["localhost:8080".to_string()]));
        let cached = CachedDiscovery::new(inner, std::time::Duration::from_secs(60));

        // First call should populate cache
        let instances = cached.all_instances().await.unwrap();
        assert_eq!(instances.len(), 1);

        // Second call should use cache
        let instances = cached.all_instances().await.unwrap();
        assert_eq!(instances.len(), 1);
    }

    #[tokio::test]
    async fn test_cached_discovery_refresh() {
        let inner = Box::new(StaticDiscovery::new(vec!["localhost:8080".to_string()]));
        let cached = CachedDiscovery::new(inner, std::time::Duration::from_secs(60));

        // Refresh should work
        cached.refresh().await.unwrap();
        let instances = cached.all_instances().await.unwrap();
        assert_eq!(instances.len(), 1);
    }

    #[test]
    fn test_discovery_source_static() {
        let source = DiscoverySource::Static {
            endpoints: vec!["host:8080".to_string()],
        };

        if let DiscoverySource::Static { endpoints } = source {
            assert_eq!(endpoints.len(), 1);
        } else {
            panic!("Expected static source");
        }
    }

    #[test]
    fn test_discovery_source_kubernetes() {
        let source = DiscoverySource::Kubernetes {
            namespace: Some("default".to_string()),
            label_selector: Some("app=archimedes".to_string()),
            port_name: Some("grpc".to_string()),
        };

        if let DiscoverySource::Kubernetes {
            namespace,
            label_selector,
            port_name,
        } = source
        {
            assert_eq!(namespace, Some("default".to_string()));
            assert_eq!(label_selector, Some("app=archimedes".to_string()));
            assert_eq!(port_name, Some("grpc".to_string()));
        } else {
            panic!("Expected kubernetes source");
        }
    }

    #[test]
    fn test_discovery_source_dns() {
        let source = DiscoverySource::Dns {
            hosts: vec!["archimedes.service.consul".to_string()],
            port: 8080,
            resolver: None,
        };

        if let DiscoverySource::Dns {
            hosts,
            port,
            resolver,
        } = source
        {
            assert_eq!(hosts.len(), 1);
            assert_eq!(port, 8080);
            assert!(resolver.is_none());
        } else {
            panic!("Expected DNS source");
        }
    }

    #[test]
    fn test_dns_discovery_creation() {
        let discovery = DnsDiscovery::new(
            vec!["example.com".to_string(), "test.local".to_string()],
            8080,
        );
        assert_eq!(discovery.hosts.len(), 2);
        assert_eq!(discovery.port, 8080);
    }

    #[test]
    fn test_dns_discovery_with_invalid_resolver() {
        let result = DnsDiscovery::with_resolver(
            vec!["example.com".to_string()],
            8080,
            "not-a-valid-address",
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_dns_discovery_with_valid_resolver() {
        let result = DnsDiscovery::with_resolver(
            vec!["example.com".to_string()],
            8080,
            "8.8.8.8:53",
        );
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_dns_discovery_localhost() {
        // This test resolves localhost which should work on any system
        let discovery = DnsDiscovery::new(vec!["localhost".to_string()], 8080);
        
        // Refresh to resolve
        discovery.refresh().await.unwrap();
        
        let instances = discovery.all_instances().await.unwrap();
        // localhost should resolve to at least one address (127.0.0.1 or ::1)
        assert!(!instances.is_empty(), "localhost should resolve to at least one IP");
        
        // Check instance metadata
        let first = &instances[0];
        assert!(first.id.starts_with("dns-localhost-"));
        assert!(first.endpoint.to_string().ends_with(":8080"));
    }

    #[tokio::test]
    async fn test_dns_discovery_nonexistent_host() {
        // This test tries to resolve a non-existent host
        let discovery = DnsDiscovery::new(
            vec!["this-host-definitely-does-not-exist-12345.invalid".to_string()],
            8080,
        );
        
        // Refresh should not fail, but return empty results
        discovery.refresh().await.unwrap();
        
        let instances = discovery.all_instances().await.unwrap();
        assert!(instances.is_empty());
    }

    #[tokio::test]
    async fn test_dns_discovery_mixed_hosts() {
        // Test with a mix of valid and invalid hosts
        let discovery = DnsDiscovery::new(
            vec![
                "localhost".to_string(),
                "nonexistent-host-xyz.invalid".to_string(),
            ],
            9090,
        );
        
        discovery.refresh().await.unwrap();
        
        let instances = discovery.all_instances().await.unwrap();
        // Should have at least the localhost resolution
        assert!(!instances.is_empty());
        
        // All instances should have port 9090
        for instance in &instances {
            assert!(instance.endpoint.to_string().contains(":9090"));
        }
    }
}
