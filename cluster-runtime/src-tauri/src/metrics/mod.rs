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
    let mut rng = rand::thread_rng();
    MetricsSnapshot {
        cpu: generate_series("CPU", "%", 42.0 + rng.gen_range(-5.0..5.0), 8.0, 30),
        memory: generate_series("Memory", "%", 58.0 + rng.gen_range(-3.0..3.0), 5.0, 30),
        network: generate_series("Network", "MB/s", 125.0 + rng.gen_range(-20.0..20.0), 30.0, 30),
        disk: generate_series("Disk I/O", "MB/s", 45.0 + rng.gen_range(-10.0..10.0), 15.0, 30),
        collected_at: Utc::now(),
    }
}
