use parking_lot::Mutex;

use super::{PluginLoader, PluginRegistry};

/// Orchestrates plugin lifecycle: loading, initialization, and shutdown.
pub struct PluginManager {
    registry: Mutex<PluginRegistry>,
    initialized: Mutex<bool>,
}

impl PluginManager {
    pub fn new() -> Self {
        Self {
            registry: Mutex::new(PluginRegistry::new()),
            initialized: Mutex::new(false),
        }
    }

    pub fn initialize(&self) {
        let mut initialized = self.initialized.lock();
        if *initialized {
            return;
        }

        let mut registry = self.registry.lock();
        PluginLoader::load_builtins(&mut registry);

        for id in registry.list_ids() {
            if let Some(plugin) = registry.get_mut(&id) {
                plugin.initialize();
            }
        }

        *initialized = true;
    }

    pub fn shutdown(&self) {
        let mut registry = self.registry.lock();
        for id in registry.list_ids() {
            if let Some(plugin) = registry.get_mut(&id) {
                plugin.shutdown();
            }
        }
        *self.initialized.lock() = false;
    }

    pub fn plugin_count(&self) -> usize {
        self.registry.lock().count()
    }
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}
