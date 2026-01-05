//! Test command implementation.

use std::path::PathBuf;

use anyhow::Result;
use clap::Args;
use tracing::info;

use eunomia_test::{ConsoleReporter, Reporter, TestConfig, TestDiscovery, TestRunner};

/// Arguments for the test command.
#[derive(Args)]
#[allow(clippy::struct_excessive_bools)]
pub struct TestArgs {
    /// Path to policies directory or specific policy file
    #[arg(default_value = "policies")]
    pub path: PathBuf,

    /// Fail on first test failure
    #[arg(short, long)]
    pub fail_fast: bool,

    /// Run tests in parallel
    #[arg(short, long)]
    pub parallel: bool,

    /// Number of parallel workers
    #[arg(short, long, default_value = "4")]
    pub workers: usize,

    /// Output format (console, json)
    #[arg(short, long, default_value = "console")]
    pub output: String,

    /// Disable colored output
    #[arg(long)]
    pub no_color: bool,

    /// Filter tests by pattern (matches package or test name)
    #[arg(long)]
    pub filter: Option<String>,

    /// Verbose output
    #[arg(short, long)]
    pub verbose: bool,
}

/// Runs the test command.
pub fn run(args: &TestArgs) -> Result<()> {
    info!(path = ?args.path, "Running policy tests");

    let path = args
        .path
        .canonicalize()
        .unwrap_or_else(|_| args.path.clone());

    // Discover tests
    if args.verbose {
        println!("Discovering tests in: {}", path.display());
    }

    let discovery = TestDiscovery::new();
    let suite = match discovery.discover(path.to_str().unwrap_or(".")) {
        Ok(s) => s,
        Err(e) => {
            anyhow::bail!("Failed to discover tests: {e}");
        }
    };

    // Filter tests if pattern provided
    let test_count = suite.test_count();
    let policy_count = suite.policy_files().len();

    if test_count == 0 {
        println!("No tests found in {}", path.display());
        println!("\nLooking for files matching '*_test.rego' pattern.");
        return Ok(());
    }

    if args.verbose {
        println!("Found {test_count} tests in {policy_count} policy files");
        println!();
        for (pkg, tests) in suite.tests_by_package() {
            println!("Package: {pkg}");
            for test in tests {
                println!("  - {}", test.name);
            }
        }
        println!();
    }

    // Configure and run tests
    let config = TestConfig::new()
        .with_fail_fast(args.fail_fast)
        .with_parallel(args.parallel)
        .with_workers(args.workers);

    let runner = TestRunner::new(config);
    let results = match runner.run_suite(&suite) {
        Ok(r) => r,
        Err(e) => {
            anyhow::bail!("Test execution failed: {e}");
        }
    };

    // Report results
    let reporter = ConsoleReporter::new().with_colors(!args.no_color);
    reporter.report(&results)?;

    if results.all_passed() {
        Ok(())
    } else {
        anyhow::bail!("{} test(s) failed", results.failed())
    }
}
