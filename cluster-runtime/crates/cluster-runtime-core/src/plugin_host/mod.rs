//! Host-side plugin context and in-process factories.

pub mod factories;
pub mod update_check;

pub struct PluginContext {
    // Logger and Config injected into plugins
}

impl PluginContext {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for PluginContext {
    fn default() -> Self {
        Self::new()
    }
}
