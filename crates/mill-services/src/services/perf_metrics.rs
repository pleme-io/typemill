use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

#[derive(Debug, Clone, serde::Serialize)]
pub struct PerfMetricSummary {
    pub count: u64,
    pub min_ms: u128,
    pub max_ms: u128,
    pub avg_ms: f64,
    pub last_ms: u128,
}

#[derive(Debug, Default, Clone)]
struct PerfAccumulator {
    count: u64,
    total_ms: u128,
    min_ms: u128,
    max_ms: u128,
    last_ms: u128,
}

static PERF_METRICS: LazyLock<Mutex<HashMap<String, PerfAccumulator>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub fn record_metric(metric: &str, value_ms: u128) {
    let Ok(mut guard) = PERF_METRICS.lock() else {
        return;
    };
    let entry = guard
        .entry(metric.to_string())
        .or_insert_with(PerfAccumulator::default);

    if entry.count == 0 {
        entry.min_ms = value_ms;
        entry.max_ms = value_ms;
    } else {
        entry.min_ms = entry.min_ms.min(value_ms);
        entry.max_ms = entry.max_ms.max(value_ms);
    }

    entry.count += 1;
    entry.total_ms = entry.total_ms.saturating_add(value_ms);
    entry.last_ms = value_ms;
}

pub fn snapshot_metrics() -> HashMap<String, PerfMetricSummary> {
    let Ok(guard) = PERF_METRICS.lock() else {
        return HashMap::new();
    };

    guard
        .iter()
        .map(|(k, v)| {
            let avg_ms = if v.count == 0 {
                0.0
            } else {
                v.total_ms as f64 / v.count as f64
            };
            (
                k.clone(),
                PerfMetricSummary {
                    count: v.count,
                    min_ms: v.min_ms,
                    max_ms: v.max_ms,
                    avg_ms,
                    last_ms: v.last_ms,
                },
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_and_snapshots_metrics() {
        record_metric("unit.test.metric", 10);
        record_metric("unit.test.metric", 30);

        let snapshot = snapshot_metrics();
        let metric = snapshot.get("unit.test.metric").unwrap();
        assert_eq!(metric.count, 2);
        assert_eq!(metric.min_ms, 10);
        assert_eq!(metric.max_ms, 30);
        assert_eq!(metric.last_ms, 30);
        assert_eq!(metric.avg_ms, 20.0);
    }
}
