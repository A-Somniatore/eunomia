//! Rollback command for reverting policy deployments.
//!
//! This command allows reverting a service's policy bundle to a previous version.
//! It supports multiple rollback strategies:
//! - Immediate: Rollback all instances at once
//! - Rolling: Rollback instances in batches
//! - Targeted: Rollback specific instances only
//!
//! # Example
//!
//! ```bash
//! # Rollback to previous version
//! eunomia rollback users-service
//!
//! # Rollback to specific version
//! eunomia rollback users-service --version 1.2.0
//!
//! # Rollback with rolling strategy
//! eunomia rollback users-service --strategy rolling
//!
//! # Dry-run to preview rollback
//! eunomia rollback users-service --dry-run
//! ```

use std::sync::Arc;
use std::time::Instant;

use anyhow::{anyhow, Context, Result};
use clap::Args;
use eunomia_audit::{AuditLogger, DistributionEvent, TracingBackend};
use eunomia_distributor::{
    config::{DiscoveryConfig, DistributorConfig},
    discovery::DiscoverySource,
    Distributor,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Arguments for the rollback command.
#[derive(Debug, Args)]
pub struct RollbackArgs {
    /// Service name to rollback
    #[arg(help = "Name of the service to rollback")]
    pub service: String,

    /// Target version to rollback to (defaults to previous version)
    #[arg(short, long, help = "Target version to rollback to")]
    pub version: Option<String>,

    /// Rollback strategy
    #[arg(
        short,
        long,
        default_value = "immediate",
        help = "Rollback strategy: immediate, rolling, or targeted"
    )]
    pub strategy: String,

    /// Control plane endpoint
    #[arg(
        long,
        env = "EUNOMIA_CONTROL_PLANE",
        default_value = "http://localhost:50052",
        help = "Control plane gRPC endpoint"
    )]
    pub control_plane: String,

    /// Reason for the rollback
    #[arg(short, long, help = "Reason for initiating rollback")]
    pub reason: Option<String>,

    /// Dry-run mode (preview without executing)
    #[arg(long, help = "Preview rollback without executing")]
    pub dry_run: bool,

    /// Force rollback even if health checks fail
    #[arg(long, help = "Force rollback even if pre-checks fail")]
    pub force: bool,

    /// Specific instance endpoints to rollback (targeted strategy only)
    #[arg(long, help = "Specific instances to rollback (for targeted strategy)")]
    pub instances: Option<Vec<String>>,

    /// Target endpoints (comma-separated host:port)
    #[arg(long, value_delimiter = ',')]
    pub endpoints: Vec<String>,

    /// Output format
    #[arg(
        short,
        long,
        default_value = "text",
        help = "Output format: text or json"
    )]
    pub output: String,

    /// Path to deployment state file
    #[arg(long, help = "Path to deployment state file")]
    pub state_file: Option<PathBuf>,
}

/// Rollback strategy types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RollbackStrategy {
    /// Rollback all instances at once
    Immediate,
    /// Rollback instances in batches
    Rolling,
    /// Rollback specific instances only
    Targeted,
}

impl std::fmt::Display for RollbackStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Immediate => write!(f, "immediate"),
            Self::Rolling => write!(f, "rolling"),
            Self::Targeted => write!(f, "targeted"),
        }
    }
}

/// Result of a rollback operation.
///
/// Used when the rollback is executed (not dry-run).
/// Will be constructed by Week 18 implementation.
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackResult {
    /// Service that was rolled back
    pub service: String,

    /// Version rolled back from
    pub from_version: String,

    /// Version rolled back to
    pub to_version: String,

    /// Strategy used
    pub strategy: String,

    /// Number of instances rolled back
    pub instances_rolled_back: usize,

    /// Number of instances failed
    pub instances_failed: usize,

    /// Total duration in milliseconds
    pub duration_ms: u64,

    /// Whether this was a dry run
    pub dry_run: bool,

    /// Rollback reason
    pub reason: Option<String>,
}

/// Rollback plan for dry-run output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackPlan {
    /// Service to rollback
    pub service: String,

    /// Current version
    pub current_version: String,

    /// Target version
    pub target_version: String,

    /// Strategy to use
    pub strategy: String,

    /// Instances to rollback
    pub instances: Vec<String>,

    /// Estimated duration
    pub estimated_duration_ms: u64,

    /// Pre-check warnings
    pub warnings: Vec<String>,
}

/// Version history entry for determining rollback targets.
///
/// Used to show available versions for rollback.
/// Will be populated from control plane in Week 18.
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionHistoryEntry {
    /// Version string
    pub version: String,

    /// Deployment timestamp
    pub deployed_at: String,

    /// Whether this version is healthy
    pub healthy: bool,

    /// Deployment ID
    pub deployment_id: String,
}

/// Runs the rollback command.
///
/// # Errors
///
/// Returns an error if:
/// - Cannot connect to control plane
/// - Service not found
/// - No previous version available
/// - Rollback fails
pub fn run(args: &RollbackArgs) -> Result<()> {
    let strategy = parse_strategy(&args.strategy)?;

    if args.dry_run {
        run_dry_run(args, strategy)
    } else {
        run_rollback(args, strategy);
        Ok(())
    }
}

fn parse_strategy(s: &str) -> Result<RollbackStrategy> {
    match s.to_lowercase().as_str() {
        "immediate" => Ok(RollbackStrategy::Immediate),
        "rolling" => Ok(RollbackStrategy::Rolling),
        "targeted" => Ok(RollbackStrategy::Targeted),
        _ => Err(anyhow!(
            "Unknown rollback strategy: {s}. Use immediate, rolling, or targeted."
        )),
    }
}

fn run_dry_run(args: &RollbackArgs, strategy: RollbackStrategy) -> Result<()> {
    // TODO: Connect to control plane and fetch deployment state
    // For now, return a placeholder plan

    let plan = RollbackPlan {
        service: args.service.clone(),
        current_version: "1.3.0".to_string(), // Would be fetched from control plane
        target_version: args.version.clone().unwrap_or_else(|| "1.2.0".to_string()),
        strategy: strategy.to_string(),
        instances: vec![
            "instance-1:50051".to_string(),
            "instance-2:50051".to_string(),
            "instance-3:50051".to_string(),
        ],
        estimated_duration_ms: match strategy {
            RollbackStrategy::Immediate => 5000,
            RollbackStrategy::Rolling => 30000,
            RollbackStrategy::Targeted => 10000,
        },
        warnings: vec![],
    };

    if args.output == "json" {
        println!("{}", serde_json::to_string_pretty(&plan)?);
    } else {
        println!("Rollback Plan (Dry Run)");
        println!("========================");
        println!("Service:         {}", plan.service);
        println!("Current Version: {}", plan.current_version);
        println!("Target Version:  {}", plan.target_version);
        println!("Strategy:        {}", plan.strategy);
        println!("Instances:       {}", plan.instances.len());
        for instance in &plan.instances {
            println!("  - {instance}");
        }
        println!("Est. Duration:   {}ms", plan.estimated_duration_ms);
        if !plan.warnings.is_empty() {
            println!("\nWarnings:");
            for warning in &plan.warnings {
                println!("  ⚠ {warning}");
            }
        }
        println!("\nUse without --dry-run to execute rollback.");
    }

    Ok(())
}

fn run_rollback(args: &RollbackArgs, strategy: RollbackStrategy) {
    // Execute rollback via Distributor
    let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");

    if let Err(e) = rt.block_on(execute_rollback(args, strategy)) {
        eprintln!("Rollback failed: {e:#}");
        std::process::exit(1);
    }
}

/// Execute the rollback operation asynchronously.
#[allow(clippy::too_many_lines)]
async fn execute_rollback(args: &RollbackArgs, strategy: RollbackStrategy) -> Result<()> {
    let start = Instant::now();

    // Build discovery configuration from endpoints
    let discovery_config = if args.endpoints.is_empty() {
        return Err(anyhow!(
            "No endpoints specified. Use --endpoints to provide target hosts."
        ));
    } else {
        DiscoveryConfig {
            source: DiscoverySource::Static {
                endpoints: args.endpoints.clone(),
            },
            refresh_interval: std::time::Duration::from_secs(30),
            cache_enabled: true,
            cache_ttl: std::time::Duration::from_secs(60),
        }
    };

    // Build distributor configuration
    let config = DistributorConfig::builder()
        .discovery(discovery_config)
        .build();

    // Create distributor
    let distributor = Distributor::new(config)
        .await
        .context("Failed to create distributor")?;

    // Determine target version
    let target_version = args
        .version
        .as_deref()
        .ok_or_else(|| anyhow!("Target version required. Use --version to specify."))?;

    // Initialize audit logger
    let audit_logger = AuditLogger::builder()
        .with_backend(Arc::new(TracingBackend::new()))
        .build();

    // Emit rollback started event
    let from_version = "unknown"; // Would come from state tracking in production
    let started_event =
        DistributionEvent::rollback_started(&args.service, from_version, target_version);
    if let Err(e) = audit_logger.log(&started_event) {
        tracing::warn!("Failed to emit rollback started audit event: {e}");
    }

    println!("Starting rollback...");
    println!("  Service:        {}", args.service);
    println!("  Target Version: {target_version}");
    println!("  Strategy:       {strategy}");
    println!("  Endpoints:      {:?}", args.endpoints);
    if let Some(reason) = &args.reason {
        println!("  Reason:         {reason}");
    }
    println!();

    // Execute rollback
    let result = distributor
        .rollback(&args.service, target_version)
        .await
        .context("Rollback operation failed")?;

    let duration = start.elapsed();

    // Emit rollback completed event
    let completed_event =
        DistributionEvent::rollback_completed(&args.service, target_version, result.failed == 0);
    if let Err(e) = audit_logger.log(&completed_event) {
        tracing::warn!("Failed to emit rollback completed audit event: {e}");
    }

    // Collect error messages from failed instances
    let errors: Vec<String> = result
        .instance_results
        .iter()
        .filter_map(|r| {
            if let eunomia_distributor::InstanceResultStatus::Failed(msg) = &r.status {
                Some(format!("{}: {}", r.instance_id, msg))
            } else {
                None
            }
        })
        .collect();

    let total_instances = result.successful + result.failed + result.skipped;

    // Output results
    if args.output == "json" {
        #[allow(clippy::cast_possible_truncation)]
        let output = RollbackResult {
            service: args.service.clone(),
            from_version: "unknown".to_string(), // Would come from state tracking
            to_version: target_version.to_string(),
            strategy: strategy.to_string(),
            instances_rolled_back: result.successful,
            instances_failed: result.failed,
            duration_ms: duration.as_millis() as u64,
            dry_run: false,
            reason: args.reason.clone(),
        };
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!("Rollback Complete");
        println!("=================");
        println!("Service:             {}", args.service);
        println!("Target Version:      {target_version}");
        println!("Strategy:            {strategy}");
        println!("Duration:            {}ms", duration.as_millis());
        println!();
        println!("Results:");
        println!("  Total Instances:   {total_instances}");
        println!("  Succeeded:         {}", result.successful);
        println!("  Failed:            {}", result.failed);

        if !errors.is_empty() {
            println!();
            println!("Errors:");
            for error in &errors {
                println!("  ✗ {error}");
            }
        }

        println!();
        if result.failed == 0 {
            println!("✓ Rollback completed successfully.");
        } else {
            println!("⚠ Rollback completed with {} failures.", result.failed);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_strategy_immediate() {
        let strategy = parse_strategy("immediate").unwrap();
        assert_eq!(strategy, RollbackStrategy::Immediate);
    }

    #[test]
    fn test_parse_strategy_rolling() {
        let strategy = parse_strategy("rolling").unwrap();
        assert_eq!(strategy, RollbackStrategy::Rolling);
    }

    #[test]
    fn test_parse_strategy_targeted() {
        let strategy = parse_strategy("targeted").unwrap();
        assert_eq!(strategy, RollbackStrategy::Targeted);
    }

    #[test]
    fn test_parse_strategy_case_insensitive() {
        assert_eq!(
            parse_strategy("IMMEDIATE").unwrap(),
            RollbackStrategy::Immediate
        );
        assert_eq!(
            parse_strategy("Rolling").unwrap(),
            RollbackStrategy::Rolling
        );
        assert_eq!(
            parse_strategy("TARGETED").unwrap(),
            RollbackStrategy::Targeted
        );
    }

    #[test]
    fn test_parse_strategy_unknown() {
        let result = parse_strategy("invalid");
        assert!(result.is_err());
    }

    #[test]
    fn test_rollback_strategy_display() {
        assert_eq!(RollbackStrategy::Immediate.to_string(), "immediate");
        assert_eq!(RollbackStrategy::Rolling.to_string(), "rolling");
        assert_eq!(RollbackStrategy::Targeted.to_string(), "targeted");
    }

    #[test]
    fn test_rollback_result_serialization() {
        let result = RollbackResult {
            service: "users-service".to_string(),
            from_version: "1.3.0".to_string(),
            to_version: "1.2.0".to_string(),
            strategy: "immediate".to_string(),
            instances_rolled_back: 3,
            instances_failed: 0,
            duration_ms: 5000,
            dry_run: false,
            reason: Some("Performance regression".to_string()),
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("users-service"));
        assert!(json.contains("1.2.0"));
        assert!(json.contains("Performance regression"));
    }

    #[test]
    fn test_rollback_plan_serialization() {
        let plan = RollbackPlan {
            service: "orders-service".to_string(),
            current_version: "2.0.0".to_string(),
            target_version: "1.9.0".to_string(),
            strategy: "rolling".to_string(),
            instances: vec!["host1:50051".to_string(), "host2:50051".to_string()],
            estimated_duration_ms: 30000,
            warnings: vec!["Some instances may be slow".to_string()],
        };

        let json = serde_json::to_string(&plan).unwrap();
        assert!(json.contains("orders-service"));
        assert!(json.contains("rolling"));
        assert!(json.contains("host1:50051"));
    }
}
