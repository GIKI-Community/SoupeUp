//! # Python Runtime Plugin
//!
//! Provides embedded Python 3.13 management for the Cluster Runtime platform.
//!
//! ## Module Structure
//!
//! ```text
//! python_runtime/
//!   types/        — Shared data types (ExecutionResult, PackageInfo, …)
//!   utils/        — Shared helpers (path resolution, subprocess runner, …)
//!   interpreter/  — Python discovery (bundled → system PATH → future download)
//!   environment/  — Virtual environment lifecycle (create / delete / activate)
//!   pip/          — Package management (install / uninstall / list / upgrade)
//!   execution/    — Code and script execution (code string / file / module / dir)
//!   services/     — PythonExecutionService (the public API for other plugins)
//!   plugin/       — PluginApi registration handle
//!   tests/        — Integration test suite
//! ```
//!
//! ## Design Principles
//!
//! * **Scheduler-agnostic**: this module has no knowledge of Dask, Ray, Celery,
//!   or any distributed computing framework.  Those become consumers.
//! * **Subprocess-based execution**: Python runs as an isolated child process.
//!   `pyo3` embedding is intentionally avoided to remove version pinning and
//!   linker complexity.
//! * **Self-contained**: all environments live next to the application binary.
//! * **Single public surface**: callers interact only with `PythonExecutionService`.

pub mod environment;
pub mod execution;
pub mod interpreter;
pub mod pip;
pub mod plugin;
pub mod process;
pub mod services;
pub mod types;
pub mod utils;

#[cfg(test)]
pub mod tests;

pub use plugin::PythonRuntimePlugin;
pub use process::{BackgroundProcessInfo, ProcessStatus};
pub use services::PythonExecutionService;
pub use types::{
    EnvironmentInfo, ExecutionContext, ExecutionResult, PackageInfo,
    PythonError, PythonResult, PythonRuntimeHealth, RuntimeStatus,
};
