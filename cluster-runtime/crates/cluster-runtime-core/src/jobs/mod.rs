pub mod api;
pub mod dependencies;
pub mod examples;
pub mod history;
pub mod manager;
pub mod models;
pub mod progress;
pub mod queue;
pub mod resources;
pub mod results;

#[cfg(test)]
mod tests;

pub use api::JobApi;
pub use history::JobHistoryStore;
pub use manager::JobManager;
pub use models::*;

// Legacy type alias for UI backward compatibility during migration
pub type JobHistory = JobHistoryStore;
