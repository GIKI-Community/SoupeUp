use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::RwLock;

use crate::jobs::models::JobResult;

pub struct ResultStore {
    results: Arc<RwLock<HashMap<String, JobResult>>>,
    persist_path: PathBuf,
}

impl ResultStore {
    pub fn new(persist_path: PathBuf) -> Self {
        Self {
            results: Arc::new(RwLock::new(HashMap::new())),
            persist_path,
        }
    }

    pub async fn load(&self) {
        if let Ok(data) = tokio::fs::read_to_string(&self.persist_path).await {
            if let Ok(map) = serde_json::from_str::<HashMap<String, JobResult>>(&data) {
                *self.results.write().await = map;
            }
        }
    }

    async fn persist(&self) {
        let results = self.results.read().await;
        if let Some(parent) = self.persist_path.parent() {
            tokio::fs::create_dir_all(parent).await.ok();
        }
        if let Ok(json) = serde_json::to_string_pretty(&*results) {
            tokio::fs::write(&self.persist_path, json).await.ok();
        }
    }

    pub async fn put(&self, result: JobResult) {
        self.results
            .write()
            .await
            .insert(result.job_id.clone(), result);
        self.persist().await;
    }

    pub async fn get(&self, job_id: &str) -> Option<JobResult> {
        self.results.read().await.get(job_id).cloned()
    }
}
