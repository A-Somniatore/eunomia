//! Build command implementation.

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Args;
use tracing::info;

use eunomia_compiler::Bundler;

/// Arguments for the build command.
#[derive(Args)]
pub struct BuildArgs {
    /// Path to policies directory
    #[arg(default_value = "policies")]
    pub path: PathBuf,

    /// Bundle name (defaults to directory name)
    #[arg(short, long)]
    pub name: Option<String>,

    /// Bundle version (required)
    #[arg(short, long)]
    pub version: String,

    /// Output path for the bundle file (defaults to dist/<name>-v<version>.bundle.tar.gz)
    #[arg(short, long)]
    pub output: Option<PathBuf>,

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
pub fn run(args: BuildArgs) -> Result<()> {
    info!(path = ?args.path, version = %args.version, "Building policy bundle");

    // Ensure policy path exists
    if !args.path.exists() {
        anyhow::bail!("Policy path does not exist: {}", args.path.display());
    }

    // Determine bundle name from path or argument
    let bundle_name = args.name.unwrap_or_else(|| {
        args.path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("bundle")
            .to_string()
    });

    // Determine output path
    let output_path = args.output.unwrap_or_else(|| {
        let output_dir = PathBuf::from("dist");
        output_dir.join(format!("{}-v{}.bundle.tar.gz", bundle_name, args.version))
    });

    println!("Eunomia Bundle Builder");
    println!("======================");
    println!("Path:    {}", args.path.display());
    println!("Name:    {bundle_name}");
    println!("Version: {}", args.version);
    println!("Output:  {}", output_path.display());
    println!();

    // Build the bundler
    let mut bundler = Bundler::new(&bundle_name)
        .version(&args.version)
        .with_optimization(args.optimize)
        .with_validation(!args.no_validate);

    if let Some(commit) = &args.git_commit {
        bundler = bundler.git_commit(commit);
    }

    // Load policies from directory
    print!("Loading policies from {}... ", args.path.display());
    bundler = bundler
        .add_policy_dir(&args.path)
        .context("Failed to load policies")?;
    println!("✓");

    // Also load data files if present
    print!("Loading data files... ");
    bundler = bundler
        .add_data_dir(&args.path)
        .context("Failed to load data files")?;
    println!("✓");

    // Create output directory if needed
    if let Some(parent) = output_path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent).context("Failed to create output directory")?;
        }
    }

    // Compile and write the bundle
    print!("Compiling bundle... ");
    let bundle = bundler
        .compile_to_file(&output_path)
        .context("Failed to compile bundle")?;
    println!("✓");

    println!();
    println!("Bundle built successfully!");
    println!("  Name:     {}", bundle.name);
    println!("  Version:  {}", bundle.version);
    println!("  Policies: {}", bundle.policy_count());
    println!("  Checksum: {}", bundle.compute_checksum());
    println!("  Output:   {}", output_path.display());

    Ok(())
}
