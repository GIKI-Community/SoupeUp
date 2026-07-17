#[cfg(test)]
mod tests {
    use crate::jobs::models::{JobSpec, ResourceRequirements, SchedulerCapabilities};
    use crate::scheduler::selection::validate_job;

    #[test]
    fn validate_job_warns_on_gpu_without_support() {
        let spec = JobSpec {
            name: "gpu-job".into(),
            description: None,
            entry_point: crate::jobs::models::EntryPoint::Example {
                example_id: "mandelbrot".into(),
                args: None,
            },
            args: serde_json::Value::Null,
            env: Default::default(),
            resources: ResourceRequirements {
                gpu_count: Some(1),
                ..Default::default()
            },
            priority: 0,
            timeout_secs: None,
            retry_policy: None,
            tags: vec![],
            metadata: Default::default(),
            execution_context: Default::default(),
        };
        let caps = SchedulerCapabilities {
            supports_gpu: false,
            supports_python: true,
            ..Default::default()
        };
        let warnings = validate_job(&spec, &caps);
        assert!(!warnings.is_empty());
    }

    #[test]
    fn example_catalog_has_six_entries() {
        assert_eq!(crate::jobs::examples::list().len(), 6);
    }
}
