use chrono::{DateTime, Utc};
use rand::Rng;
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

fn generate_series(name: &str, unit: &str, base: f64, variance: f64, count: usize) -> MetricSeries {
    let mut rng = rand::thread_rng();
    let now = Utc::now();
    let points: Vec<MetricPoint> = (0..count)
        .map(|i| {
            let offset = (count - 1 - i) as i64;
            let noise: f64 = rng.gen_range(-variance..variance);
            MetricPoint {
                timestamp: now - chrono::Duration::seconds(offset * 5),
                value: (base + noise).clamp(0.0, 100.0),
            }
        })
        .collect();

    MetricSeries {
        name: name.to_string(),
        unit: unit.to_string(),
        points,
    }
}

pub fn mock_metrics() -> MetricsSnapshot {
    // TODO: Get real metrics from metrics system
    MetricsSnapshot {
        cpu: MetricSeries {
            name: "CPU".to_string(),
            unit: "%".to_string(),
            points: Vec::new(),
        },
        memory: MetricSeries {
            name: "Memory".to_string(),
            unit: "%".to_string(),
            points: Vec::new(),
        },
        network: MetricSeries {
            name: "Network".to_string(),
            unit: "MB/s".to_string(),
            points: Vec::new(),
        },
        disk: MetricSeries {
            name: "Disk I/O".to_string(),
            unit: "MB/s".to_string(),
            points: Vec::new(),
        },
        collected_at: Utc::now(),
    }
}
