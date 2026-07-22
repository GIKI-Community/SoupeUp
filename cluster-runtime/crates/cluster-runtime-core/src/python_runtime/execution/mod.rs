use std::path::Path;
use crate::python_runtime::types::{ExecutionContext, ExecutionResult, PythonError, PythonResult};
use crate::python_runtime::utils::{run_command_captured, temp_script_path, venv_python_path};

/// Responsible for executing Python code and scripts inside managed environments.
///
/// The engine is intentionally stateless — it receives both the code/path to
/// run and the environment path on every call.  All state (active env, packages,
/// etc.) lives in the service layer above.
pub struct ExecutionEngine;

impl ExecutionEngine {
    pub fn new() -> Self {
        Self
    }

    // ─── Execution Entry Points ───────────────────────────────────────────────

    /// Execute an arbitrary Python code string.
    ///
    /// The code is written to a temporary `.py` file, executed inside the
    /// provided venv, and the file is deleted regardless of success/failure.
    pub async fn execute_code(
        &self,
        code: &str,
        env_path: &Path,
        context: &ExecutionContext,
    ) -> PythonResult<ExecutionResult> {
        let script_path = temp_script_path();

        tokio::fs::write(&script_path, code).await.map_err(|e| {
            PythonError::ExecutionError(format!("Cannot write temp script: {}", e))
        })?;

        let result = self.execute_script(&script_path, env_path, context).await;

        // Best-effort cleanup — a failure here doesn't affect the result
        tokio::fs::remove_file(&script_path).await.ok();

        result
    }

    /// Execute a `.py` file directly.
    pub async fn execute_script(
        &self,
        script_path: &Path,
        env_path: &Path,
        context: &ExecutionContext,
    ) -> PythonResult<ExecutionResult> {
        if !script_path.exists() {
            return Err(PythonError::ExecutionError(format!(
                "Script not found: {}",
                script_path.display()
            )));
        }

        let python = venv_python_path(env_path);
        let script_str = script_path.to_str().ok_or_else(|| {
            PythonError::ExecutionError("Script path contains non-UTF8 characters".to_string())
        })?;

        let mut all_args = vec![script_str.to_string()];
        all_args.extend(context.args.clone());
        let args_ref: Vec<&str> = all_args.iter().map(|s| s.as_str()).collect();

        let cwd = context.working_directory.as_deref();

        log::info!(
            "Executing script: {} (env: {})",
            script_path.display(),
            env_path.display()
        );

        let mut result = run_command_captured(
            &python,
            &args_ref,
            cwd,
            &context.env_vars,
            context.timeout_secs,
        )
        .await?;

        // Populate exception from stderr when execution failed
        if !result.success && !result.stderr.is_empty() {
            result.exception = Some(result.stderr.clone());
        }

        log::info!(
            "Script finished in {}ms (exit_code: {})",
            result.execution_time_ms,
            result.exit_code
        );

        Ok(result)
    }

    /// Run `python -m <module>` inside the managed environment.
    pub async fn execute_module(
        &self,
        module: &str,
        env_path: &Path,
        context: &ExecutionContext,
    ) -> PythonResult<ExecutionResult> {
        if module.is_empty() {
            return Err(PythonError::ExecutionError(
                "Module name must not be empty".to_string(),
            ));
        }

        let python = venv_python_path(env_path);

        let mut all_args = vec!["-m".to_string(), module.to_string()];
        all_args.extend(context.args.clone());
        let args_ref: Vec<&str> = all_args.iter().map(|s| s.as_str()).collect();

        let cwd = context.working_directory.as_deref();

        log::info!(
            "Executing module `-m {}` (env: {})",
            module,
            env_path.display()
        );

        let mut result = run_command_captured(
            &python,
            &args_ref,
            cwd,
            &context.env_vars,
            context.timeout_secs,
        )
        .await?;

        if !result.success && !result.stderr.is_empty() {
            result.exception = Some(result.stderr.clone());
        }

        log::info!(
            "Module `-m {}` finished in {}ms (exit_code: {})",
            module,
            result.execution_time_ms,
            result.exit_code
        );

        Ok(result)
    }

    /// Execute a Python package directory by running its `__main__.py`.
    pub async fn execute_directory(
        &self,
        dir_path: &Path,
        env_path: &Path,
        context: &ExecutionContext,
    ) -> PythonResult<ExecutionResult> {
        if !dir_path.is_dir() {
            return Err(PythonError::ExecutionError(format!(
                "Not a directory: {}",
                dir_path.display()
            )));
        }

        let main_py = dir_path.join("__main__.py");
        if !main_py.exists() {
            return Err(PythonError::ExecutionError(format!(
                "No __main__.py found in {}",
                dir_path.display()
            )));
        }

        let python = venv_python_path(env_path);
        let dir_str = dir_path.to_str().ok_or_else(|| {
            PythonError::ExecutionError(
                "Directory path contains non-UTF8 characters".to_string(),
            )
        })?;

        let mut all_args = vec![dir_str.to_string()];
        all_args.extend(context.args.clone());
        let args_ref: Vec<&str> = all_args.iter().map(|s| s.as_str()).collect();

        let cwd = context.working_directory.as_deref();

        log::info!(
            "Executing package directory {} (env: {})",
            dir_path.display(),
            env_path.display()
        );

        let mut result = run_command_captured(
            &python,
            &args_ref,
            cwd,
            &context.env_vars,
            context.timeout_secs,
        )
        .await?;

        if !result.success && !result.stderr.is_empty() {
            result.exception = Some(result.stderr.clone());
        }

        Ok(result)
    }

    /// Alias for `execute_script` — satisfies the `execute_file()` API surface.
    pub async fn execute_file(
        &self,
        file_path: &Path,
        env_path: &Path,
        context: &ExecutionContext,
    ) -> PythonResult<ExecutionResult> {
        self.execute_script(file_path, env_path, context).await
    }
}

impl Default for ExecutionEngine {
    fn default() -> Self {
        Self::new()
    }
}
