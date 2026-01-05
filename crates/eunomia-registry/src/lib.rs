//! # Eunomia Registry
//!
//! OCI-compatible bundle registry client for Eunomia policy bundles.
//!
//! This crate provides functionality to publish, fetch, and manage policy
//! bundles in OCI-compatible registries (Docker Registry, Harbor, ECR, GCR, etc.).
//!
//! ## Features
//!
//! - **OCI Distribution API**: Full support for OCI Distribution Specification
//! - **Multiple Auth Methods**: Basic, Bearer token, AWS ECR, GCP Artifact Registry
//! - **Local Caching**: File-based cache with LRU eviction
//! - **Version Resolution**: Semantic version resolution and tag management
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use eunomia_registry::{RegistryClient, RegistryConfig, RegistryAuth};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create registry client
//!     let config = RegistryConfig::new("https://registry.example.com")
//!         .with_namespace("policies")
//!         .with_auth(RegistryAuth::None);
//!     
//!     let client = RegistryClient::new(config)?;
//!     
//!     // Check if a bundle exists
//!     let exists = client.exists("users-service", "v1.2.0").await?;
//!     
//!     Ok(())
//! }
//! ```
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    RegistryClient                           │
//! │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
//! │  │   OciApi    │  │   Cache     │  │   VersionResolver   │  │
//! │  │  (HTTP)     │  │  (File)     │  │   (SemVer)          │  │
//! │  └─────────────┘  └─────────────┘  └─────────────────────┘  │
//! └─────────────────────────────────────────────────────────────┘
//!                          │
//!                          ▼
//! ┌─────────────────────────────────────────────────────────────┐
//! │                  OCI Registry                                │
//! │     (Docker Registry, Harbor, ECR, GCR, etc.)               │
//! └─────────────────────────────────────────────────────────────┘
//! ```

#![deny(missing_docs)]
#![deny(clippy::all)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]

mod cache;
mod client;
mod config;
mod error;
mod oci;
mod version;

pub use cache::{BundleCache, CacheConfig};
pub use client::RegistryClient;
pub use config::{RegistryAuth, RegistryConfig, TlsConfig};
pub use error::RegistryError;
pub use oci::{Descriptor, Manifest, MediaType};
pub use version::{VersionQuery, VersionResolver};
