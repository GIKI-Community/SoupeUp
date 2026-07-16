use std::os::raw::c_char;

// C-ABI compatible struct for plugin metadata
#[repr(C)]
pub struct PluginMetadata {
    pub id: *const c_char,
    pub name: *const c_char,
    pub version: *const c_char,
    pub description: *const c_char,
    pub author: *const c_char,
}

// The main C-ABI compatible Plugin trait
pub trait PluginApi: Send + Sync {
    fn metadata(&self) -> PluginMetadata;
    fn initialize(&mut self) -> bool;
    fn shutdown(&mut self);
    fn health(&self) -> bool;
}

pub struct ExamplePlugin {}

impl PluginApi for ExamplePlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            id: b"example-plugin\0".as_ptr() as *const c_char,
            name: b"Example Plugin\0".as_ptr() as *const c_char,
            version: b"1.0.0\0".as_ptr() as *const c_char,
            description: b"A dynamically loaded example plugin\0".as_ptr() as *const c_char,
            author: b"Cluster Runtime\0".as_ptr() as *const c_char,
        }
    }

    fn initialize(&mut self) -> bool {
        println!("[ExamplePlugin] Initialized.");
        true
    }

    fn shutdown(&mut self) {
        println!("[ExamplePlugin] Shutting down.");
    }

    fn health(&self) -> bool {
        true
    }
}

#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn _plugin_create() -> *mut dyn PluginApi {
    let plugin = Box::new(ExamplePlugin {});
    Box::into_raw(plugin)
}
