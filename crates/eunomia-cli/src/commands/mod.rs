//! CLI commands and argument parsing.

pub mod build;
pub mod fetch;
pub mod publish;
pub mod push;
pub mod sign;
pub mod status;
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

    /// Sign a policy bundle
    Sign(sign::SignArgs),

    /// Publish a bundle to a registry
    Publish(publish::PublishArgs),

    /// Fetch a bundle from a registry
    Fetch(fetch::FetchArgs),

    /// Validate policies
    Validate(validate::ValidateArgs),

    /// Push a policy to Archimedes instances
    Push(push::PushArgs),

    /// Check deployment status
    Status(status::StatusArgs),

    /// Print version information
    Version,
}
