//! Build command implementation.

use std::path::PathBuf;

use anyhow::Result;
use clap::Args;
use tracing::info;

use eunomia_compiler::Bundler;
use eunomia_core::Policy;

/// Arguments for the build command.
#[derive(Args)]
pub struct BuildArgs {
    /// Path to policies directory
    #[arg(default_value = "policies")]
    pub path: PathBuf,

    /// Bundle name
    #[arg(short, long)]
    pub name: Option<String>,

    /// Bundle version
    #[arg(short, long)]
    pub version: String,

    /// Output directory for the bundle
    #[arg(short, long, default_value = "dist")]
    pub output: PathBuf,

    /// Git commit SHA to include in metadata
    #[arg(long)]
    pub git_commit: Option<String>,

    /// Optimize the bundle
    #[arg(long)]
    pub optimize: bool,

    /// Skip validation
    #[arg(long)]
    pub no_validate: bool,
}

/// Runs the build command.
pub async fn run(args: BuildArgs) -> Result<()> {
    info!(path = ?args.path, version = %args.version, "Building policy bundle");

    let bundle_name = args.name.unwrap_or_else(|| {
        args.path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("bundle")
            .to_string()
    });

    println!("Eunomia Bundle Builder");
    println!("======================");
    println!("Path: {:?}", args.path);
    println!("Name: {}", bundle_name);
    println!("Version: {}", args.version);
    println!("Output: {:?}", args.output);
    println!();

    // TODO: Implement actual policy loading from directory
    // For now, create a placeholder bundle

    let mut bundler = Bundler::new(&bundle_name)
        .version(&args.version)
        .with_optimization(args.optimize)
        .with_validation(!args.no_validate);

    if let Some(commit) = args.git_commit {
        bundler = bundler.git_commit(commit);
    }

    // Add placeholder policy for demonstration
    let placeholder_policy = Policy::new(
        format!("{}.authz", bundle_name.replace('-', "_")),
        format!(
            "package {}.authz\n\ndefault allow := false\n",
            bundle_name.replace('-', "_")
        ),
    );

    bundler = bundler.add_policy(placeholder_policy);

    let bundle = bundler.compile()?;

    println!("✓ Bundle compiled successfully");
    println!("  Name: {}", bundle.name);
    println!("  Version: {}", bundle.version);
    println!("  Policies: {}", bundle.policy_count());
    println!("  File: {}", bundle.file_name());

    // TODO: Write bundle to output directory

    Ok(())
}
