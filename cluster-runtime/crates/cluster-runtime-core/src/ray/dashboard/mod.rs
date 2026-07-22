use crate::ray::settings::RaySettings;

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

pub fn dashboard_view(settings: &RaySettings) -> DashboardView {
    let base = settings.dashboard_url();
    let tabs = [
        ("overview", "Overview", "/"),
        ("jobs", "Jobs", "/#/jobs"),
        ("actors", "Actors", "/#/actors"),
        ("nodes", "Nodes", "/#/nodes"),
        ("metrics", "Metrics", "/#/metrics"),
        ("logs", "Logs", "/#/logs"),
    ]
    .into_iter()
    .map(|(id, label, path)| DashboardTab {
        id: id.to_string(),
        label: label.to_string(),
        path: path.to_string(),
        url: format!("{}{}", base, path),
    })
    .collect();

    DashboardView { base_url: base, tabs }
}
