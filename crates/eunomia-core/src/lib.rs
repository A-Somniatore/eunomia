//! # Eunomia Core
//!
//! Core types and traits for the Eunomia authorization policy platform.
//!
//! This crate provides the foundational data structures used throughout the
//! Eunomia ecosystem, including:
//!
//! - [`Policy`] - Policy model representing a Rego policy file
//! - [`Bundle`] - Compiled policy bundle for distribution
//! - [`AuthorizationDecision`] - Result of policy evaluation (from `themis-platform-types`)
//! - [`PolicyInput`] - Input schema for authorization requests (from `themis-platform-types`)
//! - [`CallerIdentity`] - Identity types (from `themis-platform-types`)
//! - [`signing`] - Ed25519 bundle signing and verification
//!
//! ## Shared Platform Types
//!
//! The core authorization types (`CallerIdentity`, `PolicyInput`, `PolicyDecision`)
//! are now provided by the `themis-platform-types` crate to ensure schema
//! compatibility across the Themis ecosystem.
//!
//! ## Example
//!
//! ```rust
//! use eunomia_core::{PolicyInput, CallerIdentity, AuthorizationDecision};
//!
//! // Create a caller identity
//! let caller = CallerIdentity::user("user-123", "user@example.com");
//!
//! // Create a policy input for authorization  
//! let input = PolicyInput::builder()
//!     .caller(caller)
//!     .service("users-service")
//!     .operation_id("getUser")
//!     .method("GET")
//!     .path("/users/user-123")
//!     .build();
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod bundle;
pub mod error;
pub mod policy;
pub mod signing;
pub mod validation;

// Legacy modules - kept for backward compatibility but deprecated
#[deprecated(since = "0.2.0", note = "Use CallerIdentity from themis-platform-types instead")]
pub mod identity;
#[deprecated(since = "0.2.0", note = "Use PolicyInput from themis-platform-types instead")]
pub mod input;
#[deprecated(since = "0.2.0", note = "Use PolicyDecision from themis-platform-types instead")]
pub mod decision;

#[cfg(test)]
mod proptest_tests;

// Re-export main types at crate root
pub use bundle::Bundle;
pub use error::{Error, Result};
pub use policy::Policy;
pub use signing::{BundleSigner, BundleVerifier, SignedBundle, SigningError, SigningKeyPair};
pub use validation::{Validate, ValidationError};

// Re-export shared platform types from themis-platform-types
// These are the canonical types that should be used across the Themis ecosystem
pub use themis_platform_types::CallerIdentity;
pub use themis_platform_types::PolicyInput;
pub use themis_platform_types::PolicyInputBuilder;
pub use themis_platform_types::PolicyDecision;
pub use themis_platform_types::RequestId;

/// Type alias for backward compatibility.
/// 
/// Use [`PolicyDecision`] directly instead.
#[deprecated(since = "0.2.0", note = "Use PolicyDecision from themis-platform-types instead")]
pub type AuthorizationDecision = PolicyDecision;
