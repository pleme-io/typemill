use std::time::{Duration, Instant};

/// Helper for timing operations
pub struct PerformanceTimer {
    start: Instant,
    operation: String,
}

impl PerformanceTimer {
    pub fn new(operation: &str) -> Self {
        Self {
            start: Instant::now(),
            operation: operation.to_string(),
        }
    }

    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }

    pub fn finish(self) -> Duration {
        let duration = self.elapsed();
        println!("{} took: {:?}", self.operation, duration);
        duration
    }
}

/// Helper for verifying test results
pub struct ResultVerifier;

impl ResultVerifier {
    pub fn verify_range_valid(range: &serde_json::Value) -> bool {
        if let (Some(start), Some(end)) = (range.get("start"), range.get("end")) {
            if let (Some(start_line), Some(start_char), Some(end_line), Some(end_char)) = (
                start.get("line").and_then(|l| l.as_u64()),
                start.get("character").and_then(|c| c.as_u64()),
                end.get("line").and_then(|l| l.as_u64()),
                end.get("character").and_then(|c| c.as_u64()),
            ) {
                return start_line <= end_line && (start_line < end_line || start_char <= end_char);
            }
        }
        false
    }

    pub fn verify_performance_threshold(
        duration: Duration,
        threshold: Duration,
        operation: &str,
    ) -> bool {
        if duration > threshold {
            eprintln!(
                "Performance warning: {} took {:?}, expected < {:?}",
                operation, duration, threshold
            );
            false
        } else {
            true
        }
    }
}
