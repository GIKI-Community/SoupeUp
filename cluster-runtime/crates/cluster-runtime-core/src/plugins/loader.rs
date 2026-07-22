use super::{MockExecutionPlugin, Plugin, PluginRegistry};

/// Loads plugins into the registry. Currently uses mock implementations only.
/// Future: dynamic library loading from configured plugin directories.
pub struct PluginLoader;

impl PluginLoader {
    pub fn load_builtins(registry: &mut PluginRegistry) {
        let builtins: Vec<Box<dyn Plugin>> = vec![
            Box::new(MockExecutionPlugin::new(
                "plugin-native",
                "Native Runtime",
                "0.1.0",
            )),
            Box::new(MockExecutionPlugin::new(
                "plugin-ray",
                "Ray Adapter",
                "0.1.0",
            )),
            Box::new(MockExecutionPlugin::new(
                "plugin-htcondor",
                "HTCondor Adapter",
                "0.0.9",
            )),
        ];

        for plugin in builtins {
            registry.register(plugin);
        }
    }

    #[allow(dead_code)]
    pub fn load_from_path(_path: &str, _registry: &mut PluginRegistry) -> Result<(), String> {
        // Future: scan directory, validate manifests, dlopen plugin libraries
        Err("Dynamic plugin loading is not yet implemented".into())
    }
}
