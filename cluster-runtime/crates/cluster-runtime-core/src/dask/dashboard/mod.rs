use crate::dask::settings::DaskSettings;

/// Dask diagnostics dashboard integration.
///
/// We do not recreate the dashboard — we embed / open the official one.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardView {
    pub base_url: String,
    pub tabs: Vec<DashboardTab>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardTab {
    pub id: String,
    pub label: String,
    pub path: String,
    pub url: String,
}

pub fn dashboard_view(settings: &DaskSettings) -> DashboardView {
    let base = settings.dashboard_url();
    let tabs = [
        ("status", "Task Stream", "/status"),
        ("workers", "Worker Memory", "/workers"),
        ("graph", "Graph", "/graph"),
        ("progress", "Progress", "/individual-progress"),
        ("system", "System", "/system"),
        ("profile", "Profile", "/profile"),
    ]
    .into_iter()
    .map(|(id, label, path)| DashboardTab {
        id: id.to_string(),
        label: label.to_string(),
        path: path.to_string(),
        url: format!("{}{}", base, path),
    })
    .collect();

    DashboardView {
        base_url: base,
        tabs,
    }
}
