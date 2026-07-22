use crate::plugin_api::{PluginApi, PluginMetadata};

pub struct RayPlugin {
    initialized: bool,
}

impl RayPlugin {
    pub fn new() -> Self {
        Self { initialized: false }
    }
}

impl Default for RayPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginApi for RayPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            id: b"plugin-ray\0".as_ptr() as *const _,
            name: b"Ray\0".as_ptr() as *const _,
            version: b"0.1.0\0".as_ptr() as *const _,
            description: b"Distributed computing via Ray.io, powered by the Python Runtime Plugin.\0"
                .as_ptr() as *const _,
            author: b"Cluster Runtime Team\0".as_ptr() as *const _,
        }
    }

    fn initialize(&mut self) -> bool {
        self.initialized = true;
        true
    }

    fn shutdown(&mut self) {
        log::info!("Ray Plugin: shutdown requested.");
        self.initialized = false;
    }

    fn health(&self) -> bool {
        self.initialized
    }
}
