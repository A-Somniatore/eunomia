//! Sign command implementation.

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Args;
use tracing::info;

use eunomia_core::signing::{BundleSigner, SigningKeyPair};
use eunomia_core::Bundle;

/// Arguments for the sign command.
#[derive(Args)]
pub struct SignArgs {
    /// Path to the bundle file to sign
    #[arg(required_unless_present = "generate_key")]
    pub bundle: Option<PathBuf>,

    /// Path to the private key file (base64-encoded Ed25519 key)
    #[arg(short, long)]
    pub key_file: Option<PathBuf>,

    /// Private key from environment variable `EUNOMIA_SIGNING_KEY`
    #[arg(long, env = "EUNOMIA_SIGNING_KEY", hide_env_values = true)]
    pub key: Option<String>,

    /// Key ID to include in the signature
    #[arg(short = 'i', long, default_value = "default")]
    pub key_id: String,

    /// Output path for signed bundle (defaults to overwriting input)
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Generate a new signing key pair and print it
    #[arg(long)]
    pub generate_key: bool,
}

/// Runs the sign command.
pub fn run(args: &SignArgs) -> Result<()> {
    // Handle key generation mode
    if args.generate_key {
        generate_key();
        return Ok(());
    }

    // Get the bundle path (required when not generating keys)
    let bundle_path = args
        .bundle
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Bundle path is required"))?;

    info!(bundle = ?bundle_path, key_id = %args.key_id, "Signing bundle");

    // Load the private key
    let key_base64 = if let Some(key_file) = &args.key_file {
        fs::read_to_string(key_file)
            .with_context(|| format!("Failed to read key file: {}", key_file.display()))?
    } else if let Some(key) = &args.key {
        key.clone()
    } else {
        anyhow::bail!(
            "No signing key provided. Use --key-file or --key (EUNOMIA_SIGNING_KEY env var)"
        );
    };

    // Create the bundle signer
    let bundle_signer = BundleSigner::from_base64(&key_base64, args.key_id.clone())
        .context("Invalid signing key format")?;

    // Ensure bundle exists
    if !bundle_path.exists() {
        anyhow::bail!("Bundle file does not exist: {}", bundle_path.display());
    }

    println!("Eunomia Bundle Signer");
    println!("=====================");
    println!("Bundle: {}", bundle_path.display());
    println!("Key ID: {}", args.key_id);
    println!();

    // Load the bundle
    print!("Loading bundle... ");
    let bundle = Bundle::from_file(bundle_path).context("Failed to load bundle")?;
    println!("✓");

    println!("  Name:    {}", bundle.name);
    println!("  Version: {}", bundle.version);
    println!();

    // Sign the bundle
    print!("Signing bundle... ");
    let signed_bundle = bundle_signer.sign(&bundle);
    println!("✓");

    // Write the signed bundle
    let output_path = args.output.as_ref().unwrap_or(bundle_path);
    print!("Writing signed bundle to {}... ", output_path.display());

    // For now, write the signature to a separate file
    let sig_file = output_path.with_extension("sig");
    let sig_json = signed_bundle
        .signatures
        .to_json()
        .context("Failed to serialize signature")?;

    fs::write(&sig_file, &sig_json).context("Failed to write signature file")?;
    println!("✓");

    // Also update the bundle with signature info if it's a new output
    if args.output.is_some() {
        // Copy the original bundle to the new location
        fs::copy(bundle_path, output_path).context("Failed to copy bundle")?;
    }

    println!();
    println!("Bundle signed successfully!");
    println!("  Signature: {}", sig_file.display());
    println!(
        "  Algorithm: {}",
        signed_bundle
            .signatures
            .signatures
            .first()
            .map_or("unknown", |s| s.algorithm.as_str())
    );
    println!("  Key ID:    {}", args.key_id);

    Ok(())
}

/// Generates a new signing key pair and prints it.
fn generate_key() {
    println!("Generating new Ed25519 signing key pair...");
    println!();

    let key_pair = SigningKeyPair::generate();

    println!("Private Key (keep this secret!):");
    println!("{}", key_pair.to_base64());
    println!();
    println!("Public Key (distribute to verifiers):");
    println!("{}", key_pair.public_key_base64());
    println!();
    println!("Usage:");
    println!("  1. Save the private key to a file (e.g., signing.key)");
    println!("  2. Sign bundles with: eunomia sign bundle.tar.gz --key-file signing.key");
    println!("  3. Distribute the public key to services that verify bundles");
}
