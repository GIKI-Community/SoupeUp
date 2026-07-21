pub mod abstraction;
pub mod selection;

#[cfg(test)]
mod tests;

pub use abstraction::{SchedulerError, SchedulerService};
pub use selection::{
    SchedulerRegistry, DASK_PLUGIN_ID, DEFAULT_SCHEDULER, MPI_PLUGIN_ID, RAY_PLUGIN_ID,
};
