//! Test result reporting.
//!
//! This module provides formatters for test results.

use std::fmt::Write as FmtWrite;
use std::io::{self, Write};

use crate::runner::{TestResult, TestResults};

/// Trait for reporting test results.
pub trait Reporter {
    /// Reports the results of a test run.
    ///
    /// # Errors
    ///
    /// Returns an IO error if writing to output fails.
    fn report(&self, results: &TestResults) -> io::Result<()>;
}

/// Console reporter that outputs results to stdout.
#[derive(Debug, Default)]
pub struct ConsoleReporter {
    /// Whether to use colors.
    use_colors: bool,
    /// Whether to show verbose output.
    verbose: bool,
}

impl ConsoleReporter {
    /// Creates a new console reporter.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            use_colors: true,
            verbose: false,
        }
    }

    /// Sets whether to use colors.
    #[must_use]
    pub const fn with_colors(mut self, colors: bool) -> Self {
        self.use_colors = colors;
        self
    }

    /// Sets verbose mode.
    #[must_use]
    pub const fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    /// Formats a passing test line.
    fn format_pass(&self, result: &TestResult) -> String {
        let checkmark = if self.use_colors {
            "\x1b[32m✓\x1b[0m"
        } else {
            "✓"
        };
        format!(
            "{} {} ({:.2}ms)",
            checkmark,
            result.name,
            result.duration.as_secs_f64() * 1000.0
        )
    }

    /// Formats a failing test line.
    fn format_fail(&self, result: &TestResult) -> String {
        let cross = if self.use_colors {
            "\x1b[31m✗\x1b[0m"
        } else {
            "✗"
        };
        let mut line = format!(
            "{} {} ({:.2}ms)",
            cross,
            result.name,
            result.duration.as_secs_f64() * 1000.0
        );

        if let Some(error) = &result.error {
            let _ = write!(line, "\n    Error: {error}");
        }

        if let (Some(expected), Some(actual)) = (&result.expected, &result.actual) {
            let _ = write!(line, "\n    Expected: {expected}");
            let _ = write!(line, "\n    Actual: {actual}");
        }

        line
    }

    /// Formats the summary line.
    fn format_summary(&self, results: &TestResults) -> String {
        let status = if results.all_passed() {
            if self.use_colors {
                "\x1b[32mPASSED\x1b[0m"
            } else {
                "PASSED"
            }
        } else if self.use_colors {
            "\x1b[31mFAILED\x1b[0m"
        } else {
            "FAILED"
        };

        format!(
            "\n{}: {} passed, {} failed, {} total ({:.2}s)",
            status,
            results.passed(),
            results.failed(),
            results.total(),
            results.total_duration.as_secs_f64()
        )
    }
}

impl Reporter for ConsoleReporter {
    fn report(&self, results: &TestResults) -> io::Result<()> {
        let mut stdout = io::stdout().lock();

        writeln!(stdout, "\nTest Results:")?;
        writeln!(stdout, "{}", "─".repeat(50))?;

        for result in &results.results {
            if result.passed {
                writeln!(stdout, "{}", self.format_pass(result))?;
            } else {
                writeln!(stdout, "{}", self.format_fail(result))?;
            }
        }

        writeln!(stdout, "{}", "─".repeat(50))?;
        writeln!(stdout, "{}", self.format_summary(results))?;

        Ok(())
    }
}

/// JSON reporter that outputs results as JSON.
#[derive(Debug, Default)]
pub struct JsonReporter {
    /// Whether to pretty-print the JSON.
    pretty: bool,
}

impl JsonReporter {
    /// Creates a new JSON reporter.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets whether to pretty-print.
    #[must_use]
    pub const fn with_pretty(mut self, pretty: bool) -> Self {
        self.pretty = pretty;
        self
    }
}

impl Reporter for JsonReporter {
    fn report(&self, results: &TestResults) -> io::Result<()> {
        let mut stdout = io::stdout().lock();

        let json = if self.pretty {
            serde_json::to_string_pretty(results)
        } else {
            serde_json::to_string(results)
        }
        .map_err(io::Error::other)?;

        writeln!(stdout, "{json}")?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn sample_results() -> TestResults {
        let mut results = TestResults::new();
        results.add(TestResult::pass(
            "test_admin_access",
            Duration::from_millis(5),
        ));
        results.add(TestResult::pass("test_user_read", Duration::from_millis(3)));
        results.add(TestResult::fail(
            "test_anonymous_denied",
            Duration::from_millis(4),
            "Expected deny, got allow",
        ));
        results.total_duration = Duration::from_millis(12);
        results
    }

    #[test]
    fn test_console_reporter_format_pass() {
        let reporter = ConsoleReporter::new().with_colors(false);
        let result = TestResult::pass("my_test", Duration::from_millis(10));

        let formatted = reporter.format_pass(&result);
        assert!(formatted.contains("✓"));
        assert!(formatted.contains("my_test"));
    }

    #[test]
    fn test_console_reporter_format_fail() {
        let reporter = ConsoleReporter::new().with_colors(false);
        let result = TestResult::fail("my_test", Duration::from_millis(10), "error message")
            .with_comparison("true", "false");

        let formatted = reporter.format_fail(&result);
        assert!(formatted.contains("✗"));
        assert!(formatted.contains("my_test"));
        assert!(formatted.contains("error message"));
        assert!(formatted.contains("Expected: true"));
        assert!(formatted.contains("Actual: false"));
    }

    #[test]
    fn test_console_reporter_summary_passed() {
        let reporter = ConsoleReporter::new().with_colors(false);
        let mut results = TestResults::new();
        results.add(TestResult::pass("test1", Duration::from_millis(5)));
        results.total_duration = Duration::from_millis(5);

        let summary = reporter.format_summary(&results);
        assert!(summary.contains("PASSED"));
        assert!(summary.contains("1 passed"));
        assert!(summary.contains("0 failed"));
    }

    #[test]
    fn test_console_reporter_summary_failed() {
        let reporter = ConsoleReporter::new().with_colors(false);
        let results = sample_results();

        let summary = reporter.format_summary(&results);
        assert!(summary.contains("FAILED"));
        assert!(summary.contains("2 passed"));
        assert!(summary.contains("1 failed"));
    }
}
