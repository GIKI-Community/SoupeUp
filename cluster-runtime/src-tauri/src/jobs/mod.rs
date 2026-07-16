use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum JobStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Job {
    pub id: String,
    pub status: JobStatus,
    pub owner: String,
    pub submitted_at: DateTime<Utc>,
    pub runtime: String,
    pub duration_secs: u64,
}

pub fn mock_jobs() -> Vec<Job> {
    vec![
        Job {
            id: "job-a1b2c3".into(),
            status: JobStatus::Running,
            owner: "alice@cluster.local".into(),
            submitted_at: Utc::now() - chrono::Duration::minutes(45),
            runtime: "native".into(),
            duration_secs: 2700,
        },
        Job {
            id: "job-d4e5f6".into(),
            status: JobStatus::Completed,
            owner: "bob@cluster.local".into(),
            submitted_at: Utc::now() - chrono::Duration::hours(3),
            runtime: "ray".into(),
            duration_secs: 5400,
        },
        Job {
            id: "job-g7h8i9".into(),
            status: JobStatus::Pending,
            owner: "carol@cluster.local".into(),
            submitted_at: Utc::now() - chrono::Duration::minutes(2),
            runtime: "htcondor".into(),
            duration_secs: 0,
        },
        Job {
            id: "job-j0k1l2".into(),
            status: JobStatus::Running,
            owner: "dave@cluster.local".into(),
            submitted_at: Utc::now() - chrono::Duration::minutes(12),
            runtime: "native".into(),
            duration_secs: 720,
        },
        Job {
            id: "job-m3n4o5".into(),
            status: JobStatus::Failed,
            owner: "eve@cluster.local".into(),
            submitted_at: Utc::now() - chrono::Duration::hours(1),
            runtime: "ray".into(),
            duration_secs: 180,
        },
        Job {
            id: "job-p6q7r8".into(),
            status: JobStatus::Running,
            owner: "frank@cluster.local".into(),
            submitted_at: Utc::now() - chrono::Duration::minutes(90),
            runtime: "native".into(),
            duration_secs: 5400,
        },
        Job {
            id: "job-s9t0u1".into(),
            status: JobStatus::Cancelled,
            owner: "grace@cluster.local".into(),
            submitted_at: Utc::now() - chrono::Duration::hours(5),
            runtime: "htcondor".into(),
            duration_secs: 300,
        },
    ]
}
