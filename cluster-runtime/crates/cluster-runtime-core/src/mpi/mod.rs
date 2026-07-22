//! # MPI Scheduler Plugin
//!
//! Native MPI job launch via `mpirun` / `mpiexec`. Independent of the Python
//! runtime for `MpiExecutable` jobs; optionally wraps Python with mpi4py.

pub mod adapter;
pub mod jobs;
pub mod launcher;
pub mod services;
pub mod settings;
pub mod types;

pub use services::MpiService;
pub use types::*;
