#[cfg(test)]
mod selection_tests {
    use std::path::PathBuf;

    use crate::jobs::models::ResourceRequirements;
    use crate::scheduler::selection::{validate_resources, SchedulerRegistry, SchedulerCapabilities};

    #[tokio::test]
    async fn registry_default_scheduler_is_dask() {
        let registry = SchedulerRegistry::new(PathBuf::from("/tmp/test-active.json"));
        assert_eq!(
            registry.active_id().await,
            crate::scheduler::selection::DEFAULT_SCHEDULER
        );
    }

    #[test]
    fn resource_validation_gpu_warning() {
        let resources = ResourceRequirements {
            gpu_count: Some(2),
            ..Default::default()
        };
        let caps = SchedulerCapabilities {
            supports_gpu: false,
            ..Default::default()
        };
        let warnings = validate_resources(&resources, &caps);
        assert!(warnings.iter().any(|w| w.contains("GPU")));
    }
}
