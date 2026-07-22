use std::os::raw::c_char;

/// C-ABI compatible struct for plugin metadata
#[repr(C)]
pub struct PluginMetadata {
    pub id: *const c_char,
    pub name: *const c_char,
    pub version: *const c_char,
    pub description: *const c_char,
    pub author: *const c_char,
}

/// The main C-ABI compatible Plugin trait
pub trait PluginApi: Send + Sync {
    fn metadata(&self) -> PluginMetadata;
    fn initialize(&mut self) -> bool;
    fn shutdown(&mut self);
    fn health(&self) -> bool;
}

// Function signature for the plugin entry point
#[allow(improper_ctypes_definitions)]
pub type PluginCreateFn = unsafe extern "C" fn() -> *mut dyn PluginApi;
