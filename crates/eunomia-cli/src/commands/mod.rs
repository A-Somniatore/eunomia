//! CLI commands and argument parsing.

pub mod build;
pub mod test;
pub mod validate;

use clap::{Parser, Subcommand};

/// Eunomia - Authorization Policy Platform for Themis
#[derive(Parser)]
#[command(name = "eunomia")]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

/// Available commands.
#[derive(Subcommand)]
pub enum Commands {
    /// Run policy tests
    Test(test::TestArgs),

    /// Build a policy bundle
    Build(build::BuildArgs),

    /// Validate policies
    Validate(validate::ValidateArgs),

    /// Print version information
    Version,
}
