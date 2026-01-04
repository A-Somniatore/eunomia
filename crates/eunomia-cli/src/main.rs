//! Eunomia CLI - Command-line interface for the Eunomia authorization platform.

use anyhow::Result;
use clap::Parser;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod commands;

use commands::{Cli, Commands};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "eunomia=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Test(args) => commands::test::run(args).await,
        Commands::Build(args) => commands::build::run(args).await,
        Commands::Validate(args) => commands::validate::run(args).await,
        Commands::Version => {
            println!("eunomia {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
    }
}
