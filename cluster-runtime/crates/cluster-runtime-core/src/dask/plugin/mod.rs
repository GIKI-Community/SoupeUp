use crate::plugin_api::{PluginApi, PluginMetadata};

/// Lightweight registration handle for the Dask Scheduler Plugin.
/// Heavy initialization happens asynchronously in the Tauri setup hook.
pub struct DaskSchedulerPlugin {
    initialized: bool,
}

impl DaskSchedulerPlugin {
    pub fn new() -> Self {
        Self { initialized: false }
    }
}

impl Default for DaskSchedulerPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginApi for DaskSchedulerPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            id: b"plugin-dask-scheduler\0".as_ptr() as *const _,
            name: b"Dask Scheduler\0".as_ptr() as *const _,
            version: b"0.1.0\0".as_ptr() as *const _,
            description: b"Distributed scheduling via Dask, powered by the Python Runtime Plugin.\0"
                .as_ptr() as *const _,
            author: b"Cluster Runtime Team\0".as_ptr() as *const _,
        }
    }

    fn initialize(&mut self) -> bool {
        self.initialized = true;
        true
    }

    fn shutdown(&mut self) {
        log::info!("Dask Scheduler Plugin: shutdown requested.");
        self.initialized = false;
    }

    fn health(&self) -> bool {
        self.initialized
    }
}
