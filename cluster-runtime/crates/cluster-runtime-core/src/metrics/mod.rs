use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetricPoint {
    pub timestamp: DateTime<Utc>,
    pub value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetricSeries {
    pub name: String,
    pub unit: String,
    pub points: Vec<MetricPoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetricsSnapshot {
    pub cpu: MetricSeries,
    pub memory: MetricSeries,
    pub network: MetricSeries,
    pub disk: MetricSeries,
    pub collected_at: DateTime<Utc>,
}

fn empty_series(name: &str, unit: &str) -> MetricSeries {
    MetricSeries {
        name: name.to_string(),
        unit: unit.to_string(),
        points: Vec::new(),
    }
}

/// Returns series metadata with no points — history is built client-side from zero.
pub fn mock_metrics() -> MetricsSnapshot {
    MetricsSnapshot {
        cpu: empty_series("CPU", "%"),
        memory: empty_series("Memory", "%"),
        network: empty_series("Network", "MB/s"),
        disk: empty_series("Disk I/O", "MB/s"),
        collected_at: Utc::now(),
    }
}
