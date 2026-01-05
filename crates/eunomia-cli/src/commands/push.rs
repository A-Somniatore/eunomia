//! Push command for deploying policies to Archimedes instances.
//!
//! This command uses the distributor to push policy bundles to target instances
//! using various deployment strategies.

use std::time::Duration;

use anyhow::{Context, Result};
use clap::Args;
use eunomia_distributor::{
    config::{DiscoveryConfig, DistributorConfig},
    discovery::DiscoverySource,
    strategy::DeploymentStrategy,
    Distributor,
};

/// Arguments for the push command.
#[derive(Args)]
pub struct PushArgs {
    /// Target service name
    #[arg(short, long)]
    pub service: String,

    /// Policy version to deploy
    #[arg(short, long)]
    pub version: String,

    /// Deployment strategy: immediate, canary, rolling
    #[arg(long, default_value = "immediate")]
    pub strategy: String,

    /// Target endpoints (comma-separated host:port)
    #[arg(long, value_delimiter = ',')]
    pub endpoints: Vec<String>,

    /// Canary percentage (for canary strategy)
    #[arg(long, default_value = "10")]
    pub canary_percentage: u32,

    /// Canary validation duration in seconds (for canary strategy)
    #[arg(long, default_value = "300")]
    pub canary_duration: u64,

    /// Batch size (for rolling strategy)
    #[arg(long, default_value = "1")]
    pub batch_size: usize,

    /// Batch delay in seconds (for rolling strategy)
    #[arg(long, default_value = "30")]
    pub batch_delay: u64,

    /// Maximum concurrent deployments
    #[arg(long, default_value = "10")]
    pub max_concurrent: usize,

    /// Enable auto-rollback on failure
    #[arg(long)]
    pub auto_rollback: bool,

    /// Maximum failures before stopping
    #[arg(long, default_value = "1")]
    pub max_failures: u32,

    /// Dry run - don't actually push
    #[arg(long)]
    pub dry_run: bool,

    /// Output format: text, json
    #[arg(long, default_value = "text")]
    pub output: String,
}

/// Execute the push command.
pub async fn execute(args: PushArgs) -> Result<()> {
    if args.endpoints.is_empty() {
        anyhow::bail!("At least one endpoint must be specified with --endpoints");
    }

    print_deployment_header(&args);

    // Parse strategy
    let strategy = parse_strategy(&args)?;
    println!("   Strategy config: {:?}", strategy.strategy_type());

    // Create distributor config
    let config = DistributorConfig {
        discovery: DiscoveryConfig {
            source: DiscoverySource::Static {
                endpoints: args.endpoints.clone(),
            },
            ..Default::default()
        },
        ..Default::default()
    };

    if args.dry_run {
        print_dry_run(&args.endpoints);
        return Ok(());
    }

    // Create distributor and deploy
    let distributor = Distributor::new(config)
        .await
        .context("Failed to create distributor")?;

    let result = distributor
        .deploy(&args.service, &args.version, strategy)
        .await
        .context("Deployment failed")?;

    // Output results
    if args.output == "json" {
        print_json_output(&result)?;
    } else {
        print_text_output(&result);
    }

    Ok(())
}

fn print_deployment_header(args: &PushArgs) {
    println!("🚀 Deploying policy to {} instances...", args.endpoints.len());
    println!("   Service: {}", args.service);
    println!("   Version: {}", args.version);
    println!("   Strategy: {}", args.strategy);

    if args.dry_run {
        println!("\n⚠️  DRY RUN - No actual deployment will be performed\n");
    }
}

fn print_dry_run(endpoints: &[String]) {
    println!("\n📋 Would deploy to:");
    for (i, endpoint) in endpoints.iter().enumerate() {
        println!("   {}. {}", i + 1, endpoint);
    }
    println!("\n✅ Dry run complete");
}

fn print_json_output(result: &eunomia_distributor::DeploymentResult) -> Result<()> {
    let json = serde_json::to_string_pretty(&ResultOutput {
        deployment_id: result.deployment_id.clone(),
        successful: result.successful,
        failed: result.failed,
        skipped: result.skipped,
        instance_results: result
            .instance_results
            .iter()
            .map(|r| {
                let (status, message) = match &r.status {
                    eunomia_distributor::InstanceResultStatus::Success => {
                        ("Success".to_string(), None)
                    }
                    eunomia_distributor::InstanceResultStatus::Failed(msg) => {
                        ("Failed".to_string(), Some(msg.clone()))
                    }
                    eunomia_distributor::InstanceResultStatus::Skipped => {
                        ("Skipped".to_string(), None)
                    }
                };
                InstanceResultOutput {
                    instance_id: r.instance_id.clone(),
                    status,
                    message,
                }
            })
            .collect(),
    })?;
    println!("{json}");
    Ok(())
}

fn print_text_output(result: &eunomia_distributor::DeploymentResult) {
    println!("\n📊 Deployment Results:");
    println!("   Deployment ID: {}", result.deployment_id);
    println!(
        "   ✅ Successful: {} / {}",
        result.successful,
        result.successful + result.failed + result.skipped
    );
    if result.failed > 0 {
        println!("   ❌ Failed: {}", result.failed);
    }
    if result.skipped > 0 {
        println!("   ⏭️  Skipped: {}", result.skipped);
    }

    if !result.instance_results.is_empty() {
        println!("\n   Instance Details:");
        for r in &result.instance_results {
            let (status_icon, message) = match &r.status {
                eunomia_distributor::InstanceResultStatus::Success => ("✅", "OK".to_string()),
                eunomia_distributor::InstanceResultStatus::Failed(msg) => ("❌", msg.clone()),
                eunomia_distributor::InstanceResultStatus::Skipped => ("⏭️", "Skipped".to_string()),
            };
            println!("   {} {} - {}", status_icon, r.instance_id, message);
        }
    }

    if result.is_fully_successful() {
        println!("\n🎉 Deployment completed successfully!");
    } else {
        println!("\n⚠️  Deployment completed with issues");
    }
}

fn parse_strategy(args: &PushArgs) -> Result<DeploymentStrategy> {
    let mut strategy = match args.strategy.to_lowercase().as_str() {
        "immediate" => DeploymentStrategy::immediate(),
        "canary" => DeploymentStrategy::canary(
            args.canary_percentage,
            Duration::from_secs(args.canary_duration),
        ),
        "rolling" => {
            DeploymentStrategy::rolling(args.batch_size, Duration::from_secs(args.batch_delay))
        }
        other => anyhow::bail!(
            "Unknown strategy '{other}'. Use: immediate, canary, or rolling"
        ),
    };

    if args.auto_rollback {
        strategy = strategy.with_auto_rollback(true);
    }

    if args.max_failures > 0 {
        strategy = strategy.with_max_failures(args.max_failures);
    }

    Ok(strategy)
}

#[derive(serde::Serialize)]
struct ResultOutput {
    deployment_id: String,
    successful: usize,
    failed: usize,
    skipped: usize,
    instance_results: Vec<InstanceResultOutput>,
}

#[derive(serde::Serialize)]
struct InstanceResultOutput {
    instance_id: String,
    status: String,
    message: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_strategy_immediate() {
        let args = PushArgs {
            service: "test".to_string(),
            version: "1.0.0".to_string(),
            strategy: "immediate".to_string(),
            endpoints: vec!["localhost:8080".to_string()],
            canary_percentage: 10,
            canary_duration: 300,
            batch_size: 1,
            batch_delay: 30,
            max_concurrent: 10,
            auto_rollback: false,
            max_failures: 1,
            dry_run: false,
            output: "text".to_string(),
        };

        let strategy = parse_strategy(&args).unwrap();
        assert_eq!(
            strategy.strategy_type(),
            eunomia_distributor::strategy::StrategyType::Immediate
        );
    }

    #[test]
    fn test_parse_strategy_canary() {
        let args = PushArgs {
            service: "test".to_string(),
            version: "1.0.0".to_string(),
            strategy: "canary".to_string(),
            endpoints: vec!["localhost:8080".to_string()],
            canary_percentage: 20,
            canary_duration: 600,
            batch_size: 1,
            batch_delay: 30,
            max_concurrent: 10,
            auto_rollback: true,
            max_failures: 2,
            dry_run: false,
            output: "text".to_string(),
        };

        let strategy = parse_strategy(&args).unwrap();
        assert_eq!(
            strategy.strategy_type(),
            eunomia_distributor::strategy::StrategyType::Canary
        );
    }

    #[test]
    fn test_parse_strategy_rolling() {
        let args = PushArgs {
            service: "test".to_string(),
            version: "1.0.0".to_string(),
            strategy: "rolling".to_string(),
            endpoints: vec!["localhost:8080".to_string()],
            canary_percentage: 10,
            canary_duration: 300,
            batch_size: 5,
            batch_delay: 60,
            max_concurrent: 10,
            auto_rollback: false,
            max_failures: 1,
            dry_run: false,
            output: "text".to_string(),
        };

        let strategy = parse_strategy(&args).unwrap();
        assert_eq!(
            strategy.strategy_type(),
            eunomia_distributor::strategy::StrategyType::Rolling
        );
    }

    #[test]
    fn test_parse_strategy_unknown() {
        let args = PushArgs {
            service: "test".to_string(),
            version: "1.0.0".to_string(),
            strategy: "unknown".to_string(),
            endpoints: vec!["localhost:8080".to_string()],
            canary_percentage: 10,
            canary_duration: 300,
            batch_size: 1,
            batch_delay: 30,
            max_concurrent: 10,
            auto_rollback: false,
            max_failures: 1,
            dry_run: false,
            output: "text".to_string(),
        };

        let result = parse_strategy(&args);
        assert!(result.is_err());
    }
}
