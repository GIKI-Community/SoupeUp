use std::collections::HashMap;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use thiserror::Error;

// ─── Execution ────────────────────────────────────────────────────────────────

/// Structured result returned from every code or script execution.
/// Callers must never receive raw strings — they get this.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionResult {
    /// Everything written to stdout during execution.
    pub stdout: String,
    /// Everything written to stderr during execution.
    pub stderr: String,
    /// Process exit code. 0 means success.
    pub exit_code: i32,
    /// Wall-clock duration of the execution in milliseconds.
    pub execution_time_ms: u64,
    /// Optional structured return value (future: extract via helper module).
    pub return_value: Option<String>,
    /// Full Python traceback text, if the process exited with an error.
    pub exception: Option<String>,
    /// True when exit_code == 0.
    pub success: bool,
}

/// Configuration injected into every execution call.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionContext {
    /// Directory the Python process will be launched from.
    pub working_directory: Option<PathBuf>,
    /// Additional environment variables to merge into the process environment.
    pub env_vars: HashMap<String, String>,
    /// Extra command-line arguments passed after the script/module name.
    pub args: Vec<String>,
    /// Hard timeout in seconds. None disables the timeout.
    pub timeout_secs: Option<u64>,
    /// Optional text to send to stdin.
    pub stdin: Option<String>,
}

// ─── Packages ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageInfo {
    pub name: String,
    pub version: String,
    /// Filesystem path to the package (from `pip show`).
    pub location: String,
}

// ─── Environments ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentInfo {
    pub name: String,
    pub path: PathBuf,
    pub python_version: Option<String>,
    pub package_count: usize,
    /// Whether this is the currently active environment.
    pub active: bool,
}

// ─── Runtime Health ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum RuntimeStatus {
    Initializing,
    Ready,
    Degraded,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PythonRuntimeHealth {
    pub status: RuntimeStatus,
    pub python_version: Option<String>,
    pub active_environment: Option<String>,
    pub environment_path: Option<PathBuf>,
    pub interpreter_path: Option<PathBuf>,
    /// Bundled Python was found at this path (vs system Python).
    pub is_bundled: bool,
}

// ─── Errors ───────────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum PythonError {
    #[error("Python interpreter not found: {0}")]
    InterpreterNotFound(String),

    #[error("Virtual environment error: {0}")]
    EnvironmentError(String),

    #[error("Execution error: {0}")]
    ExecutionError(String),

    #[error("Package manager error: {0}")]
    PackageError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Execution timed out after {0} seconds")]
    Timeout(u64),

    #[error("Service not yet initialized — Python runtime is still starting up")]
    NotInitialized,

    #[error("JSON parsing error: {0}")]
    JsonError(String),
}

pub type PythonResult<T> = Result<T, PythonError>;
