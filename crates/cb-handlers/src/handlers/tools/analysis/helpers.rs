// Shared analysis utilities
//
// This module provides common functionality used across analysis handlers:
// - File filtering and extension matching
// - Statistical aggregation (averages, sums, max values)
// - Workspace-wide collection patterns
//
// Used by: analyze.quality (workspace scope), analyze.dependencies, analyze.dead_code

use std::path::{Path, PathBuf};

/// Filter files by supported language extensions
pub fn filter_analyzable_files(
    files: &[String],
    base_path: &Path,
    supported_extensions: &[String],
) -> Vec<PathBuf> {
    files
        .iter()
        .filter_map(|file| {
            let path = if file.starts_with('/') {
                PathBuf::from(file)
            } else {
                base_path.join(file)
            };

            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if supported_extensions.contains(&ext.to_string()) {
                    return Some(path);
                }
            }
            None
        })
        .collect()
}

/// Calculate weighted average across multiple files
///
/// # Arguments
/// * `values` - Iterator of (value, weight) pairs
///
/// # Example
/// ```
/// // Average complexity weighted by function count
/// let avg = weighted_average(files.iter().map(|f| (f.avg_complexity, f.function_count)));
/// ```
pub fn weighted_average<I>(values: I) -> f64
where
    I: Iterator<Item = (f64, usize)>,
{
    let mut total_value = 0.0;
    let mut total_weight = 0;

    for (value, weight) in values {
        total_value += value * weight as f64;
        total_weight += weight;
    }

    if total_weight > 0 {
        total_value / total_weight as f64
    } else {
        0.0
    }
}

/// Aggregate statistics helper
#[derive(Debug, Clone, Default)]
pub struct AggregateStats {
    pub count: usize,
    pub sum: f64,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub average: f64,
}

impl AggregateStats {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, value: f64) {
        self.count += 1;
        self.sum += value;

        self.min = Some(match self.min {
            Some(min) if min < value => min,
            _ => value,
        });

        self.max = Some(match self.max {
            Some(max) if max > value => max,
            _ => value,
        });

        self.average = self.sum / self.count as f64;
    }

    pub fn merge(&mut self, other: &AggregateStats) {
        if other.count == 0 {
            return;
        }

        self.count += other.count;
        self.sum += other.sum;

        if let Some(other_min) = other.min {
            self.min = Some(match self.min {
                Some(min) if min < other_min => min,
                _ => other_min,
            });
        }

        if let Some(other_max) = other.max {
            self.max = Some(match self.max {
                Some(max) if max > other_max => max,
                _ => other_max,
            });
        }

        self.average = self.sum / self.count as f64;
    }
}

/// Workspace analysis context for multi-file operations
pub struct WorkspaceAnalysisContext {
    pub base_path: PathBuf,
    pub supported_extensions: Vec<String>,
    pub files: Vec<PathBuf>,
}

impl WorkspaceAnalysisContext {
    pub fn new(base_path: impl AsRef<Path>, supported_extensions: Vec<String>) -> Self {
        Self {
            base_path: base_path.as_ref().to_path_buf(),
            supported_extensions,
            files: Vec::new(),
        }
    }

    pub fn add_files(&mut self, raw_files: &[String]) {
        let filtered =
            filter_analyzable_files(raw_files, &self.base_path, &self.supported_extensions);
        self.files.extend(filtered);
    }

    pub fn file_count(&self) -> usize {
        self.files.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_analyzable_files() {
        let files = vec![
            "src/main.rs".to_string(),
            "src/lib.ts".to_string(),
            "README.md".to_string(),
        ];
        let base = Path::new("/project");
        let extensions = vec!["rs".to_string(), "ts".to_string()];

        let filtered = filter_analyzable_files(&files, base, &extensions);
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_weighted_average() {
        let values = vec![(5.0, 2), (10.0, 3), (15.0, 1)];
        let avg = weighted_average(values.into_iter());
        // (5*2 + 10*3 + 15*1) / (2+3+1) = 55 / 6 = 9.166...
        assert!((avg - 9.166).abs() < 0.01);
    }

    #[test]
    fn test_aggregate_stats() {
        let mut stats = AggregateStats::new();
        stats.add(5.0);
        stats.add(10.0);
        stats.add(15.0);

        assert_eq!(stats.count, 3);
        assert_eq!(stats.sum, 30.0);
        assert_eq!(stats.min, Some(5.0));
        assert_eq!(stats.max, Some(15.0));
        assert_eq!(stats.average, 10.0);
    }

    #[test]
    fn test_aggregate_stats_merge() {
        let mut stats1 = AggregateStats::new();
        stats1.add(5.0);
        stats1.add(10.0);

        let mut stats2 = AggregateStats::new();
        stats2.add(15.0);
        stats2.add(20.0);

        stats1.merge(&stats2);
        assert_eq!(stats1.count, 4);
        assert_eq!(stats1.sum, 50.0);
        assert_eq!(stats1.min, Some(5.0));
        assert_eq!(stats1.max, Some(20.0));
        assert_eq!(stats1.average, 12.5);
    }
}
