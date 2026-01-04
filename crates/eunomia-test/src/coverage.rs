//! Policy coverage analysis.
//!
//! This module provides coverage tracking for policy tests.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Coverage data for a policy.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CoverageReport {
    /// Coverage by file.
    pub files: HashMap<String, FileCoverage>,
    /// Overall coverage percentage.
    pub total_coverage: f64,
    /// Total lines.
    pub total_lines: usize,
    /// Covered lines.
    pub covered_lines: usize,
}

/// Coverage data for a single file.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FileCoverage {
    /// File path.
    pub path: String,
    /// Lines that were executed.
    pub covered_lines: Vec<usize>,
    /// Lines that were not executed.
    pub uncovered_lines: Vec<usize>,
    /// Total executable lines.
    pub total_lines: usize,
    /// Coverage percentage for this file.
    pub coverage_percent: f64,
}

impl CoverageReport {
    /// Creates a new empty coverage report.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds file coverage data.
    pub fn add_file(&mut self, file: FileCoverage) {
        self.covered_lines += file.covered_lines.len();
        self.total_lines += file.total_lines;
        self.files.insert(file.path.clone(), file);
        self.recalculate_total();
    }

    /// Recalculates total coverage.
    #[allow(clippy::cast_precision_loss)]
    fn recalculate_total(&mut self) {
        if self.total_lines > 0 {
            self.total_coverage = (self.covered_lines as f64 / self.total_lines as f64) * 100.0;
        }
    }

    /// Returns true if coverage meets the threshold.
    #[must_use]
    pub fn meets_threshold(&self, threshold: f64) -> bool {
        self.total_coverage >= threshold
    }
}

impl FileCoverage {
    /// Creates new file coverage data.
    #[must_use]
    pub fn new(path: impl Into<String>, total_lines: usize) -> Self {
        Self {
            path: path.into(),
            total_lines,
            covered_lines: Vec::new(),
            uncovered_lines: (1..=total_lines).collect(),
            coverage_percent: 0.0,
        }
    }

    /// Marks a line as covered.
    pub fn mark_covered(&mut self, line: usize) {
        if !self.covered_lines.contains(&line) && line <= self.total_lines {
            self.covered_lines.push(line);
            self.uncovered_lines.retain(|&l| l != line);
            self.recalculate_percent();
        }
    }

    /// Marks multiple lines as covered.
    pub fn mark_lines_covered(&mut self, lines: &[usize]) {
        for &line in lines {
            self.mark_covered(line);
        }
    }

    /// Recalculates coverage percentage.
    #[allow(clippy::cast_precision_loss)]
    fn recalculate_percent(&mut self) {
        if self.total_lines > 0 {
            self.coverage_percent =
                (self.covered_lines.len() as f64 / self.total_lines as f64) * 100.0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_coverage_new() {
        let coverage = FileCoverage::new("test.rego", 10);

        assert_eq!(coverage.path, "test.rego");
        assert_eq!(coverage.total_lines, 10);
        assert!(coverage.covered_lines.is_empty());
        assert_eq!(coverage.uncovered_lines.len(), 10);
        assert_eq!(coverage.coverage_percent, 0.0);
    }

    #[test]
    fn test_file_coverage_mark_covered() {
        let mut coverage = FileCoverage::new("test.rego", 10);

        coverage.mark_covered(1);
        coverage.mark_covered(5);
        coverage.mark_covered(10);

        assert_eq!(coverage.covered_lines.len(), 3);
        assert_eq!(coverage.uncovered_lines.len(), 7);
        assert_eq!(coverage.coverage_percent, 30.0);
    }

    #[test]
    fn test_file_coverage_mark_lines() {
        let mut coverage = FileCoverage::new("test.rego", 10);

        coverage.mark_lines_covered(&[1, 2, 3, 4, 5]);

        assert_eq!(coverage.covered_lines.len(), 5);
        assert_eq!(coverage.coverage_percent, 50.0);
    }

    #[test]
    fn test_coverage_report() {
        let mut report = CoverageReport::new();

        let mut file1 = FileCoverage::new("file1.rego", 10);
        file1.mark_lines_covered(&[1, 2, 3, 4, 5]); // 50%

        let mut file2 = FileCoverage::new("file2.rego", 10);
        file2.mark_lines_covered(&[1, 2, 3, 4, 5, 6, 7, 8]); // 80%

        report.add_file(file1);
        report.add_file(file2);

        assert_eq!(report.total_lines, 20);
        assert_eq!(report.covered_lines, 13);
        assert_eq!(report.total_coverage, 65.0);
    }

    #[test]
    fn test_coverage_threshold() {
        let mut report = CoverageReport::new();
        report.total_coverage = 80.0;

        assert!(report.meets_threshold(80.0));
        assert!(report.meets_threshold(70.0));
        assert!(!report.meets_threshold(90.0));
    }
}
