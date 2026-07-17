//! # Dask Scheduler Plugin
//!
//! Adapter that integrates Dask into Cluster Runtime without owning Python,
//! package management, or networking beyond Dask configuration.
//!
//! All Python interaction goes through `PythonExecutionService`.
//! Scheduler / Worker / Client are controlled via Dask's official Python API.

pub mod client;
pub mod dashboard;
pub mod examples;
pub mod jobs;
pub mod monitoring;
pub mod plugin;
pub mod process_util;
pub mod scheduler;
pub mod scripts;
pub mod services;
pub mod settings;
pub mod types;
pub mod worker;

pub use services::DaskService;
pub use services::example_failure;
pub use types::*;
