//! Detect imports in submitted job source and install missing packages
//! into the active Python venv (shared by local Ray/Dask workers).

use std::collections::{BTreeSet, HashMap, HashSet};

use crate::jobs::models::{DependencyReport, EntryPoint, JobSpec};
use crate::python_runtime::PythonExecutionService;

/// Map common import names to their PyPI package names.
fn import_to_pip_name(import_name: &str) -> &str {
    match import_name {
        "cv2" => "opencv-python",
        "PIL" => "Pillow",
        "sklearn" => "scikit-learn",
        "bs4" => "beautifulsoup4",
        "yaml" => "PyYAML",
        "Crypto" => "pycryptodome",
        "skimage" => "scikit-image",
        "dateutil" => "python-dateutil",
        "dotenv" => "python-dotenv",
        "OpenSSL" => "pyOpenSSL",
        "win32api" | "win32com" | "win32gui" | "pythoncom" => "pywin32",
        other => other,
    }
}

/// Extract top-level module names from Python source.
///
/// Handles `import a`, `import a.b as c`, `import a, b`, and `from a.b import c`.
/// Skips relative imports (`from .`) and `__future__`.
pub fn parse_imports(source: &str) -> Vec<String> {
    let mut found = BTreeSet::new();

    for raw_line in source.lines() {
        let line = strip_comment(raw_line).trim();
        if line.is_empty() {
            continue;
        }

        if let Some(rest) = line.strip_prefix("from ") {
            let rest = rest.trim();
            if rest.starts_with('.') {
                continue; // relative import
            }
            let module = rest
                .split_whitespace()
                .next()
                .unwrap_or("")
                .trim_matches(|c: char| !is_ident_char(c) && c != '.');
            if let Some(top) = top_level(module) {
                if top != "__future__" {
                    found.insert(top.to_string());
                }
            }
            continue;
        }

        if let Some(rest) = line.strip_prefix("import ") {
            for part in rest.split(',') {
                let name = part
                    .trim()
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .trim_matches(|c: char| !is_ident_char(c) && c != '.');
                if let Some(top) = top_level(name) {
                    if top != "__future__" {
                        found.insert(top.to_string());
                    }
                }
            }
        }
    }

    found.into_iter().collect()
}

fn strip_comment(line: &str) -> &str {
    // Simple: strip `#` outside of strings is good enough for import lines.
    if let Some(idx) = line.find('#') {
        // Avoid stripping `#` inside quotes on the same line when possible.
        let before = &line[..idx];
        let single = before.matches('\'').count();
        let double = before.matches('"').count();
        if single % 2 == 0 && double % 2 == 0 {
            return before;
        }
    }
    line
}

fn is_ident_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

fn top_level(dotted: &str) -> Option<&str> {
    let name = dotted.split('.').next()?.trim();
    if name.is_empty() || !name.chars().all(is_ident_char) {
        None
    } else {
        Some(name)
    }
}

fn source_from_spec(spec: &JobSpec) -> Option<&str> {
    match &spec.entry_point {
        EntryPoint::PythonFunction { body } => Some(body.as_str()),
        EntryPoint::PythonScript { script } => Some(script.as_str()),
        EntryPoint::PythonModule { .. }
        | EntryPoint::Example { .. }
        | EntryPoint::MpiExecutable { .. } => None,
    }
}

#[derive(Debug, serde::Deserialize)]
struct ProbeResult {
    #[serde(default)]
    missing: Vec<String>,
    #[serde(default)]
    present: Vec<String>,
    #[serde(default)]
    skipped: Vec<String>,
}

/// Probe the active venv: classify candidate import names as stdlib / present / missing.
async fn probe_modules(
    python: &PythonExecutionService,
    candidates: &[String],
) -> Result<ProbeResult, String> {
    if candidates.is_empty() {
        return Ok(ProbeResult {
            missing: vec![],
            present: vec![],
            skipped: vec![],
        });
    }

    let names_json = serde_json::to_string(candidates)
        .map_err(|e| format!("Failed to serialize import names: {e}"))?;

    // Embed the candidate list as a JSON literal; probe only uses find_spec (no import).
    let code = format!(
        r#"
import json, sys, importlib.util

candidates = {names_json}
stdlib = set(getattr(sys, "stdlib_module_names", ())) | set(sys.builtin_module_names)
missing, present, skipped = [], [], []
for name in candidates:
    if name in stdlib:
        skipped.append(name)
        continue
    try:
        spec = importlib.util.find_spec(name)
    except (ImportError, ModuleNotFoundError, ValueError, AttributeError):
        spec = None
    if spec is None:
        missing.append(name)
    else:
        present.append(name)
print(json.dumps({{"missing": missing, "present": present, "skipped": skipped}}))
"#,
        names_json = names_json
    );

    let result = python
        .execute_code(&code, None)
        .await
        .map_err(|e| format!("Dependency probe failed: {e}"))?;

    if !result.success {
        return Err(format!(
            "Dependency probe failed: {}",
            result
                .exception
                .or_else(|| Some(result.stderr.clone()))
                .unwrap_or_else(|| "unknown probe error".into())
        ));
    }

    let stdout = result.stdout.trim();
    // Take the last non-empty line in case of incidental output.
    let json_line = stdout
        .lines()
        .rev()
        .find(|l| l.trim().starts_with('{'))
        .unwrap_or(stdout);

    serde_json::from_str(json_line).map_err(|e| {
        format!("Failed to parse dependency probe output: {e}; stdout={stdout:?}")
    })
}

/// Parse imports from the job source, probe the active venv, and install missing packages.
///
/// Fail-fast: returns `Err` with a detailed message on the first install failure.
pub async fn resolve_and_install(
    python: &PythonExecutionService,
    spec: &JobSpec,
) -> Result<DependencyReport, String> {
    let detected = source_from_spec(spec)
        .map(parse_imports)
        .unwrap_or_default();

    let probe = probe_modules(python, &detected).await?;

    let installed_pkgs = python
        .list_packages()
        .await
        .map_err(|e| format!("Failed to list installed packages: {e}"))?;
    let installed_names: HashSet<String> = installed_pkgs
        .iter()
        .map(|p| p.name.to_ascii_lowercase())
        .collect();

    // Map missing imports → pip names (dedupe, preserve order via BTreeSet then Vec).
    let mut to_install: BTreeSet<String> = BTreeSet::new();
    let mut import_for_pip: HashMap<String, String> = HashMap::new();
    for import_name in &probe.missing {
        let pip_name = import_to_pip_name(import_name).to_string();
        import_for_pip.insert(pip_name.clone(), import_name.clone());
        if !installed_names.contains(&pip_name.to_ascii_lowercase()) {
            to_install.insert(pip_name);
        }
    }

    // Union with explicitly declared resources.packages.
    for pkg in &spec.resources.packages {
        let trimmed = pkg.trim();
        if trimmed.is_empty() {
            continue;
        }
        if !installed_names.contains(&trimmed.to_ascii_lowercase()) {
            to_install.insert(trimmed.to_string());
        }
    }

    let mut newly_installed = Vec::new();
    for pip_name in &to_install {
        log::info!("Job dependencies: installing missing package '{}'", pip_name);
        match python.install_package(pip_name, None).await {
            Ok(_) => newly_installed.push(pip_name.clone()),
            Err(e) => {
                let hint = import_for_pip
                    .get(pip_name)
                    .map(|imp| format!(" (detected from import `{imp}`)"))
                    .unwrap_or_default();
                return Err(format!(
                    "Failed to install package `{pip_name}`{hint}: {e}. \
                     Check the package name and network connectivity, then retry."
                ));
            }
        }
    }

    let mut already_present = probe.present.clone();
    // Declared packages that were already installed count as already present.
    for pkg in &spec.resources.packages {
        let trimmed = pkg.trim();
        if trimmed.is_empty() {
            continue;
        }
        if installed_names.contains(&trimmed.to_ascii_lowercase())
            && !already_present
                .iter()
                .any(|p| p.eq_ignore_ascii_case(trimmed))
            && !newly_installed
                .iter()
                .any(|p| p.eq_ignore_ascii_case(trimmed))
        {
            already_present.push(trimmed.to_string());
        }
    }
    // Missing imports whose pip package was already installed.
    for import_name in &probe.missing {
        let pip_name = import_to_pip_name(import_name);
        if installed_names.contains(&pip_name.to_ascii_lowercase())
            && !newly_installed
                .iter()
                .any(|p| p.eq_ignore_ascii_case(pip_name))
            && !already_present
                .iter()
                .any(|p| p.eq_ignore_ascii_case(import_name) || p.eq_ignore_ascii_case(pip_name))
        {
            already_present.push(import_name.clone());
        }
    }

    Ok(DependencyReport {
        detected,
        installed: newly_installed,
        already_present,
        skipped_stdlib: probe.skipped,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_imports() {
        let src = r#"
import os
import numpy as np
from sklearn.linear_model import LinearRegression
import a.b, c
from .relative import x
from __future__ import annotations
"#;
        let imports = parse_imports(src);
        assert!(imports.contains(&"os".to_string()));
        assert!(imports.contains(&"numpy".to_string()));
        assert!(imports.contains(&"sklearn".to_string()));
        assert!(imports.contains(&"a".to_string()));
        assert!(imports.contains(&"c".to_string()));
        assert!(!imports.iter().any(|i| i == "__future__"));
        assert!(!imports.iter().any(|i| i.starts_with('.')));
    }

    #[test]
    fn maps_common_import_names() {
        assert_eq!(import_to_pip_name("cv2"), "opencv-python");
        assert_eq!(import_to_pip_name("PIL"), "Pillow");
        assert_eq!(import_to_pip_name("sklearn"), "scikit-learn");
        assert_eq!(import_to_pip_name("numpy"), "numpy");
    }

    #[test]
    fn strips_inline_comments() {
        let src = "import requests  # HTTP client\n";
        let imports = parse_imports(src);
        assert_eq!(imports, vec!["requests".to_string()]);
    }
}
