//! Publish command implementation.
//!
//! Publishes policy bundles to an OCI-compatible registry.

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Args;
use tracing::info;

use eunomia_core::Bundle;
use eunomia_registry::{RegistryAuth, RegistryClient, RegistryConfig};

/// Arguments for the publish command.
#[derive(Args)]
pub struct PublishArgs {
    /// Path to the bundle file to publish
    #[arg(required = true)]
    pub bundle: PathBuf,

    /// Registry URL (e.g., `<https://registry.example.com>`)
    #[arg(short, long, env = "EUNOMIA_REGISTRY_URL")]
    pub registry: String,

    /// Service name (repository name in the registry)
    #[arg(short, long)]
    pub service: Option<String>,

    /// Version tag for the bundle (defaults to bundle version)
    #[arg(short, long)]
    pub version: Option<String>,

    /// Namespace prefix for the repository
    #[arg(short, long, env = "EUNOMIA_REGISTRY_NAMESPACE")]
    pub namespace: Option<String>,

    /// Bearer token for authentication
    #[arg(long, env = "EUNOMIA_REGISTRY_TOKEN", hide_env_values = true)]
    pub token: Option<String>,

    /// Username for basic authentication
    #[arg(short, long, env = "EUNOMIA_REGISTRY_USERNAME")]
    pub username: Option<String>,

    /// Password for basic authentication
    #[arg(long, env = "EUNOMIA_REGISTRY_PASSWORD", hide_env_values = true)]
    pub password: Option<String>,

    /// Skip confirmation prompt
    #[arg(long)]
    pub yes: bool,

    /// Request timeout in seconds
    #[arg(long, default_value = "60")]
    pub timeout: u64,
}

/// Runs the publish command.
///
/// # Errors
///
/// Returns an error if:
/// - The bundle cannot be loaded
/// - Registry authentication fails
/// - The upload fails
pub fn run(args: &PublishArgs) -> Result<()> {
    // Use tokio runtime for async operations
    let rt = tokio::runtime::Runtime::new().context("Failed to create async runtime")?;
    rt.block_on(run_async(args))
}

async fn run_async(args: &PublishArgs) -> Result<()> {
    info!(bundle = ?args.bundle, registry = %args.registry, "Publishing bundle");

    // Ensure bundle exists
    if !args.bundle.exists() {
        anyhow::bail!("Bundle file does not exist: {}", args.bundle.display());
    }

    println!("Eunomia Bundle Publisher");
    println!("========================");
    println!();

    // Load the bundle
    print!("Loading bundle... ");
    let bundle = Bundle::from_file(&args.bundle).context("Failed to load bundle")?;
    println!("✓");

    let service = args.service.clone().unwrap_or_else(|| bundle.name.clone());

    let version = args
        .version
        .clone()
        .unwrap_or_else(|| format!("v{}", bundle.version));

    println!();
    println!("Bundle Information:");
    println!("  Name:    {}", bundle.name);
    println!("  Version: {}", bundle.version);
    println!();
    println!("Registry Information:");
    println!("  URL:       {}", args.registry);
    println!("  Service:   {service}");
    println!("  Tag:       {version}");
    if let Some(ref ns) = args.namespace {
        println!("  Namespace: {ns}");
    }
    println!();

    // Confirm unless --yes is passed
    if !args.yes {
        use std::io::{self, Write};
        print!("Publish this bundle? [y/N] ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Aborted.");
            return Ok(());
        }
    }

    // Build registry configuration
    let mut config = RegistryConfig::new(&args.registry)
        .with_timeout(std::time::Duration::from_secs(args.timeout));

    if let Some(ref ns) = args.namespace {
        config = config.with_namespace(ns);
    }

    // Set authentication
    let auth = determine_auth(args)?;
    config = config.with_auth(auth);

    // Create client
    print!("Connecting to registry... ");
    let client = RegistryClient::new(config).context("Failed to create registry client")?;
    println!("✓");

    // Publish bundle
    print!("Uploading bundle... ");
    let digest = client
        .publish(&service, &version, &bundle)
        .await
        .context("Failed to publish bundle")?;
    println!("✓");

    println!();
    println!("Bundle published successfully!");
    println!();
    println!("  Digest: {digest}");
    println!("  Pull:   {}/{service}:{version}", args.registry);
    println!();
    println!("To fetch this bundle:");
    println!(
        "  eunomia fetch --registry {} --service {service} --version {version}",
        args.registry
    );

    Ok(())
}

/// Determines the authentication method from CLI arguments.
fn determine_auth(args: &PublishArgs) -> Result<RegistryAuth> {
    if let Some(ref token) = args.token {
        return Ok(RegistryAuth::Bearer {
            token: token.clone(),
        });
    }

    if let (Some(ref username), Some(ref password)) = (&args.username, &args.password) {
        return Ok(RegistryAuth::Basic {
            username: username.clone(),
            password: password.clone(),
        });
    }

    if args.username.is_some() || args.password.is_some() {
        anyhow::bail!("Both --username and --password are required for basic authentication");
    }

    // No auth - useful for local development
    Ok(RegistryAuth::None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_determine_auth_none() {
        let args = PublishArgs {
            bundle: PathBuf::from("test.bundle"),
            registry: "https://registry.example.com".to_string(),
            service: None,
            version: None,
            namespace: None,
            token: None,
            username: None,
            password: None,
            yes: false,
            timeout: 60,
        };

        let auth = determine_auth(&args).unwrap();
        assert!(matches!(auth, RegistryAuth::None));
    }

    #[test]
    fn test_determine_auth_bearer() {
        let args = PublishArgs {
            bundle: PathBuf::from("test.bundle"),
            registry: "https://registry.example.com".to_string(),
            service: None,
            version: None,
            namespace: None,
            token: Some("test-token".to_string()),
            username: None,
            password: None,
            yes: false,
            timeout: 60,
        };

        let auth = determine_auth(&args).unwrap();
        match auth {
            RegistryAuth::Bearer { token } => assert_eq!(token, "test-token"),
            _ => panic!("Expected Bearer auth"),
        }
    }

    #[test]
    fn test_determine_auth_basic() {
        let args = PublishArgs {
            bundle: PathBuf::from("test.bundle"),
            registry: "https://registry.example.com".to_string(),
            service: None,
            version: None,
            namespace: None,
            token: None,
            username: Some("user".to_string()),
            password: Some("pass".to_string()),
            yes: false,
            timeout: 60,
        };

        let auth = determine_auth(&args).unwrap();
        match auth {
            RegistryAuth::Basic { username, password } => {
                assert_eq!(username, "user");
                assert_eq!(password, "pass");
            }
            _ => panic!("Expected Basic auth"),
        }
    }

    #[test]
    fn test_determine_auth_incomplete_basic() {
        let args = PublishArgs {
            bundle: PathBuf::from("test.bundle"),
            registry: "https://registry.example.com".to_string(),
            service: None,
            version: None,
            namespace: None,
            token: None,
            username: Some("user".to_string()),
            password: None,
            yes: false,
            timeout: 60,
        };

        let result = determine_auth(&args);
        assert!(result.is_err());
    }

    #[test]
    fn test_determine_auth_bearer_takes_precedence() {
        let args = PublishArgs {
            bundle: PathBuf::from("test.bundle"),
            registry: "https://registry.example.com".to_string(),
            service: None,
            version: None,
            namespace: None,
            token: Some("bearer-token".to_string()),
            username: Some("user".to_string()),
            password: Some("pass".to_string()),
            yes: false,
            timeout: 60,
        };

        let auth = determine_auth(&args).unwrap();
        assert!(matches!(auth, RegistryAuth::Bearer { .. }));
    }
}
