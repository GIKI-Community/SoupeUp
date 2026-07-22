//! Runtime logging: stderr + in-memory ring buffer for the Logs UI.

use std::collections::VecDeque;
use std::io::Write;
use std::sync::atomic::{AtomicU64, Ordering};

use chrono::Utc;
use log::{Level, LevelFilter, Log, Metadata, Record};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

const RING_CAPACITY: usize = 2000;

static SEQ: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl From<Level> for LogLevel {
    fn from(level: Level) -> Self {
        match level {
            Level::Error => LogLevel::Error,
            Level::Warn => LogLevel::Warn,
            Level::Info => LogLevel::Info,
            Level::Debug => LogLevel::Debug,
            Level::Trace => LogLevel::Trace,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogEntry {
    pub id: String,
    pub timestamp: chrono::DateTime<Utc>,
    pub module: String,
    pub level: LogLevel,
    pub message: String,
}

struct RingState {
    entries: VecDeque<LogEntry>,
}

static RING: Mutex<Option<RingState>> = Mutex::new(None);

struct AppLogger {
    max_level: LevelFilter,
}

impl Log for AppLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= self.max_level
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        let level = record.level();
        let target = record.target();
        let message = format!("{}", record.args());
        let now = Utc::now();

        // Console (visible when running from a terminal / `pnpm tauri dev`).
        let _ = writeln!(
            std::io::stderr(),
            "{} [{:>5}] {}: {}",
            now.format("%H:%M:%S%.3f"),
            level,
            target,
            message
        );

        let entry = LogEntry {
            id: format!(
                "{}-{}",
                SEQ.fetch_add(1, Ordering::Relaxed),
                Uuid::new_v4()
            ),
            timestamp: now,
            module: target.to_string(),
            level: level.into(),
            message,
        };

        let mut guard = RING.lock();
        let ring = guard.get_or_insert_with(|| RingState {
            entries: VecDeque::with_capacity(RING_CAPACITY),
        });
        if ring.entries.len() >= RING_CAPACITY {
            ring.entries.pop_front();
        }
        ring.entries.push_back(entry);
    }

    fn flush(&self) {}
}

/// Install the global logger (safe to call once; subsequent calls are ignored).
pub fn init() {
    let max_level = parse_level(
        std::env::var("RUST_LOG")
            .ok()
            .as_deref()
            .unwrap_or("info"),
    );

    let logger = AppLogger { max_level };
    // Ignore AlreadyInitialized — headless binary and GUI may both call init.
    let _ = log::set_boxed_logger(Box::new(logger));
    log::set_max_level(max_level);

    {
        let mut guard = RING.lock();
        if guard.is_none() {
            *guard = Some(RingState {
                entries: VecDeque::with_capacity(RING_CAPACITY),
            });
        }
    }

    log::info!(
        "logging: ready (level={max_level:?}, set RUST_LOG=debug for more detail)"
    );
}

fn parse_level(raw: &str) -> LevelFilter {
    // Support both "debug" and "cluster_runtime=debug,info".
    let last = raw
        .split(',')
        .last()
        .unwrap_or(raw)
        .split('=')
        .last()
        .unwrap_or("info")
        .trim();
    match last.to_ascii_lowercase().as_str() {
        "trace" => LevelFilter::Trace,
        "debug" => LevelFilter::Debug,
        "info" => LevelFilter::Info,
        "warn" | "warning" => LevelFilter::Warn,
        "error" => LevelFilter::Error,
        "off" => LevelFilter::Off,
        _ => LevelFilter::Info,
    }
}

/// Snapshot of recent log lines for the Logs UI / debugging.
pub fn recent_logs() -> Vec<LogEntry> {
    RING.lock()
        .as_ref()
        .map(|r| r.entries.iter().cloned().collect())
        .unwrap_or_default()
}

#[deprecated(note = "use recent_logs()")]
pub fn mock_logs() -> Vec<LogEntry> {
    recent_logs()
}
