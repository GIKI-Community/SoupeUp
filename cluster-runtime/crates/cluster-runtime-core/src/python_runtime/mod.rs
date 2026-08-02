//! # Python Runtime Plugin
//!
//! Provides Python management for the Cluster Runtime platform.
//!
//! ## Module Structure
//!
//! ```text
//! python_runtime/
//!   types/        — Shared data types (ExecutionResult, PackageInfo, …)
//!   utils/        — Shared helpers (path resolution, subprocess runner, …)
//!   interpreter/  — Python discovery (system → managed → auto-download)
//!   download/     — Per-machine python-build-standalone fetch into data_dir
//!   environment/  — Virtual environment lifecycle (create / delete / activate)
//!   pip/          — Package management (install / uninstall / list / upgrade)
//!   execution/    — Code and script execution
//!   services/     — PythonExecutionService (public API for other plugins)
//!   plugin/       — PluginApi registration handle
//!   tests/        — Integration test suite
//! ```
//!
//! Python is **not** shipped inside the app installer. Each machine uses a
//! system interpreter (with CR-managed venvs) or downloads a compatible
//! standalone build into `{data_dir}/python/` on first run.

pub mod download;
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
    EnvironmentInfo, ExecutionContext, ExecutionResult, PackageInfo, PythonError, PythonResult,
    PythonRuntimeHealth, RuntimeStatus,
};
