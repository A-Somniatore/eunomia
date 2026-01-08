//! gRPC server implementation for the Control Plane.
//!
//! This module provides the gRPC services for policy distribution:
//! - `ControlPlaneService`: Deploy, rollback, and monitor policies
//! - `PolicyReceiverService`: Handle policy updates from the registry
//!
//! # Rate Limiting
//!
//! The server supports configurable rate limiting per endpoint:
//!
//! ```rust,ignore
//! use eunomia_distributor::grpc::rate_limit::{RateLimitConfig, EndpointRateLimits};
//!
//! let limits = EndpointRateLimits::default()
//!     .with_deploy_policy(RateLimitConfig::new(50).with_burst_size(25));
//! ```
//!
//! # Example
//!
//! ```rust,ignore
//! use std::sync::Arc;
//! use eunomia_distributor::{Distributor, DistributorConfig};
//! use eunomia_distributor::grpc::{GrpcServer, GrpcServerConfig};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let distributor = Arc::new(Distributor::new(DistributorConfig::default()).await?);
//!     let config = GrpcServerConfig::new("0.0.0.0:9090".parse()?);
//!     let server = GrpcServer::new(distributor, config);
//!     
//!     let handle = server.run().await?;
//!     // Server is now running
//!     
//!     // To shutdown:
//!     handle.shutdown();
//!     Ok(())
//! }
//! ```

mod control_plane;
mod policy_receiver;
pub mod rate_limit;
mod server;
pub mod types;

pub use control_plane::{ControlPlane, ControlPlaneService, ControlPlaneServiceServer};
pub use policy_receiver::{PolicyReceiver, PolicyReceiverService, PolicyReceiverServiceServer};
pub use rate_limit::{
    create_rate_limiter, EndpointRateLimits, EndpointStats, RateLimitConfig, RateLimitStats,
    RateLimiterRegistry, SharedRateLimiter, TokenBucket,
};
pub use server::{GrpcServer, GrpcServerConfig, GrpcServerError, GrpcServerHandle, TlsConfig};
