/// Core plugin trait. All execution engines and extensions implement this interface.
pub trait Plugin: Send + Sync {
    fn id(&self) -> String;
    fn name(&self) -> String;
    fn version(&self) -> String;
    fn initialize(&mut self);
    fn shutdown(&mut self);
}
