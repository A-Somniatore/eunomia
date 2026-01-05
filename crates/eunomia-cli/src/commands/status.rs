//! Status command implementation.

use std::path::{Path, PathBuf};

use anyhow::Result;
use clap::Args;
use serde::{Deserialize, Serialize};
use tracing::info;

/// Arguments for the status command.
#[derive(Args)]
pub struct StatusArgs {
    /// Service name to check status for
    #[arg(short, long)]
    pub service: Option<String>,

    /// Path to the state directory
    #[arg(long, default_value = ".eunomia")]
    pub state_dir: PathBuf,

    /// Output format (text, json)
    #[arg(short, long, default_value = "text")]
    pub format: OutputFormat,

    /// Show verbose output
    #[arg(short, long)]
    pub verbose: bool,
}

/// Output format for status command.
#[derive(Clone, Debug, Default, clap::ValueEnum)]
pub enum OutputFormat {
    #[default]
    Text,
    Json,
}

/// Deployment status information.
#[derive(Debug, Serialize, Deserialize)]
pub struct DeploymentStatus {
    pub service: String,
    pub version: String,
    pub status: String,
    pub instances: Vec<InstanceStatus>,
    pub last_updated: String,
}

/// Instance status information.
#[derive(Debug, Serialize, Deserialize)]
pub struct InstanceStatus {
    pub endpoint: String,
    pub status: String,
    pub version: Option<String>,
    pub last_health_check: Option<String>,
}

/// Overall status summary.
#[derive(Debug, Serialize, Deserialize)]
pub struct StatusSummary {
    pub deployments: Vec<DeploymentStatus>,
    pub total_instances: usize,
    pub healthy_instances: usize,
    pub unhealthy_instances: usize,
}

/// Runs the status command.
pub fn run(args: &StatusArgs) -> Result<()> {
    info!(service = ?args.service, "Checking deployment status");

    // Read deployment state from state directory
    let summary = read_deployment_state(&args.state_dir, args.service.as_deref())?;

    match args.format {
        OutputFormat::Text => print_text_status(&summary, args.verbose),
        OutputFormat::Json => print_json_status(&summary)?,
    }

    Ok(())
}

fn read_deployment_state(
    state_dir: &Path,
    service_filter: Option<&str>,
) -> Result<StatusSummary> {
    let state_file = state_dir.join("deployments.json");

    // If no state file exists, return empty status
    if !state_file.exists() {
        return Ok(StatusSummary {
            deployments: Vec::new(),
            total_instances: 0,
            healthy_instances: 0,
            unhealthy_instances: 0,
        });
    }

    // Read and parse state file
    let content = std::fs::read_to_string(&state_file)?;
    let mut deployments: Vec<DeploymentStatus> = serde_json::from_str(&content)?;

    // Filter by service if specified
    if let Some(service) = service_filter {
        deployments.retain(|d| d.service == service);
    }

    // Calculate summary statistics
    let total_instances: usize = deployments.iter().map(|d| d.instances.len()).sum();
    let healthy_instances: usize = deployments
        .iter()
        .flat_map(|d| &d.instances)
        .filter(|i| i.status == "healthy")
        .count();
    let unhealthy_instances = total_instances - healthy_instances;

    Ok(StatusSummary {
        deployments,
        total_instances,
        healthy_instances,
        unhealthy_instances,
    })
}

fn print_text_status(summary: &StatusSummary, verbose: bool) {
    println!("Eunomia Deployment Status");
    println!("=========================");
    println!();

    if summary.deployments.is_empty() {
        println!("No deployments found.");
        println!();
        println!("Run 'eunomia push' to deploy a policy bundle.");
        return;
    }

    // Summary line
    println!(
        "Total: {} instances ({} healthy, {} unhealthy)",
        summary.total_instances, summary.healthy_instances, summary.unhealthy_instances
    );
    println!();

    for deployment in &summary.deployments {
        let status_icon = match deployment.status.as_str() {
            "deployed" => "✓",
            "deploying" => "⟳",
            "failed" => "✗",
            "rolling_back" => "⟲",
            _ => "?",
        };

        println!(
            "{} {} v{} [{}]",
            status_icon, deployment.service, deployment.version, deployment.status
        );
        println!("  Last updated: {}", deployment.last_updated);

        if verbose {
            println!("  Instances:");
            for instance in &deployment.instances {
                let instance_icon = match instance.status.as_str() {
                    "healthy" => "●",
                    "unhealthy" => "○",
                    _ => "?",
                };
                println!(
                    "    {} {} ({})",
                    instance_icon, instance.endpoint, instance.status
                );
                if let Some(version) = &instance.version {
                    println!("      Version: {version}");
                }
                if let Some(last_check) = &instance.last_health_check {
                    println!("      Last check: {last_check}");
                }
            }
        } else {
            let healthy = deployment
                .instances
                .iter()
                .filter(|i| i.status == "healthy")
                .count();
            let total = deployment.instances.len();
            println!("  Instances: {healthy}/{total} healthy");
        }
        println!();
    }
}

fn print_json_status(summary: &StatusSummary) -> Result<()> {
    let json = serde_json::to_string_pretty(summary)?;
    println!("{json}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_empty_state_returns_empty_summary() {
        let temp_dir = TempDir::new().unwrap();
        let summary = read_deployment_state(&temp_dir.path().to_path_buf(), None).unwrap();

        assert!(summary.deployments.is_empty());
        assert_eq!(summary.total_instances, 0);
        assert_eq!(summary.healthy_instances, 0);
        assert_eq!(summary.unhealthy_instances, 0);
    }

    #[test]
    fn test_read_deployment_state() {
        let temp_dir = TempDir::new().unwrap();
        let state_file = temp_dir.path().join("deployments.json");

        let deployments = vec![DeploymentStatus {
            service: "users-service".to_string(),
            version: "1.0.0".to_string(),
            status: "deployed".to_string(),
            instances: vec![
                InstanceStatus {
                    endpoint: "host1:8080".to_string(),
                    status: "healthy".to_string(),
                    version: Some("1.0.0".to_string()),
                    last_health_check: Some("2026-01-05T10:00:00Z".to_string()),
                },
                InstanceStatus {
                    endpoint: "host2:8080".to_string(),
                    status: "unhealthy".to_string(),
                    version: Some("1.0.0".to_string()),
                    last_health_check: Some("2026-01-05T10:00:00Z".to_string()),
                },
            ],
            last_updated: "2026-01-05T10:00:00Z".to_string(),
        }];

        std::fs::write(&state_file, serde_json::to_string(&deployments).unwrap()).unwrap();

        let summary = read_deployment_state(&temp_dir.path().to_path_buf(), None).unwrap();

        assert_eq!(summary.deployments.len(), 1);
        assert_eq!(summary.total_instances, 2);
        assert_eq!(summary.healthy_instances, 1);
        assert_eq!(summary.unhealthy_instances, 1);
    }

    #[test]
    fn test_filter_by_service() {
        let temp_dir = TempDir::new().unwrap();
        let state_file = temp_dir.path().join("deployments.json");

        let deployments = vec![
            DeploymentStatus {
                service: "users-service".to_string(),
                version: "1.0.0".to_string(),
                status: "deployed".to_string(),
                instances: vec![],
                last_updated: "2026-01-05T10:00:00Z".to_string(),
            },
            DeploymentStatus {
                service: "orders-service".to_string(),
                version: "2.0.0".to_string(),
                status: "deployed".to_string(),
                instances: vec![],
                last_updated: "2026-01-05T10:00:00Z".to_string(),
            },
        ];

        std::fs::write(&state_file, serde_json::to_string(&deployments).unwrap()).unwrap();

        let summary =
            read_deployment_state(&temp_dir.path().to_path_buf(), Some("users-service")).unwrap();

        assert_eq!(summary.deployments.len(), 1);
        assert_eq!(summary.deployments[0].service, "users-service");
    }
}
