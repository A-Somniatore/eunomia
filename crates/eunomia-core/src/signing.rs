//! Bundle signing and verification using Ed25519.
//!
//! This module provides cryptographic signing and verification for policy bundles
//! using the Ed25519 signature algorithm. Bundle signing ensures:
//!
//! - **Authenticity**: Bundles come from a trusted source
//! - **Integrity**: Bundles haven't been tampered with
//!
//! # Architecture
//!
//! The signing process works as follows:
//!
//! 1. Compute the bundle's checksum (SHA-256 of canonical content)
//! 2. Sign the checksum with an Ed25519 private key
//! 3. Store the signature in `.signatures/.manifest.sig` format
//!
//! # Example
//!
//! ```rust
//! use eunomia_core::signing::{BundleSigner, BundleVerifier, SigningKeyPair};
//! use eunomia_core::Bundle;
//!
//! // Generate a new key pair (in practice, load from secure storage)
//! let key_pair = SigningKeyPair::generate();
//!
//! // Create a signer
//! let signer = BundleSigner::new(key_pair.signing_key().clone(), "prod-2026".to_string());
//!
//! // Sign a bundle
//! let bundle = Bundle::builder("my-service").version("1.0.0").build();
//! let signed = signer.sign(&bundle);
//!
//! // Create a verifier with the public key
//! let mut verifier = BundleVerifier::new();
//! verifier.add_public_key("prod-2026", key_pair.verifying_key().clone());
//!
//! // Verify the signature
//! assert!(verifier.verify(&signed).is_ok());
//! ```

use std::collections::HashMap;

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};

use crate::Bundle;

/// Errors that can occur during signing or verification.
#[derive(Debug, thiserror::Error)]
pub enum SigningError {
    /// The key ID was not found in the verifier's key set.
    #[error("unknown key ID: {0}")]
    UnknownKeyId(String),

    /// The signature is invalid.
    #[error("invalid signature")]
    InvalidSignature,

    /// Failed to decode base64 signature.
    #[error("failed to decode signature: {0}")]
    DecodeError(String),

    /// Invalid key format.
    #[error("invalid key format: {0}")]
    InvalidKeyFormat(String),
}

/// A signature on a bundle.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BundleSignature {
    /// The key ID that was used to create this signature.
    #[serde(rename = "keyid")]
    pub key_id: String,

    /// The signature algorithm (always "ed25519").
    pub algorithm: String,

    /// The base64-encoded signature value.
    pub value: String,
}

impl BundleSignature {
    /// Creates a new bundle signature.
    #[must_use]
    pub fn new(key_id: String, signature_bytes: &[u8]) -> Self {
        Self {
            key_id,
            algorithm: "ed25519".to_string(),
            value: BASE64.encode(signature_bytes),
        }
    }

    /// Decodes the signature value from base64.
    ///
    /// # Errors
    ///
    /// Returns an error if the base64 decoding fails.
    pub fn decode_value(&self) -> Result<Vec<u8>, SigningError> {
        BASE64
            .decode(&self.value)
            .map_err(|e| SigningError::DecodeError(e.to_string()))
    }
}

/// Collection of signatures for a bundle.
///
/// This matches the OPA bundle signature format stored in `.signatures/.manifest.sig`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SignatureFile {
    /// List of signatures on this bundle.
    pub signatures: Vec<BundleSignature>,
}

impl SignatureFile {
    /// Creates a new empty signature file.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a signature to the file.
    pub fn add_signature(&mut self, signature: BundleSignature) {
        self.signatures.push(signature);
    }

    /// Returns the number of signatures.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.signatures.len()
    }

    /// Returns true if there are no signatures.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.signatures.is_empty()
    }

    /// Serializes the signature file to JSON.
    ///
    /// # Errors
    ///
    /// Returns an error if JSON serialization fails.
    pub fn to_json(&self) -> Result<String, SigningError> {
        serde_json::to_string_pretty(self)
            .map_err(|e| SigningError::DecodeError(e.to_string()))
    }

    /// Deserializes a signature file from JSON.
    ///
    /// # Errors
    ///
    /// Returns an error if JSON parsing fails.
    pub fn from_json(json: &str) -> Result<Self, SigningError> {
        serde_json::from_str(json).map_err(|e| SigningError::DecodeError(e.to_string()))
    }
}

/// A signed bundle containing the original bundle and its signatures.
#[derive(Debug, Clone)]
pub struct SignedBundle {
    /// The original bundle.
    pub bundle: Bundle,

    /// The signatures on this bundle.
    pub signatures: SignatureFile,
}

impl SignedBundle {
    /// Creates a new signed bundle.
    #[must_use]
    pub const fn new(bundle: Bundle, signatures: SignatureFile) -> Self {
        Self { bundle, signatures }
    }

    /// Creates an unsigned bundle wrapper.
    #[must_use]
    pub fn unsigned(bundle: Bundle) -> Self {
        Self {
            bundle,
            signatures: SignatureFile::new(),
        }
    }

    /// Returns true if the bundle has at least one signature.
    #[must_use]
    pub const fn is_signed(&self) -> bool {
        !self.signatures.is_empty()
    }
}

/// An Ed25519 key pair for signing and verification.
#[derive(Debug)]
pub struct SigningKeyPair {
    signing_key: SigningKey,
}

impl SigningKeyPair {
    /// Generates a new random key pair.
    #[must_use]
    pub fn generate() -> Self {
        let signing_key = SigningKey::generate(&mut OsRng);
        Self { signing_key }
    }

    /// Creates a key pair from a 32-byte seed.
    ///
    /// # Errors
    ///
    /// Returns an error if the seed is not exactly 32 bytes.
    pub fn from_seed(seed: &[u8]) -> Result<Self, SigningError> {
        let seed_array: [u8; 32] = seed
            .try_into()
            .map_err(|_| SigningError::InvalidKeyFormat("seed must be 32 bytes".to_string()))?;
        let signing_key = SigningKey::from_bytes(&seed_array);
        Ok(Self { signing_key })
    }

    /// Creates a key pair from a base64-encoded private key.
    ///
    /// # Errors
    ///
    /// Returns an error if the key format is invalid.
    pub fn from_base64(encoded: &str) -> Result<Self, SigningError> {
        let bytes = BASE64
            .decode(encoded.trim())
            .map_err(|e| SigningError::InvalidKeyFormat(e.to_string()))?;
        Self::from_seed(&bytes)
    }

    /// Returns the signing (private) key.
    #[must_use]
    pub const fn signing_key(&self) -> &SigningKey {
        &self.signing_key
    }

    /// Returns the verifying (public) key.
    #[must_use]
    pub fn verifying_key(&self) -> VerifyingKey {
        self.signing_key.verifying_key()
    }

    /// Exports the private key as bytes.
    #[must_use]
    pub fn to_bytes(&self) -> [u8; 32] {
        self.signing_key.to_bytes()
    }

    /// Exports the private key as base64.
    #[must_use]
    pub fn to_base64(&self) -> String {
        BASE64.encode(self.to_bytes())
    }

    /// Exports the public key as bytes.
    #[must_use]
    pub fn public_key_bytes(&self) -> [u8; 32] {
        self.verifying_key().to_bytes()
    }

    /// Exports the public key as base64.
    #[must_use]
    pub fn public_key_base64(&self) -> String {
        BASE64.encode(self.public_key_bytes())
    }
}

/// Signs policy bundles with Ed25519.
#[derive(Debug)]
pub struct BundleSigner {
    signing_key: SigningKey,
    key_id: String,
}

impl BundleSigner {
    /// Creates a new bundle signer.
    #[must_use]
    pub const fn new(signing_key: SigningKey, key_id: String) -> Self {
        Self { signing_key, key_id }
    }

    /// Creates a signer from a key pair.
    #[must_use]
    pub fn from_key_pair(key_pair: &SigningKeyPair, key_id: String) -> Self {
        Self::new(key_pair.signing_key.clone(), key_id)
    }

    /// Creates a signer from a base64-encoded private key.
    ///
    /// # Errors
    ///
    /// Returns an error if the key format is invalid.
    pub fn from_base64(encoded: &str, key_id: String) -> Result<Self, SigningError> {
        let key_pair = SigningKeyPair::from_base64(encoded)?;
        Ok(Self::from_key_pair(&key_pair, key_id))
    }

    /// Returns the key ID for this signer.
    #[must_use]
    pub fn key_id(&self) -> &str {
        &self.key_id
    }

    /// Signs a bundle.
    ///
    /// This computes the bundle's checksum and signs it with the private key.
    #[must_use]
    pub fn sign(&self, bundle: &Bundle) -> SignedBundle {
        // Compute the canonical checksum to sign
        let checksum = bundle.compute_checksum();

        // Sign the checksum bytes (as UTF-8 hex string)
        let signature: Signature = self.signing_key.sign(checksum.as_bytes());

        // Create the signature record
        let bundle_sig = BundleSignature::new(self.key_id.clone(), &signature.to_bytes());

        let mut signatures = SignatureFile::new();
        signatures.add_signature(bundle_sig);

        SignedBundle::new(bundle.clone(), signatures)
    }

    /// Signs a checksum string directly.
    ///
    /// This is useful when you already have the checksum computed.
    #[must_use]
    pub fn sign_checksum(&self, checksum: &str) -> BundleSignature {
        let signature: Signature = self.signing_key.sign(checksum.as_bytes());
        BundleSignature::new(self.key_id.clone(), &signature.to_bytes())
    }
}

/// Verifies bundle signatures with Ed25519 public keys.
#[derive(Debug, Default)]
pub struct BundleVerifier {
    public_keys: HashMap<String, VerifyingKey>,
}

impl BundleVerifier {
    /// Creates a new empty verifier.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a public key for verification.
    pub fn add_public_key(&mut self, key_id: impl Into<String>, public_key: VerifyingKey) {
        self.public_keys.insert(key_id.into(), public_key);
    }

    /// Adds a public key from base64.
    ///
    /// # Errors
    ///
    /// Returns an error if the key format is invalid.
    pub fn add_public_key_base64(
        &mut self,
        key_id: impl Into<String>,
        encoded: &str,
    ) -> Result<(), SigningError> {
        let bytes = BASE64
            .decode(encoded.trim())
            .map_err(|e| SigningError::InvalidKeyFormat(e.to_string()))?;

        let key_bytes: [u8; 32] = bytes
            .try_into()
            .map_err(|_| SigningError::InvalidKeyFormat("public key must be 32 bytes".to_string()))?;

        let public_key = VerifyingKey::from_bytes(&key_bytes)
            .map_err(|e| SigningError::InvalidKeyFormat(e.to_string()))?;

        self.add_public_key(key_id, public_key);
        Ok(())
    }

    /// Returns the number of registered public keys.
    #[must_use]
    pub fn key_count(&self) -> usize {
        self.public_keys.len()
    }

    /// Verifies a signed bundle.
    ///
    /// Returns `Ok(())` if at least one signature is valid.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The bundle has no signatures
    /// - No signatures could be verified (unknown keys or invalid signatures)
    pub fn verify(&self, signed: &SignedBundle) -> Result<(), SigningError> {
        if signed.signatures.is_empty() {
            return Err(SigningError::InvalidSignature);
        }

        // Compute the expected checksum
        let checksum = signed.bundle.compute_checksum();

        // Try to verify at least one signature
        for sig in &signed.signatures.signatures {
            if self.verify_signature(&checksum, sig).is_ok() {
                return Ok(());
            }
        }

        Err(SigningError::InvalidSignature)
    }

    /// Verifies all signatures on a bundle.
    ///
    /// Returns a list of key IDs that successfully verified.
    ///
    /// # Errors
    ///
    /// Returns an error if no signatures could be verified.
    pub fn verify_all(&self, signed: &SignedBundle) -> Result<Vec<String>, SigningError> {
        let checksum = signed.bundle.compute_checksum();
        let mut verified_keys = Vec::new();

        for sig in &signed.signatures.signatures {
            if self.verify_signature(&checksum, sig).is_ok() {
                verified_keys.push(sig.key_id.clone());
            }
        }

        if verified_keys.is_empty() {
            Err(SigningError::InvalidSignature)
        } else {
            Ok(verified_keys)
        }
    }

    /// Verifies a single signature against a checksum.
    fn verify_signature(
        &self,
        checksum: &str,
        signature: &BundleSignature,
    ) -> Result<(), SigningError> {
        // Look up the public key
        let public_key = self
            .public_keys
            .get(&signature.key_id)
            .ok_or_else(|| SigningError::UnknownKeyId(signature.key_id.clone()))?;

        // Decode the signature
        let sig_bytes = signature.decode_value()?;
        let sig_array: [u8; 64] = sig_bytes
            .try_into()
            .map_err(|_| SigningError::InvalidSignature)?;
        let sig = Signature::from_bytes(&sig_array);

        // Verify
        public_key
            .verify(checksum.as_bytes(), &sig)
            .map_err(|_| SigningError::InvalidSignature)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper function to create a simple test bundle.
    fn test_bundle(name: &str, version: &str) -> Bundle {
        Bundle::builder(name)
            .version(version)
            .build()
    }

    /// Helper function to create a bundle with a policy.
    fn test_bundle_with_policy(name: &str, version: &str, pkg: &str, source: &str) -> Bundle {
        Bundle::builder(name)
            .version(version)
            .add_policy(pkg, source)
            .build()
    }

    #[test]
    fn test_generate_key_pair() {
        let key_pair = SigningKeyPair::generate();
        let public_key = key_pair.verifying_key();

        // Should be able to export and reimport
        let exported = key_pair.to_base64();
        let reimported = SigningKeyPair::from_base64(&exported).unwrap();

        assert_eq!(
            reimported.verifying_key().to_bytes(),
            public_key.to_bytes()
        );
    }

    #[test]
    fn test_sign_and_verify_bundle() {
        let key_pair = SigningKeyPair::generate();
        let signer = BundleSigner::from_key_pair(&key_pair, "test-key".to_string());

        let mut verifier = BundleVerifier::new();
        verifier.add_public_key("test-key", key_pair.verifying_key());

        let bundle = test_bundle_with_policy(
            "test-service",
            "1.0.0",
            "test.authz",
            "package test\ndefault allow := false",
        );

        let signed = signer.sign(&bundle);

        assert!(signed.is_signed());
        assert!(verifier.verify(&signed).is_ok());
    }

    #[test]
    fn test_verify_fails_with_wrong_key() {
        let key_pair1 = SigningKeyPair::generate();
        let key_pair2 = SigningKeyPair::generate();

        let signer = BundleSigner::from_key_pair(&key_pair1, "key1".to_string());

        let mut verifier = BundleVerifier::new();
        verifier.add_public_key("key1", key_pair2.verifying_key()); // Wrong key!

        let bundle = test_bundle("test-service", "1.0.0");
        let signed = signer.sign(&bundle);

        assert!(verifier.verify(&signed).is_err());
    }

    #[test]
    fn test_verify_fails_with_unknown_key_id() {
        let key_pair = SigningKeyPair::generate();
        let signer = BundleSigner::from_key_pair(&key_pair, "unknown-key".to_string());

        let mut verifier = BundleVerifier::new();
        verifier.add_public_key("different-key", key_pair.verifying_key());

        let bundle = test_bundle("test-service", "1.0.0");
        let signed = signer.sign(&bundle);

        let result = verifier.verify(&signed);
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_fails_with_tampered_bundle() {
        let key_pair = SigningKeyPair::generate();
        let signer = BundleSigner::from_key_pair(&key_pair, "test-key".to_string());

        let mut verifier = BundleVerifier::new();
        verifier.add_public_key("test-key", key_pair.verifying_key());

        // Create a bundle with actual content
        let bundle = test_bundle_with_policy(
            "test-service",
            "1.0.0",
            "test.authz",
            "package test\ndefault allow := false",
        );
        let mut signed = signer.sign(&bundle);

        // Tamper with the bundle (different policy content changes the checksum)
        signed.bundle = test_bundle_with_policy(
            "tampered-service",
            "1.0.0",
            "test.authz",
            "package test\ndefault allow := true",  // Different content!
        );

        assert!(verifier.verify(&signed).is_err());
    }

    #[test]
    fn test_unsigned_bundle() {
        let bundle = test_bundle("test-service", "1.0.0");
        let signed = SignedBundle::unsigned(bundle);

        assert!(!signed.is_signed());

        let verifier = BundleVerifier::new();
        assert!(verifier.verify(&signed).is_err());
    }

    #[test]
    fn test_signature_file_serialization() {
        let key_pair = SigningKeyPair::generate();
        let signer = BundleSigner::from_key_pair(&key_pair, "test-key".to_string());

        let bundle = test_bundle("test-service", "1.0.0");
        let signed = signer.sign(&bundle);

        let json = signed.signatures.to_json().unwrap();
        let parsed = SignatureFile::from_json(&json).unwrap();

        assert_eq!(parsed.signatures.len(), 1);
        assert_eq!(parsed.signatures[0].key_id, "test-key");
        assert_eq!(parsed.signatures[0].algorithm, "ed25519");
    }

    #[test]
    fn test_multiple_signatures() {
        let key_pair1 = SigningKeyPair::generate();
        let key_pair2 = SigningKeyPair::generate();

        let signer1 = BundleSigner::from_key_pair(&key_pair1, "key1".to_string());
        let signer2 = BundleSigner::from_key_pair(&key_pair2, "key2".to_string());

        let bundle = test_bundle("test-service", "1.0.0");
        let checksum = bundle.compute_checksum();

        let mut signatures = SignatureFile::new();
        signatures.add_signature(signer1.sign_checksum(&checksum));
        signatures.add_signature(signer2.sign_checksum(&checksum));

        let signed = SignedBundle::new(bundle, signatures);

        // Verifier with only key1
        let mut verifier1 = BundleVerifier::new();
        verifier1.add_public_key("key1", key_pair1.verifying_key());
        assert!(verifier1.verify(&signed).is_ok());

        // Verifier with only key2
        let mut verifier2 = BundleVerifier::new();
        verifier2.add_public_key("key2", key_pair2.verifying_key());
        assert!(verifier2.verify(&signed).is_ok());

        // Verifier with both keys
        let mut verifier_both = BundleVerifier::new();
        verifier_both.add_public_key("key1", key_pair1.verifying_key());
        verifier_both.add_public_key("key2", key_pair2.verifying_key());

        let verified = verifier_both.verify_all(&signed).unwrap();
        assert_eq!(verified.len(), 2);
    }

    #[test]
    fn test_public_key_from_base64() {
        let key_pair = SigningKeyPair::generate();
        let public_base64 = key_pair.public_key_base64();

        let mut verifier = BundleVerifier::new();
        verifier
            .add_public_key_base64("test-key", &public_base64)
            .unwrap();

        assert_eq!(verifier.key_count(), 1);
    }

    #[test]
    fn test_signer_from_base64() {
        let key_pair = SigningKeyPair::generate();
        let private_base64 = key_pair.to_base64();
        let public_key = key_pair.verifying_key();

        let signer = BundleSigner::from_base64(&private_base64, "test-key".to_string()).unwrap();

        let mut verifier = BundleVerifier::new();
        verifier.add_public_key("test-key", public_key);

        let bundle = test_bundle("test-service", "1.0.0");
        let signed = signer.sign(&bundle);

        assert!(verifier.verify(&signed).is_ok());
    }
}
