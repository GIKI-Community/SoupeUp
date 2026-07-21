use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MpiToolchain {
    /// Absolute or PATH-resolved launcher binary (`mpirun` or `mpiexec`).
    pub launcher: String,
    /// Detected flavour for logging / flags.
    pub flavour: MpiFlavour,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum MpiFlavour {
    OpenMpi,
    Mpich,
    MsMpi,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MpiLaunchResult {
    pub success: bool,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub execution_time_ms: u64,
    pub ranks: u32,
}

#[derive(Debug, thiserror::Error)]
pub enum MpiError {
    #[error("MPI toolchain not found: {0}")]
    ToolchainNotFound(String),
    #[error("MPI not initialized: {0}")]
    NotReady(String),
    #[error("Job error: {0}")]
    Job(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type MpiResult<T> = Result<T, MpiError>;
