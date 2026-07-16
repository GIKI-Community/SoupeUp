pub mod manifest;

use libloading::{Library, Symbol};
use std::path::Path;
use std::sync::Arc;
use crate::plugin_api::{PluginApi, PluginCreateFn};
use self::manifest::PluginManifest;

pub struct PluginLoader {
    // Keep loaded libraries in memory so they don't get unloaded while in use
    loaded_libs: Vec<Arc<Library>>,
}

impl PluginLoader {
    pub fn new() -> Self {
        Self {
            loaded_libs: Vec::new(),
        }
    }

    pub fn load_manifest<P: AsRef<Path>>(path: P) -> Result<PluginManifest, String> {
        let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        toml::from_str(&content).map_err(|e| e.to_string())
    }

    /// Dynamically load a plugin DLL
    pub unsafe fn load_plugin<P: AsRef<Path>>(&mut self, path: P) -> Result<Box<dyn PluginApi>, String> {
        let lib = Library::new(path.as_ref()).map_err(|e| e.to_string())?;
        
        // Find the plugin creation entry point
        let create_fn: Symbol<PluginCreateFn> = lib.get(b"_plugin_create\0").map_err(|e| e.to_string())?;
        
        // Create the plugin instance
        let plugin_ptr = create_fn();
        let plugin = Box::from_raw(plugin_ptr);
        
        self.loaded_libs.push(Arc::new(lib));
        
        Ok(plugin)
    }
}
