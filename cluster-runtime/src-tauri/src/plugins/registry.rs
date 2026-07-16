use std::collections::HashMap;

use super::Plugin;

/// In-memory registry of loaded plugin instances.
pub struct PluginRegistry {
    plugins: HashMap<String, Box<dyn Plugin>>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
        }
    }

    pub fn register(&mut self, plugin: Box<dyn Plugin>) {
        let id = plugin.id();
        self.plugins.insert(id, plugin);
    }

    pub fn get(&self, id: &str) -> Option<&dyn Plugin> {
        self.plugins.get(id).map(|p| p.as_ref())
    }

    pub fn get_mut(&mut self, id: &str) -> Option<&mut Box<dyn Plugin>> {
        self.plugins.get_mut(id)
    }

    pub fn remove(&mut self, id: &str) -> Option<Box<dyn Plugin>> {
        self.plugins.remove(id)
    }

    pub fn list_ids(&self) -> Vec<String> {
        self.plugins.keys().cloned().collect()
    }

    pub fn count(&self) -> usize {
        self.plugins.len()
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}
