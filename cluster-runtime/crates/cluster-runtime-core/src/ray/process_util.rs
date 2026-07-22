use crate::python_runtime::BackgroundProcessInfo;
use crate::python_runtime::ProcessStatus;

pub fn background_process_error(proc: &BackgroundProcessInfo) -> Option<String> {
    for line in proc.stdout_tail.lines().rev() {
        if let Some(json) = line.strip_prefix("RAY_WORKER_ERROR ") {
            return Some(json.to_string());
        }
        if let Some(json) = line.strip_prefix("RAY_HEAD_ERROR ") {
            return Some(json.to_string());
        }
    }

    let err_lines: Vec<&str> = proc
        .stderr_tail
        .lines()
        .filter(|l| {
            let l = l.trim();
            l.contains("Traceback")
                || l.contains("Error:")
                || l.contains("ERROR:")
                || l.starts_with("ERROR")
        })
        .collect();
    if !err_lines.is_empty() {
        return Some(err_lines.join("\n"));
    }

    if proc.status == ProcessStatus::Failed {
        return Some(format!(
            "Process exited with code {}",
            proc.exit_code.unwrap_or(-1)
        ));
    }

    None
}

pub fn is_ready(proc: &BackgroundProcessInfo, marker: &str) -> bool {
    proc.stdout_tail.lines().any(|l| l.starts_with(marker))
}

pub fn format_process_logs(proc: &BackgroundProcessInfo) -> String {
    let mut parts = Vec::new();
    if !proc.stdout_tail.trim().is_empty() {
        parts.push(proc.stdout_tail.trim().to_string());
    }
    if !proc.stderr_tail.trim().is_empty() {
        parts.push(format!("--- stderr ---\n{}", proc.stderr_tail.trim()));
    }
    parts.join("\n\n")
}
