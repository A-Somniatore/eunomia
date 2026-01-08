//! gRPC server implementation for the Control Plane.
//!
//! This module provides the main server struct that binds both
//! `ControlPlane` and `PolicyReceiver` services.
//!
//! ## mTLS Support
//!
//! The server supports mutual TLS (mTLS) for secure communication:
//!
//! ```rust,ignore
//! let config = GrpcServerConfig::default()
//!     .with_tls(TlsConfig {
//!         cert_pem: include_str!("server.crt").to_string(),
//!         key_pem: include_str!("server.key").to_string(),
//!         ca_cert_pem: Some(include_str!("ca.crt").to_string()),
//!     });
//! ```
//!
//! ## Rate Limiting
//!
//! The server supports configurable rate limiting per endpoint:
//!
//! ```rust,ignore
//! use eunomia_distributor::grpc::rate_limit::{EndpointRateLimits, RateLimitConfig};
//!
//! let config = GrpcServerConfig::default()
//!     .with_rate_limits(
//!         EndpointRateLimits::default()
//!             .with_deploy_policy(RateLimitConfig::new(50))
//!     );
//! ```

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::oneshot;
use tonic::transport::{Certificate, Identity, Server, ServerTlsConfig};
use tracing::{info, warn};

use super::control_plane::ControlPlaneService;
use super::rate_limit::{EndpointRateLimits, RateLimiterRegistry};
use crate::Distributor;

/// TLS configuration for the gRPC server.
#[derive(Debug, Clone)]
pub struct TlsConfig {
    /// Server certificate in PEM format.
    pub cert_pem: String,
    /// Server private key in PEM format.
    pub key_pem: String,
    /// Optional CA certificate for client verification (enables mTLS).
    pub ca_cert_pem: Option<String>,
}

/// gRPC server configuration.
#[derive(Debug, Clone)]
pub struct GrpcServerConfig {
    /// Address to bind the server to.
    pub bind_address: SocketAddr,
    /// TCP keep-alive interval.
    pub tcp_keepalive: Option<Duration>,
    /// TCP nodelay flag.
    pub tcp_nodelay: bool,
    /// Maximum concurrent streams per connection.
    pub max_concurrent_streams: Option<u32>,
    /// HTTP/2 keep-alive interval.
    pub http2_keepalive_interval: Option<Duration>,
    /// HTTP/2 keep-alive timeout.
    pub http2_keepalive_timeout: Option<Duration>,
    /// Maximum receive message size (bytes).
    pub max_recv_message_size: Option<usize>,
    /// Maximum send message size (bytes).
    pub max_send_message_size: Option<usize>,
    /// Enable reflection service.
    pub enable_reflection: bool,
    /// Enable health check service.
    pub enable_health_check: bool,
    /// TLS configuration for secure connections.
    pub tls_config: Option<TlsConfig>,
    /// Rate limiting configuration per endpoint.
    pub rate_limits: Option<EndpointRateLimits>,
}

impl Default for GrpcServerConfig {
    fn default() -> Self {
        Self {
            bind_address: "127.0.0.1:9090".parse().expect("valid address"),
            tcp_keepalive: Some(Duration::from_secs(60)),
            tcp_nodelay: true,
            max_concurrent_streams: Some(200),
            http2_keepalive_interval: Some(Duration::from_secs(30)),
            http2_keepalive_timeout: Some(Duration::from_secs(10)),
            max_recv_message_size: Some(4 * 1024 * 1024), // 4MB
            max_send_message_size: Some(4 * 1024 * 1024), // 4MB
            enable_reflection: true,
            enable_health_check: true,
            tls_config: None,
            rate_limits: Some(EndpointRateLimits::default()),
        }
    }
}

impl GrpcServerConfig {
    /// Create a new config with the given bind address.
    pub fn new(bind_address: SocketAddr) -> Self {
        Self {
            bind_address,
            ..Default::default()
        }
    }

    /// Set TCP keep-alive interval.
    pub fn with_tcp_keepalive(mut self, keepalive: Duration) -> Self {
        self.tcp_keepalive = Some(keepalive);
        self
    }

    /// Disable TCP keep-alive.
    pub fn without_tcp_keepalive(mut self) -> Self {
        self.tcp_keepalive = None;
        self
    }

    /// Set max concurrent streams.
    pub fn with_max_concurrent_streams(mut self, max: u32) -> Self {
        self.max_concurrent_streams = Some(max);
        self
    }

    /// Set max receive message size.
    pub fn with_max_recv_message_size(mut self, size: usize) -> Self {
        self.max_recv_message_size = Some(size);
        self
    }

    /// Enable or disable reflection service.
    pub fn with_reflection(mut self, enable: bool) -> Self {
        self.enable_reflection = enable;
        self
    }

    /// Configure TLS for secure connections.
    ///
    /// If `ca_cert_pem` is provided in the config, mutual TLS (mTLS)
    /// is enabled and clients must present valid certificates.
    pub fn with_tls(mut self, tls: TlsConfig) -> Self {
        self.tls_config = Some(tls);
        self
    }

    /// Check if TLS is enabled.
    pub fn is_tls_enabled(&self) -> bool {
        self.tls_config.is_some()
    }

    /// Check if mTLS (mutual TLS) is enabled.
    pub fn is_mtls_enabled(&self) -> bool {
        self.tls_config
            .as_ref()
            .is_some_and(|tls| tls.ca_cert_pem.is_some())
    }

    /// Configure rate limiting per endpoint.
    ///
    /// Rate limiting protects the server from abuse and ensures fair resource allocation.
    /// By default, rate limiting is enabled with sensible defaults.
    pub fn with_rate_limits(mut self, limits: EndpointRateLimits) -> Self {
        self.rate_limits = Some(limits);
        self
    }

    /// Disable rate limiting.
    pub fn without_rate_limits(mut self) -> Self {
        self.rate_limits = None;
        self
    }

    /// Check if rate limiting is enabled.
    pub fn is_rate_limiting_enabled(&self) -> bool {
        self.rate_limits.is_some()
    }
}

/// gRPC server handle for graceful shutdown.
#[derive(Debug)]
pub struct GrpcServerHandle {
    shutdown_tx: oneshot::Sender<()>,
}

impl GrpcServerHandle {
    /// Trigger graceful shutdown.
    pub fn shutdown(self) {
        let _ = self.shutdown_tx.send(());
    }
}

/// gRPC server for the Control Plane.
///
/// This server exposes:
/// - `ControlPlane` service: Deploy, rollback, and monitor policies
/// - `PolicyReceiver` service: (Future) Receive policy updates from Archimedes
pub struct GrpcServer {
    config: GrpcServerConfig,
    distributor: Arc<Distributor>,
    rate_limiter: Option<Arc<RateLimiterRegistry>>,
}

impl std::fmt::Debug for GrpcServer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GrpcServer")
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

impl GrpcServer {
    /// Create a new gRPC server.
    pub fn new(distributor: Arc<Distributor>, config: GrpcServerConfig) -> Self {
        let rate_limiter = config
            .rate_limits
            .clone()
            .map(|limits| Arc::new(RateLimiterRegistry::new(limits)));

        Self {
            config,
            distributor,
            rate_limiter,
        }
    }

    /// Create a new gRPC server with default configuration.
    pub fn with_distributor(distributor: Arc<Distributor>) -> Self {
        Self::new(distributor, GrpcServerConfig::default())
    }

    /// Get the server configuration.
    pub fn config(&self) -> &GrpcServerConfig {
        &self.config
    }

    /// Get the bind address.
    pub fn bind_address(&self) -> SocketAddr {
        self.config.bind_address
    }

    /// Get the rate limiter registry (if enabled).
    pub fn rate_limiter(&self) -> Option<&Arc<RateLimiterRegistry>> {
        self.rate_limiter.as_ref()
    }

    /// Run the gRPC server.
    ///
    /// Returns a handle that can be used to trigger graceful shutdown.
    pub async fn run(self) -> Result<GrpcServerHandle, GrpcServerError> {
        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

        let control_plane = ControlPlaneService::new(Arc::clone(&self.distributor))
            .with_rate_limiter_opt(self.rate_limiter.as_ref().map(Arc::clone));

        let addr = self.config.bind_address;

        // Configure TLS if enabled
        let tls_config = if let Some(ref tls) = self.config.tls_config {
            let identity = Identity::from_pem(tls.cert_pem.as_bytes(), tls.key_pem.as_bytes());
            let mut server_tls = ServerTlsConfig::new().identity(identity);

            // Enable mTLS if CA cert provided
            if let Some(ref ca_cert) = tls.ca_cert_pem {
                let ca_cert = Certificate::from_pem(ca_cert.as_bytes());
                server_tls = server_tls.client_ca_root(ca_cert);
                info!("mTLS enabled - client certificates will be verified");
            }

            Some(server_tls)
        } else {
            None
        };

        let tls_mode = if self.config.is_mtls_enabled() {
            "mTLS"
        } else if self.config.is_tls_enabled() {
            "TLS"
        } else {
            "insecure"
        };
        info!("Starting gRPC server on {} ({})", addr, tls_mode);

        let mut builder = Server::builder();

        // Apply TLS configuration
        if let Some(tls) = tls_config {
            builder = builder
                .tls_config(tls)
                .map_err(|e| GrpcServerError::TlsConfig(e.to_string()))?;
        }

        // Apply TCP settings
        if self.config.tcp_nodelay {
            builder = builder.tcp_nodelay(true);
        }
        if let Some(keepalive) = self.config.tcp_keepalive {
            builder = builder.tcp_keepalive(Some(keepalive));
        }

        // Apply HTTP/2 settings
        if let Some(interval) = self.config.http2_keepalive_interval {
            builder = builder.http2_keepalive_interval(Some(interval));
        }
        if let Some(timeout) = self.config.http2_keepalive_timeout {
            builder = builder.http2_keepalive_timeout(Some(timeout));
        }
        if let Some(max_streams) = self.config.max_concurrent_streams {
            builder = builder.concurrency_limit_per_connection(max_streams as usize);
        }

        // Build the router with services
        let router = builder.add_service(control_plane.into_service());

        // Spawn the server
        tokio::spawn(async move {
            let result = router
                .serve_with_shutdown(addr, async {
                    let _ = shutdown_rx.await;
                    info!("gRPC server shutdown signal received");
                })
                .await;

            if let Err(e) = result {
                warn!("gRPC server error: {}", e);
            }
        });

        Ok(GrpcServerHandle { shutdown_tx })
    }

    /// Run the server and block until shutdown.
    pub async fn run_until_shutdown(self) -> Result<(), GrpcServerError> {
        let handle = self.run().await?;

        // Wait for Ctrl+C
        tokio::signal::ctrl_c()
            .await
            .map_err(|e| GrpcServerError::Internal(e.to_string()))?;

        info!("Shutting down gRPC server...");
        handle.shutdown();

        // Give some time for graceful shutdown
        tokio::time::sleep(Duration::from_millis(500)).await;

        Ok(())
    }
}

/// gRPC server errors.
#[derive(Debug, thiserror::Error)]
pub enum GrpcServerError {
    /// Transport error.
    #[error("transport error: {0}")]
    Transport(#[from] tonic::transport::Error),

    /// TLS configuration error.
    #[error("TLS configuration error: {0}")]
    TlsConfig(String),

    /// Internal error.
    #[error("internal error: {0}")]
    Internal(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = GrpcServerConfig::default();
        assert_eq!(config.bind_address, "127.0.0.1:9090".parse().unwrap());
        assert!(config.tcp_nodelay);
        assert!(config.enable_reflection);
        assert_eq!(config.max_recv_message_size, Some(4 * 1024 * 1024));
        assert!(config.tls_config.is_none());
        assert!(!config.is_tls_enabled());
        assert!(!config.is_mtls_enabled());
    }

    #[test]
    fn test_config_builder() {
        let addr: SocketAddr = "0.0.0.0:8080".parse().unwrap();
        let config = GrpcServerConfig::new(addr)
            .with_tcp_keepalive(Duration::from_secs(120))
            .with_max_concurrent_streams(500)
            .with_max_recv_message_size(8 * 1024 * 1024)
            .with_reflection(false);

        assert_eq!(config.bind_address, addr);
        assert_eq!(config.tcp_keepalive, Some(Duration::from_secs(120)));
        assert_eq!(config.max_concurrent_streams, Some(500));
        assert!(!config.enable_reflection);
    }

    #[test]
    fn test_config_disable_keepalive() {
        let config = GrpcServerConfig::default().without_tcp_keepalive();
        assert!(config.tcp_keepalive.is_none());
    }

    #[test]
    fn test_tls_config_server_only() {
        let config = GrpcServerConfig::default().with_tls(TlsConfig {
            cert_pem: "-----BEGIN CERTIFICATE-----\ntest\n-----END CERTIFICATE-----".to_string(),
            key_pem: "-----BEGIN PRIVATE KEY-----\ntest\n-----END PRIVATE KEY-----".to_string(),
            ca_cert_pem: None,
        });

        assert!(config.is_tls_enabled());
        assert!(!config.is_mtls_enabled());
    }

    #[test]
    fn test_mtls_config() {
        let config = GrpcServerConfig::default().with_tls(TlsConfig {
            cert_pem: "-----BEGIN CERTIFICATE-----\ntest\n-----END CERTIFICATE-----".to_string(),
            key_pem: "-----BEGIN PRIVATE KEY-----\ntest\n-----END PRIVATE KEY-----".to_string(),
            ca_cert_pem: Some(
                "-----BEGIN CERTIFICATE-----\nca\n-----END CERTIFICATE-----".to_string(),
            ),
        });

        assert!(config.is_tls_enabled());
        assert!(config.is_mtls_enabled());
    }

    #[test]
    fn test_tls_config_struct() {
        let tls = TlsConfig {
            cert_pem: "cert".to_string(),
            key_pem: "key".to_string(),
            ca_cert_pem: Some("ca".to_string()),
        };

        assert_eq!(tls.cert_pem, "cert");
        assert_eq!(tls.key_pem, "key");
        assert_eq!(tls.ca_cert_pem, Some("ca".to_string()));
    }

    #[test]
    fn test_rate_limiting_enabled_by_default() {
        let config = GrpcServerConfig::default();
        assert!(config.is_rate_limiting_enabled());
        assert!(config.rate_limits.is_some());
    }

    #[test]
    fn test_rate_limiting_disabled() {
        let config = GrpcServerConfig::default().without_rate_limits();
        assert!(!config.is_rate_limiting_enabled());
        assert!(config.rate_limits.is_none());
    }

    #[test]
    fn test_custom_rate_limits() {
        use crate::grpc::rate_limit::RateLimitConfig;

        let custom_limits = EndpointRateLimits::new(RateLimitConfig::new(500))
            .with_deploy_policy(RateLimitConfig::new(100).with_burst_size(50));

        let config = GrpcServerConfig::default().with_rate_limits(custom_limits);
        assert!(config.is_rate_limiting_enabled());

        let limits = config.rate_limits.as_ref().unwrap();
        assert_eq!(limits.default.requests_per_second.get(), 500);
        assert_eq!(
            limits
                .deploy_policy
                .as_ref()
                .unwrap()
                .requests_per_second
                .get(),
            100
        );
    }
}
