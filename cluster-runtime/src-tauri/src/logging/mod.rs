use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogEntry {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub module: String,
    pub level: LogLevel,
    pub message: String,
}

pub fn mock_logs() -> Vec<LogEntry> {
    let now = Utc::now();
    vec![
        LogEntry {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: now - chrono::Duration::seconds(5),
            module: "nodes".into(),
            level: LogLevel::Info,
            message: "Node alpha-workstation heartbeat received".into(),
        },
        LogEntry {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: now - chrono::Duration::seconds(12),
            module: "plugins".into(),
            level: LogLevel::Info,
            message: "Plugin 'Native Runtime' initialized successfully".into(),
        },
        LogEntry {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: now - chrono::Duration::seconds(28),
            module: "jobs".into(),
            level: LogLevel::Info,
            message: "Job job-a1b2c3 started on node-001".into(),
        },
        LogEntry {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: now - chrono::Duration::seconds(45),
            module: "network".into(),
            level: LogLevel::Warn,
            message: "Connection latency to zeta-compute exceeded threshold (320ms)".into(),
        },
        LogEntry {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: now - chrono::Duration::minutes(1),
            module: "nodes".into(),
            level: LogLevel::Error,
            message: "Node zeta-compute disconnected unexpectedly".into(),
        },
        LogEntry {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: now - chrono::Duration::minutes(2),
            module: "metrics".into(),
            level: LogLevel::Debug,
            message: "Metrics collection cycle completed in 42ms".into(),
        },
        LogEntry {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: now - chrono::Duration::minutes(3),
            module: "security".into(),
            level: LogLevel::Info,
            message: "Local certificate renewed successfully".into(),
        },
        LogEntry {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: now - chrono::Duration::minutes(5),
            module: "jobs".into(),
            level: LogLevel::Error,
            message: "Job job-m3n4o5 failed: resource allocation timeout".into(),
        },
        LogEntry {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: now - chrono::Duration::minutes(8),
            module: "plugins".into(),
            level: LogLevel::Warn,
            message: "Plugin 'HTCondor Adapter' is disabled but referenced by 2 nodes".into(),
        },
        LogEntry {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: now - chrono::Duration::minutes(12),
            module: "core".into(),
            level: LogLevel::Info,
            message: "Cluster Runtime started (v0.1.0)".into(),
        },
        LogEntry {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: now - chrono::Duration::minutes(15),
            module: "network".into(),
            level: LogLevel::Trace,
            message: "gRPC listener bound to 127.0.0.1:9470".into(),
        },
        LogEntry {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: now - chrono::Duration::minutes(20),
            module: "storage".into(),
            level: LogLevel::Debug,
            message: "SQLite WAL checkpoint completed".into(),
        },
    ]
}
