//! Runtime logging: stderr + optional file + in-memory ring for the Logs UI.

use std::collections::VecDeque;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
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
static FILE_LOG: Mutex<Option<File>> = Mutex::new(None);
static FILE_PATH: Mutex<Option<PathBuf>> = Mutex::new(None);

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

        let line = format!(
            "{} [{:>5}] {}: {}\n",
            now.format("%Y-%m-%d %H:%M:%S%.3f"),
            level,
            target,
            message
        );

        // Console (visible when running from a terminal / `pnpm tauri dev`).
        let _ = write!(std::io::stderr(), "{line}");

        if let Some(file) = FILE_LOG.lock().as_mut() {
            let _ = file.write_all(line.as_bytes());
            let _ = file.flush();
        }

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

    fn flush(&self) {
        if let Some(file) = FILE_LOG.lock().as_mut() {
            let _ = file.flush();
        }
    }
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

/// Append logs to `{data_dir}/logs/cluster-runtime.log` (GUI installs have no console).
pub fn attach_file_log(data_dir: &Path) {
    let dir = data_dir.join("logs");
    if let Err(e) = std::fs::create_dir_all(&dir) {
        log::warn!("logging: cannot create {}: {e}", dir.display());
        return;
    }
    let path = dir.join("cluster-runtime.log");
    match OpenOptions::new().create(true).append(true).open(&path) {
        Ok(file) => {
            *FILE_LOG.lock() = Some(file);
            *FILE_PATH.lock() = Some(path.clone());
            log::info!("logging: writing to {}", path.display());
        }
        Err(e) => log::warn!("logging: cannot open {}: {e}", path.display()),
    }
}

pub fn file_log_path() -> Option<PathBuf> {
    FILE_PATH.lock().clone()
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
