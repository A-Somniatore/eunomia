//! Fetch command implementation.
//!
//! Fetches policy bundles from an OCI-compatible registry.

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Args;
use tracing::info;

use eunomia_registry::{RegistryAuth, RegistryClient, RegistryConfig};

/// Arguments for the fetch command.
#[derive(Args)]
pub struct FetchArgs {
    /// Registry URL (e.g., `<https://registry.example.com>`)
    #[arg(short, long, env = "EUNOMIA_REGISTRY_URL")]
    pub registry: String,

    /// Service name (repository name in the registry)
    #[arg(short, long)]
    pub service: String,

    /// Version to fetch (tag, "latest", or digest)
    #[arg(short, long, default_value = "latest")]
    pub version: String,

    /// Output path for the bundle
    #[arg(short, long)]
    pub output: Option<PathBuf>,

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

    /// Request timeout in seconds
    #[arg(long, default_value = "60")]
    pub timeout: u64,

    /// Print bundle information without downloading
    #[arg(long)]
    pub info_only: bool,
}

/// Runs the fetch command.
///
/// # Errors
///
/// Returns an error if:
/// - Registry authentication fails
/// - The bundle cannot be fetched
/// - The output file cannot be written
pub fn run(args: &FetchArgs) -> Result<()> {
    // Use tokio runtime for async operations
    let rt = tokio::runtime::Runtime::new().context("Failed to create async runtime")?;
    rt.block_on(run_async(args))
}

async fn run_async(args: &FetchArgs) -> Result<()> {
    info!(
        registry = %args.registry,
        service = %args.service,
        version = %args.version,
        "Fetching bundle"
    );

    println!("Eunomia Bundle Fetcher");
    println!("======================");
    println!();

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

    // Resolve version
    print!("Resolving version '{}'... ", args.version);
    let resolved_version = client
        .resolve_version(&args.service, &args.version)
        .await
        .context("Failed to resolve version")?;
    println!("{resolved_version}");

    // Check if bundle exists
    print!("Checking bundle existence... ");
    if !client.exists(&args.service, &resolved_version).await? {
        println!("✗");
        anyhow::bail!(
            "Bundle not found: {}:{}",
            args.service,
            resolved_version
        );
    }
    println!("✓");

    if args.info_only {
        // Just list available tags
        println!();
        println!("Available versions:");
        let tags = client.list_tags(&args.service).await?;
        for tag in &tags {
            if tag == &resolved_version {
                println!("  * {tag} (selected)");
            } else {
                println!("    {tag}");
            }
        }
        return Ok(());
    }

    // Fetch bundle
    print!("Downloading bundle... ");
    let bundle = client
        .fetch(&args.service, &resolved_version)
        .await
        .context("Failed to fetch bundle")?;
    println!("✓");

    println!();
    println!("Bundle Information:");
    println!("  Name:    {}", bundle.name);
    println!("  Version: {}", bundle.version);
    println!("  Policies: {}", bundle.policies.len());

    // Determine output path
    let output_path = args.output.clone().unwrap_or_else(|| {
        PathBuf::from(format!(
            "{}-{}.bundle",
            args.service,
            resolved_version.replace(':', "_")
        ))
    });

    // Save bundle
    print!("Saving to {}... ", output_path.display());
    bundle
        .write_to_file(&output_path)
        .context("Failed to save bundle")?;
    println!("✓");

    println!();
    println!("Bundle fetched successfully!");
    println!("  Output: {}", output_path.display());

    Ok(())
}

/// Determines the authentication method from CLI arguments.
fn determine_auth(args: &FetchArgs) -> Result<RegistryAuth> {
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
        let args = FetchArgs {
            registry: "https://registry.example.com".to_string(),
            service: "test-service".to_string(),
            version: "latest".to_string(),
            output: None,
            namespace: None,
            token: None,
            username: None,
            password: None,
            timeout: 60,
            info_only: false,
        };

        let auth = determine_auth(&args).unwrap();
        assert!(matches!(auth, RegistryAuth::None));
    }

    #[test]
    fn test_determine_auth_bearer() {
        let args = FetchArgs {
            registry: "https://registry.example.com".to_string(),
            service: "test-service".to_string(),
            version: "latest".to_string(),
            output: None,
            namespace: None,
            token: Some("test-token".to_string()),
            username: None,
            password: None,
            timeout: 60,
            info_only: false,
        };

        let auth = determine_auth(&args).unwrap();
        match auth {
            RegistryAuth::Bearer { token } => assert_eq!(token, "test-token"),
            _ => panic!("Expected Bearer auth"),
        }
    }

    #[test]
    fn test_determine_auth_basic() {
        let args = FetchArgs {
            registry: "https://registry.example.com".to_string(),
            service: "test-service".to_string(),
            version: "latest".to_string(),
            output: None,
            namespace: None,
            token: None,
            username: Some("user".to_string()),
            password: Some("pass".to_string()),
            timeout: 60,
            info_only: false,
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
        let args = FetchArgs {
            registry: "https://registry.example.com".to_string(),
            service: "test-service".to_string(),
            version: "latest".to_string(),
            output: None,
            namespace: None,
            token: None,
            username: Some("user".to_string()),
            password: None,
            timeout: 60,
            info_only: false,
        };

        let result = determine_auth(&args);
        assert!(result.is_err());
    }
}
