//! # Ray Plugin
//!
//! Adapter that integrates Ray.io into Cluster Runtime without owning Python,
//! package management, or networking beyond Ray configuration.

pub mod adapter;
pub mod client;
pub mod dashboard;
pub mod examples;
pub mod head;
pub mod jobs;
pub mod monitoring;
pub mod plugin;
pub mod process_util;
pub mod scripts;
pub mod services;
pub mod settings;
pub mod types;
pub mod worker;

pub use services::RayService;
pub use services::example_failure;
pub use types::*;
