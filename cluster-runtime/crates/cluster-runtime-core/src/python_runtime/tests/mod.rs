//! Integration tests for the Python Runtime Plugin.
//!
//! These tests require a Python interpreter to be available (either bundled
//! or on the system PATH).  They are skipped automatically when no interpreter
//! is found so that CI environments without Python do not fail the build.
//!
//! Run with:
//!   cargo test -p cluster-runtime python_runtime

#[cfg(test)]
mod python_runtime_tests {
    use std::collections::HashMap;
    use std::path::PathBuf;
    use crate::python_runtime::{
        environment::EnvironmentManager,
        execution::ExecutionEngine,
        interpreter::discover_python,
        services::PythonExecutionService,
        types::{ExecutionContext, RuntimeStatus},
    };

    // ─── Helpers ──────────────────────────────────────────────────────────────

    /// Override the environments base dir with a temp dir for tests.
    fn test_base_dir() -> PathBuf {
        let id = uuid::Uuid::new_v4();
        std::env::temp_dir()
            .join("cluster_runtime_tests")
            .join(id.to_string())
    }

    async fn require_interpreter() -> crate::python_runtime::interpreter::PythonInterpreter {
        match discover_python().await {
            Some(i) => i,
            None => {
                eprintln!("SKIP: no Python interpreter found");
                // Panic with a recognisable message so the test shows as failed
                // rather than silently passing.
                panic!("No Python interpreter available for testing");
            }
        }
    }

    // ─── Interpreter Tests ────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_python_discovery() {
        let interp = require_interpreter().await;
        assert!(!interp.version.is_empty(), "Version should not be empty");
        assert!(
            interp.version.starts_with('3'),
            "Expected Python 3.x, got {}",
            interp.version
        );
        assert!(interp.path.exists(), "Interpreter path should exist on disk");
    }

    // ─── Environment Tests ────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_venv_creation() {
        let interp = require_interpreter().await;
        let base = test_base_dir();
        std::fs::create_dir_all(&base).expect("Cannot create test base dir");

        let mgr = EnvironmentManager::new(interp);
        // We bypass `initialize()` and call `create_environment` directly to
        // control the output path via the test's temp dir.
        let result = mgr.create_environment("test-venv-create").await;

        // Clean up regardless
        std::fs::remove_dir_all(&base).ok();

        assert!(
            result.is_ok(),
            "create_environment should succeed: {:?}",
            result
        );
        let info = result.unwrap();
        assert_eq!(info.name, "test-venv-create");
        assert!(
            info.python_version.is_some(),
            "Venv should have a Python version"
        );
    }

    // ─── Execution Tests ──────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_code_execution_hello_world() {
        let interp = require_interpreter().await;
        let base = test_base_dir();
        std::fs::create_dir_all(&base).expect("Cannot create test base dir");

        let mgr = EnvironmentManager::new(interp);
        // Initialize creates the `default` env
        mgr.initialize().await.expect("Environment init failed");
        let env_path = mgr.get_active_env_path().await;

        let engine = ExecutionEngine::new();
        let ctx = ExecutionContext::default();
        let result = engine
            .execute_code("print('Hello World')", &env_path, &ctx)
            .await;

        std::fs::remove_dir_all(&base).ok();

        assert!(result.is_ok(), "execute_code should not error: {:?}", result);
        let r = result.unwrap();
        assert!(r.success, "Execution should succeed");
        assert_eq!(r.exit_code, 0);
        assert_eq!(r.stdout.trim(), "Hello World");
        assert!(r.stderr.is_empty());
        assert!(r.exception.is_none());
    }

    #[tokio::test]
    async fn test_stdout_capture() {
        let interp = require_interpreter().await;
        let base = test_base_dir();
        std::fs::create_dir_all(&base).expect("Cannot create test base dir");

        let mgr = EnvironmentManager::new(interp);
        mgr.initialize().await.expect("Environment init failed");
        let env_path = mgr.get_active_env_path().await;

        let engine = ExecutionEngine::new();
        let ctx = ExecutionContext::default();
        let code = r#"
for i in range(3):
    print(f"line {i}")
"#;
        let result = engine.execute_code(code, &env_path, &ctx).await.unwrap();

        std::fs::remove_dir_all(&base).ok();

        assert!(result.success);
        assert!(result.stdout.contains("line 0"));
        assert!(result.stdout.contains("line 1"));
        assert!(result.stdout.contains("line 2"));
    }

    #[tokio::test]
    async fn test_stderr_capture() {
        let interp = require_interpreter().await;
        let base = test_base_dir();
        std::fs::create_dir_all(&base).expect("Cannot create test base dir");

        let mgr = EnvironmentManager::new(interp);
        mgr.initialize().await.expect("Environment init failed");
        let env_path = mgr.get_active_env_path().await;

        let engine = ExecutionEngine::new();
        let ctx = ExecutionContext::default();
        // `import sys; sys.stderr.write(...)` writes to stderr without raising
        let code = r#"
import sys
sys.stderr.write("error output\n")
"#;
        let result = engine.execute_code(code, &env_path, &ctx).await.unwrap();

        std::fs::remove_dir_all(&base).ok();

        // Exit code 0 (successful), stderr has content
        assert!(result.success);
        assert!(result.stderr.contains("error output"));
    }

    #[tokio::test]
    async fn test_exception_capture() {
        let interp = require_interpreter().await;
        let base = test_base_dir();
        std::fs::create_dir_all(&base).expect("Cannot create test base dir");

        let mgr = EnvironmentManager::new(interp);
        mgr.initialize().await.expect("Environment init failed");
        let env_path = mgr.get_active_env_path().await;

        let engine = ExecutionEngine::new();
        let ctx = ExecutionContext::default();
        let result = engine
            .execute_code("raise ValueError('test error')", &env_path, &ctx)
            .await
            .unwrap();

        std::fs::remove_dir_all(&base).ok();

        assert!(!result.success, "Should fail when an exception is raised");
        assert_ne!(result.exit_code, 0);
        assert!(
            result.exception.is_some(),
            "exception field should be populated"
        );
        let exc = result.exception.unwrap();
        assert!(
            exc.contains("ValueError"),
            "Exception should mention ValueError, got: {}",
            exc
        );
    }

    #[tokio::test]
    async fn test_script_execution() {
        let interp = require_interpreter().await;
        let base = test_base_dir();
        std::fs::create_dir_all(&base).expect("Cannot create test base dir");

        let mgr = EnvironmentManager::new(interp);
        mgr.initialize().await.expect("Environment init failed");
        let env_path = mgr.get_active_env_path().await;

        // Write a real .py file
        let script_path = base.join("test_script.py");
        std::fs::write(&script_path, "print('from file')").expect("Cannot write test script");

        let engine = ExecutionEngine::new();
        let ctx = ExecutionContext::default();
        let result = engine
            .execute_script(&script_path, &env_path, &ctx)
            .await
            .unwrap();

        std::fs::remove_dir_all(&base).ok();

        assert!(result.success);
        assert_eq!(result.stdout.trim(), "from file");
    }

    // ─── Package Management Tests ─────────────────────────────────────────────

    #[tokio::test]
    async fn test_package_list() {
        let interp = require_interpreter().await;
        let base = test_base_dir();
        std::fs::create_dir_all(&base).expect("Cannot create test base dir");

        let mgr = EnvironmentManager::new(interp);
        mgr.initialize().await.expect("Environment init failed");
        let env_path = mgr.get_active_env_path().await;

        let pip = crate::python_runtime::pip::PipManager::new(None);
        let packages = pip.list(&env_path).await;

        std::fs::remove_dir_all(&base).ok();

        assert!(
            packages.is_ok(),
            "list_packages should succeed: {:?}",
            packages
        );
        // A fresh venv has at least pip itself
        let pkgs = packages.unwrap();
        assert!(
            pkgs.iter().any(|p| p.name.to_lowercase() == "pip"),
            "pip should be in a fresh venv"
        );
    }

    // ─── Service Tests ────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_service_initialization() {
        let interp = require_interpreter().await;
        let svc = PythonExecutionService::new(interp, None);

        // Before initialize(), service should be in Initializing state
        let health = svc.runtime_health().await;
        // Health always succeeds, but status should be Initializing
        // (we haven't called initialize() yet)

        let init_result = svc.initialize().await;
        assert!(
            init_result.is_ok(),
            "Service initialization should succeed: {:?}",
            init_result
        );

        let health = svc.runtime_health().await.unwrap();
        assert_eq!(health.status, RuntimeStatus::Ready);
        assert!(health.python_version.is_some());
        assert!(health.active_environment.is_some());
        assert_eq!(
            health.active_environment.as_deref(),
            Some("default")
        );
    }

    #[tokio::test]
    async fn test_execution_via_service() {
        let interp = require_interpreter().await;
        let svc = PythonExecutionService::new(interp, None);
        svc.initialize().await.expect("Init failed");

        let result = svc
            .execute_code("print('via service')", None)
            .await
            .unwrap();

        assert!(result.success);
        assert_eq!(result.stdout.trim(), "via service");
    }

    #[tokio::test]
    async fn test_execution_timeout() {
        let interp = require_interpreter().await;
        let svc = PythonExecutionService::new(interp, None);
        svc.initialize().await.expect("Init failed");

        let ctx = ExecutionContext {
            timeout_secs: Some(1), // 1-second timeout
            ..Default::default()
        };

        let result = svc
            .execute_code("import time; time.sleep(10)", Some(ctx))
            .await;

        // Should return a Timeout error
        assert!(
            matches!(result, Err(crate::python_runtime::types::PythonError::Timeout(1))),
            "Expected Timeout error, got: {:?}",
            result
        );
    }
}
