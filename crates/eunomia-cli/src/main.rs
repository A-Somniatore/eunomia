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
        Commands::Test(args) => commands::test::run(&args),
        Commands::Build(args) => commands::build::run(args),
        Commands::Sign(args) => commands::sign::run(&args),
        Commands::Publish(args) => commands::publish::run(&args),
        Commands::Fetch(args) => commands::fetch::run(&args),
        Commands::Validate(args) => commands::validate::run(&args),
        Commands::Push(args) => commands::push::execute(args).await,
        Commands::Status(args) => commands::status::run(&args),
        Commands::Version => {
            println!("eunomia {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
    }
}
