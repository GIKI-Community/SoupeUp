use crate::plugin_api::{PluginApi, PluginMetadata};

/// Implements the `PluginApi` trait so the Python Runtime Plugin can be
/// registered in the `PluginRegistry` alongside dynamically-loaded plugins.
///
/// The actual heavy initialization is done asynchronously in `lib.rs` via the
/// Tauri setup hook.  This struct is a lightweight registration handle that
/// lets the runtime appear in the plugin list with the correct metadata.
pub struct PythonRuntimePlugin {
    initialized: bool,
}

impl PythonRuntimePlugin {
    pub fn new() -> Self {
        Self { initialized: false }
    }
}

impl Default for PythonRuntimePlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginApi for PythonRuntimePlugin {
    fn metadata(&self) -> PluginMetadata {
        // These byte strings are static and null-terminated.
        PluginMetadata {
            id: b"plugin-python-runtime\0".as_ptr() as *const _,
            name: b"Python Runtime\0".as_ptr() as *const _,
            version: b"0.1.0\0".as_ptr() as *const _,
            description: b"Embedded Python 3.13 runtime with virtual environment and package management.\0"
                .as_ptr() as *const _,
            author: b"Cluster Runtime Team\0".as_ptr() as *const _,
        }
    }

    /// The async PythonExecutionService is initialized in the Tauri setup hook,
    /// not here.  This method simply marks the plugin object as registered.
    fn initialize(&mut self) -> bool {
        self.initialized = true;
        true
    }

    fn shutdown(&mut self) {
        log::info!("Python Runtime Plugin: shutdown requested.");
        self.initialized = false;
    }

    fn health(&self) -> bool {
        self.initialized
    }
}
