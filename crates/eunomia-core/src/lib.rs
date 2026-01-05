//! # Eunomia Core
//!
//! Core types and traits for the Eunomia authorization policy platform.
//!
//! This crate provides the foundational data structures used throughout the
//! Eunomia ecosystem, including:
//!
//! - [`Policy`] - Policy model representing a Rego policy file
//! - [`Bundle`] - Compiled policy bundle for distribution
//! - [`AuthorizationDecision`] - Result of policy evaluation
//! - [`PolicyInput`] - Input schema for authorization requests
//! - [`CallerIdentity`] - Identity types (SPIFFE, User, `ApiKey`, Anonymous)
//! - [`signing`] - Ed25519 bundle signing and verification
//!
//! ## Example
//!
//! ```rust
//! use eunomia_core::{PolicyInput, CallerIdentity, AuthorizationDecision};
//!
//! // Create a policy input for authorization
//! let input = PolicyInput::builder()
//!     .caller(CallerIdentity::user("user-123", vec!["admin".to_string()]))
//!     .service("users-service")
//!     .operation_id("getUser")
//!     .method("GET")
//!     .path("/users/user-123")
//!     .build();
//! ```

#![doc = include_str!("../README.md")]

pub mod bundle;
pub mod decision;
pub mod error;
pub mod identity;
pub mod input;
pub mod policy;
pub mod signing;
pub mod validation;

#[cfg(test)]
mod proptest_tests;

// Re-export main types at crate root
pub use bundle::Bundle;
pub use decision::AuthorizationDecision;
pub use error::{Error, Result};
pub use identity::CallerIdentity;
pub use input::PolicyInput;
pub use policy::Policy;
pub use signing::{BundleSigner, BundleVerifier, SignedBundle, SigningError, SigningKeyPair};
pub use validation::{Validate, ValidationError};
