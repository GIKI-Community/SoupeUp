use super::Plugin;

/// Mock plugin implementation for development and testing.
pub struct MockExecutionPlugin {
    id: String,
    name: String,
    version: String,
    running: bool,
}

impl MockExecutionPlugin {
    pub fn new(id: &str, name: &str, version: &str) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            version: version.to_string(),
            running: false,
        }
    }
}

impl Plugin for MockExecutionPlugin {
    fn id(&self) -> String {
        self.id.clone()
    }

    fn name(&self) -> String {
        self.name.clone()
    }

    fn version(&self) -> String {
        self.version.clone()
    }

    fn initialize(&mut self) {
        self.running = true;
    }

    fn shutdown(&mut self) {
        self.running = false;
    }
}
