//! HTTP server for metrics endpoint.
//!
//! Provides a `/metrics` endpoint for Prometheus scraping.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Router,
};
use tokio::sync::oneshot;
use tracing::{info, error};

use crate::MetricsRegistry;

/// Server configuration for the metrics endpoint.
#[derive(Debug, Clone)]
pub struct MetricsServerConfig {
    /// Address to bind the server to.
    pub address: SocketAddr,
    /// Path for the metrics endpoint.
    pub path: String,
}

impl Default for MetricsServerConfig {
    fn default() -> Self {
        Self {
            address: "0.0.0.0:9090".parse().expect("valid address"),
            path: "/metrics".to_string(),
        }
    }
}

/// Handle for the running metrics server.
pub struct MetricsServerHandle {
    /// Shutdown signal sender.
    shutdown_tx: oneshot::Sender<()>,
}

impl MetricsServerHandle {
    /// Gracefully shuts down the metrics server.
    pub fn shutdown(self) {
        let _ = self.shutdown_tx.send(());
    }
}

/// Starts the metrics HTTP server.
///
/// Returns a handle that can be used to shut down the server.
///
/// # Arguments
///
/// * `config` - Server configuration
///
/// # Errors
///
/// Returns an error if the server fails to bind to the address.
///
/// # Examples
///
/// ```no_run
/// use eunomia_metrics::{MetricsServerConfig, serve_metrics};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let config = MetricsServerConfig::default();
/// let handle = serve_metrics(config).await?;
///
/// // Later, shut down the server
/// handle.shutdown();
/// # Ok(())
/// # }
/// ```
pub async fn serve_metrics(
    config: MetricsServerConfig,
) -> Result<MetricsServerHandle, std::io::Error> {
    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    let registry = Arc::new(MetricsRegistry::global().clone());

    let app = Router::new()
        .route(&config.path, get(metrics_handler))
        .route("/health", get(health_handler))
        .route("/ready", get(ready_handler))
        .with_state(registry);

    let listener = tokio::net::TcpListener::bind(config.address).await?;
    info!("Metrics server listening on {}", config.address);

    tokio::spawn(async move {
        axum::serve(listener, app)
            .with_graceful_shutdown(async move {
                let _ = shutdown_rx.await;
                info!("Metrics server shutting down");
            })
            .await
            .expect("server error");
    });

    Ok(MetricsServerHandle { shutdown_tx })
}

/// Handler for the `/metrics` endpoint.
async fn metrics_handler(State(registry): State<Arc<MetricsRegistry>>) -> impl IntoResponse {
    match registry.prometheus_output() {
        Ok(output) => (StatusCode::OK, output),
        Err(e) => {
            error!("Failed to encode metrics: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to encode metrics: {e}"),
            )
        }
    }
}

/// Handler for the `/health` endpoint (liveness probe).
async fn health_handler() -> impl IntoResponse {
    (StatusCode::OK, "healthy")
}

/// Handler for the `/ready` endpoint (readiness probe).
async fn ready_handler() -> impl IntoResponse {
    // TODO: Add actual readiness checks (registry connectivity, etc.)
    (StatusCode::OK, "ready")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = MetricsServerConfig::default();
        assert_eq!(config.address.port(), 9090);
        assert_eq!(config.path, "/metrics");
    }

    #[test]
    fn test_custom_config() {
        let config = MetricsServerConfig {
            address: "127.0.0.1:8080".parse().unwrap(),
            path: "/custom/metrics".to_string(),
        };
        assert_eq!(config.address.port(), 8080);
        assert_eq!(config.path, "/custom/metrics");
    }

    #[tokio::test]
    async fn test_serve_metrics() {
        // Use a random port to avoid conflicts
        let config = MetricsServerConfig {
            address: "127.0.0.1:0".parse().unwrap(),
            path: "/metrics".to_string(),
        };

        let handle = serve_metrics(config).await.expect("server should start");

        // Give the server a moment to start
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        handle.shutdown();
    }
}
