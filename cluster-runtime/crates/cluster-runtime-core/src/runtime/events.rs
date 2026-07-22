use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuntimeEvent {
    NodeRegistered(String),
    WorkerStarted(String),
    WorkerStopped(String),
    JobQueued(String),
    JobStarted(String),
    JobCompleted(String),
    TaskQueued(String),
    TaskStarted(String),
    TaskCompleted(String),
    TaskFailed(String),
    TaskCancelled(String),
    ResourcesUpdated,
    PluginLoaded(String),
    PluginUnloaded(String),
}
