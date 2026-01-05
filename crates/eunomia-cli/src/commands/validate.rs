//! Validate command implementation.

use std::path::PathBuf;

use anyhow::Result;
use clap::Args;
use tracing::info;

use eunomia_compiler::{Analyzer, Parser};

/// Arguments for the validate command.
#[derive(Args)]
pub struct ValidateArgs {
    /// Path to policies directory or specific policy file
    #[arg(default_value = "policies")]
    pub path: PathBuf,

    /// Require default allow/deny rules
    #[arg(long, default_value = "true")]
    pub require_default: bool,

    /// Show detailed output
    #[arg(short, long)]
    pub verbose: bool,
}

/// Runs the validate command.
pub fn run(args: &ValidateArgs) -> Result<()> {
    info!(path = ?args.path, "Validating policies");

    println!("Eunomia Policy Validator");
    println!("========================");
    println!("Path: {}", args.path.display());
    println!();

    let parser = Parser::new();
    let analyzer = Analyzer::new().with_require_default(args.require_default);

    // Check if path is a file or directory
    if args.path.is_file() {
        validate_file(&args.path, &parser, &analyzer, args.verbose)?;
    } else if args.path.is_dir() {
        validate_directory(&args.path, &parser, &analyzer, args.verbose)?;
    } else {
        anyhow::bail!("Path does not exist: {}", args.path.display());
    }

    println!("\n✓ All policies validated successfully");
    Ok(())
}

fn validate_file(
    path: &PathBuf,
    parser: &Parser,
    analyzer: &Analyzer,
    verbose: bool,
) -> Result<()> {
    let policy = parser.parse_file(path)?;

    if verbose {
        println!("Validating: {}", path.display());
        println!("  Package: {}", policy.package_name);
    }

    let result = analyzer.analyze(&policy)?;

    if verbose {
        println!(
            "  Has default: {}",
            result.has_default_allow || result.has_default_deny
        );
        println!("  Imports: {}", result.imports.len());
        println!("  Rules: {}", result.rules.len());

        for warning in &result.warnings {
            println!("  ⚠ Warning: {}", warning.message);
        }
    }

    println!("✓ {}", path.display());
    Ok(())
}

fn validate_directory(
    path: &PathBuf,
    parser: &Parser,
    analyzer: &Analyzer,
    verbose: bool,
) -> Result<()> {
    let mut count = 0;
    let mut errors = Vec::new();

    for entry in walkdir(path)? {
        let entry_path = entry?;
        if entry_path.extension().is_some_and(|e| e == "rego") {
            match validate_file(&entry_path, parser, analyzer, verbose) {
                Ok(()) => count += 1,
                Err(e) => errors.push((entry_path, e)),
            }
        }
    }

    if !errors.is_empty() {
        println!("\nErrors:");
        for (path, error) in &errors {
            println!("✗ {}: {error}", path.display());
        }
        anyhow::bail!("{} validation errors", errors.len());
    }

    println!("\nValidated {count} policies");
    Ok(())
}

/// Simple directory walker (placeholder - would use walkdir crate in production).
fn walkdir(path: &PathBuf) -> Result<impl Iterator<Item = Result<PathBuf>>> {
    let entries: Vec<_> = std::fs::read_dir(path)?
        .filter_map(std::result::Result::ok)
        .map(|e| Ok(e.path()))
        .collect();

    Ok(entries.into_iter())
}
