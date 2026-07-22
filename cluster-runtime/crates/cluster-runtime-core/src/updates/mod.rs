//! Check GitHub Releases for a newer app version (notify-only; no auto-install).

use chrono::Utc;
use serde::{Deserialize, Serialize};

const DEFAULT_REPO: &str = "GIKI-Community/SoupeUp";
const USER_AGENT: &str = "cluster-runtime-updater";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCheckResult {
    pub current_version: String,
    pub latest_version: Option<String>,
    pub update_available: bool,
    pub release_url: Option<String>,
    pub release_notes: Option<String>,
    pub checked_at: String,
    pub message: String,
}

#[derive(Debug, Deserialize)]
struct GithubRelease {
    tag_name: String,
    html_url: String,
    body: Option<String>,
    draft: bool,
    prerelease: bool,
}

fn repo_slug() -> String {
    std::env::var("CLUSTER_RUNTIME_UPDATE_REPO").unwrap_or_else(|_| DEFAULT_REPO.to_string())
}

fn normalize_version(raw: &str) -> String {
    raw.trim().trim_start_matches('v').trim_start_matches('V').to_string()
}

fn is_newer(latest: &str, current: &str) -> bool {
    let Ok(latest_v) = semver::Version::parse(&normalize_version(latest)) else {
        return normalize_version(latest) != normalize_version(current)
            && normalize_version(latest) > normalize_version(current);
    };
    let Ok(current_v) = semver::Version::parse(&normalize_version(current)) else {
        return true;
    };
    latest_v > current_v
}

/// Query GitHub Releases for the configured repo and compare to this build's version.
pub async fn check_for_updates() -> Result<UpdateCheckResult, String> {
    let current_version = env!("CARGO_PKG_VERSION").to_string();
    let repo = repo_slug();
    let url = format!("https://api.github.com/repos/{repo}/releases/latest");
    let checked_at = Utc::now().to_rfc3339();

    let client = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {e}"))?;

    let response = client
        .get(&url)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| format!("Failed to reach GitHub: {e}"))?;

    let status = response.status();
    if status.as_u16() == 404 {
        return Ok(UpdateCheckResult {
            current_version,
            latest_version: None,
            update_available: false,
            release_url: Some(format!("https://github.com/{repo}/releases")),
            release_notes: None,
            checked_at,
            message: "No GitHub releases found yet.".into(),
        });
    }

    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(format!(
            "GitHub returned {status}: {}",
            body.chars().take(200).collect::<String>()
        ));
    }

    let release: GithubRelease = response
        .json()
        .await
        .map_err(|e| format!("Invalid GitHub response: {e}"))?;

    if release.draft || release.prerelease {
        return Ok(UpdateCheckResult {
            current_version,
            latest_version: None,
            update_available: false,
            release_url: Some(release.html_url),
            release_notes: release.body,
            checked_at,
            message: "Latest GitHub release is a draft or prerelease; ignoring.".into(),
        });
    }

    let latest = normalize_version(&release.tag_name);
    let update_available = is_newer(&latest, &current_version);

    Ok(UpdateCheckResult {
        message: if update_available {
            format!("Update available: v{latest} (you have v{current_version}).")
        } else {
            format!("You are up to date (v{current_version}).")
        },
        current_version,
        latest_version: Some(latest),
        update_available,
        release_url: Some(release.html_url),
        release_notes: release.body,
        checked_at,
    })
}

#[cfg(test)]
mod tests {
    use super::{is_newer, normalize_version};

    #[test]
    fn strips_v_prefix() {
        assert_eq!(normalize_version("v1.2.3"), "1.2.3");
        assert_eq!(normalize_version("1.2.3"), "1.2.3");
    }

    #[test]
    fn detects_newer_semver() {
        assert!(is_newer("0.2.0", "0.1.0"));
        assert!(!is_newer("0.1.0", "0.1.0"));
        assert!(!is_newer("0.1.0", "0.2.0"));
        assert!(is_newer("v0.1.1", "0.1.0"));
    }
}
