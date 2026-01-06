//! gRPC server implementation for the Control Plane.
//!
//! This module provides the main server struct that binds both
//! `ControlPlane` and `PolicyReceiver` services.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::oneshot;
use tonic::transport::Server;
use tracing::{info, warn};

use super::control_plane::ControlPlaneService;
use crate::Distributor;

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
        Self {
            config,
            distributor,
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

    /// Run the gRPC server.
    ///
    /// Returns a handle that can be used to trigger graceful shutdown.
    pub async fn run(self) -> Result<GrpcServerHandle, GrpcServerError> {
        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

        let control_plane = ControlPlaneService::new(Arc::clone(&self.distributor));

        let addr = self.config.bind_address;

        info!("Starting gRPC server on {}", addr);

        let mut builder = Server::builder();

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
}
