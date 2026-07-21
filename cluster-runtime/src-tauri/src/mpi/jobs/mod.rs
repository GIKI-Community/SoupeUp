//! MPI job helpers (entry-point mapping lives in `MpiService`).

use crate::jobs::models::EntryPoint;

pub fn is_python_mpi_entry(entry: &EntryPoint) -> bool {
    matches!(
        entry,
        EntryPoint::PythonScript { .. } | EntryPoint::PythonFunction { .. }
    )
}
