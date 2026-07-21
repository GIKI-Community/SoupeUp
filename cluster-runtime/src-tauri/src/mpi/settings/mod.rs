use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MpiSettings {
    /// Preferred launcher name when several are on PATH.
    pub preferred_launcher: Option<String>,
    /// Default rank count when job does not specify ranks / cpu_cores.
    pub default_ranks: u32,
    /// Extra args prepended to every mpirun/mpiexec invocation.
    #[serde(default)]
    pub extra_launcher_args: Vec<String>,
}

impl Default for MpiSettings {
    fn default() -> Self {
        Self {
            preferred_launcher: None,
            default_ranks: 2,
            extra_launcher_args: Vec::new(),
        }
    }
}
