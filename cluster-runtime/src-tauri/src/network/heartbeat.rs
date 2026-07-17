//! Heartbeat module for cluster health monitoring
//! 
//! Implements heartbeat sending and monitoring for cluster nodes

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};
use crate::network::messages::Message;
use crate::network::transport::Transport;

/// Node health status
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Offline,
}

/// Heartbeat information for a node
#[derive(Debug, Clone)]
pub struct HeartbeatInfo {
    pub node_id: String,
    pub last_heartbeat: i64,
    pub status: HealthStatus,
    pub cpu_usage: f32,
    pub ram_usage: u64,
    pub ram_total: u64,
    pub worker_count: usize,
    pub active_jobs: usize,
}

impl HeartbeatInfo {
    pub fn new(node_id: String) -> Self {
        Self {
            node_id,
            last_heartbeat: chrono::Utc::now().timestamp(),
            status: HealthStatus::Healthy,
            cpu_usage: 0.0,
            ram_usage: 0,
            ram_total: 0,
            worker_count: 0,
            active_jobs: 0,
        }
    }

    pub fn update(&mut self, cpu_usage: f32, ram_usage: u64, ram_total: u64, worker_count: usize, active_jobs: usize) {
        self.last_heartbeat = chrono::Utc::now().timestamp();
        self.cpu_usage = cpu_usage;
        self.ram_usage = ram_usage;
        self.ram_total = ram_total;
        self.worker_count = worker_count;
        self.active_jobs = active_jobs;
        self.status = HealthStatus::Healthy;
    }

    pub fn mark_unhealthy(&mut self) {
        self.status = HealthStatus::Unhealthy;
    }

    pub fn mark_offline(&mut self) {
        self.status = HealthStatus::Offline;
    }

    pub fn is_timed_out(&self, timeout_seconds: i64) -> bool {
        let now = chrono::Utc::now().timestamp();
        now - self.last_heartbeat > timeout_seconds
    }
}

/// Heartbeat manager
pub struct HeartbeatManager {
    /// Heartbeat information for tracked nodes
    heartbeats: Arc<RwLock<HashMap<String, HeartbeatInfo>>>,
    /// Heartbeat interval in seconds
    interval: u64,
    /// Heartbeat timeout in seconds
    timeout: u64,
}

impl HeartbeatManager {
    pub fn new(interval: u64, timeout: u64) -> Self {
        Self {
            heartbeats: Arc::new(RwLock::new(HashMap::new())),
            interval,
            timeout,
        }
    }

    /// Register a node for heartbeat tracking
    pub async fn register_node(&self, node_id: String) {
        let mut heartbeats = self.heartbeats.write().await;
        heartbeats.entry(node_id.clone()).or_insert(HeartbeatInfo::new(node_id));
    }

    /// Update heartbeat for a node
    pub async fn update_heartbeat(
        &self,
        node_id: &str,
        cpu_usage: f32,
        ram_usage: u64,
        ram_total: u64,
        worker_count: usize,
        active_jobs: usize,
    ) {
        let mut heartbeats = self.heartbeats.write().await;
        if let Some(info) = heartbeats.get_mut(node_id) {
            info.update(cpu_usage, ram_usage, ram_total, worker_count, active_jobs);
        } else {
            let mut info = HeartbeatInfo::new(node_id.to_string());
            info.update(cpu_usage, ram_usage, ram_total, worker_count, active_jobs);
            heartbeats.insert(node_id.to_string(), info);
        }
    }

    /// Get heartbeat info for a node
    pub async fn get_heartbeat(&self, node_id: &str) -> Option<HeartbeatInfo> {
        let heartbeats = self.heartbeats.read().await;
        heartbeats.get(node_id).cloned()
    }

    /// Get all heartbeat info
    pub async fn get_all_heartbeats(&self) -> Vec<HeartbeatInfo> {
        let heartbeats = self.heartbeats.read().await;
        heartbeats.values().cloned().collect()
    }

    /// Check for timed-out nodes
    pub async fn check_timeouts(&self) -> Vec<String> {
        let mut timed_out = Vec::new();
        let mut heartbeats = self.heartbeats.write().await;
        
        for (node_id, info) in heartbeats.iter_mut() {
            if info.is_timed_out(self.timeout as i64) {
                info.mark_offline();
                timed_out.push(node_id.clone());
            }
        }
        
        timed_out
    }

    /// Remove a node from heartbeat tracking
    pub async fn remove_node(&self, node_id: &str) {
        let mut heartbeats = self.heartbeats.write().await;
        heartbeats.remove(node_id);
    }

    /// Start the heartbeat monitoring task
    pub async fn start_monitoring<F>(&self, mut on_timeout: F)
    where
        F: FnMut(String) + Send + 'static,
    {
        let manager = self.clone();
        let check_interval = self.timeout / 2;
        
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(check_interval));
            loop {
                interval.tick().await;
                let timed_out = manager.check_timeouts().await;
                for node_id in timed_out {
                    on_timeout(node_id);
                }
            }
        });
    }

    /// Start sending heartbeats from this node
    pub async fn start_sending(
        &self,
        node_id: String,
        transport: Arc<RwLock<Transport>>,
        get_resource_info: impl Fn() -> (f32, u64, u64, usize, usize) + Send + 'static,
    ) {
        let interval_duration = Duration::from_secs(self.interval);
        let node_id_clone = node_id.clone();
        
        tokio::spawn(async move {
            let mut interval = interval(interval_duration);
            loop {
                interval.tick().await;
                
                let (cpu_usage, ram_usage, ram_total, worker_count, active_jobs) = get_resource_info();
                
                let heartbeat_msg = Message::heartbeat(
                    node_id_clone.clone(),
                    cpu_usage,
                    ram_usage,
                    ram_total,
                    worker_count,
                    active_jobs,
                    "healthy".to_string(),
                );
                
                let mut transport_guard = transport.write().await;
                if let Err(e) = transport_guard.send(&heartbeat_msg).await {
                    eprintln!("Failed to send heartbeat: {}", e);
                }
            }
        });
    }
}

impl Clone for HeartbeatManager {
    fn clone(&self) -> Self {
        Self {
            heartbeats: self.heartbeats.clone(),
            interval: self.interval,
            timeout: self.timeout,
        }
    }
}

impl Default for HeartbeatManager {
    fn default() -> Self {
        Self::new(5, 30)
    }
}

/// Heartbeat event
#[derive(Debug, Clone)]
pub enum HeartbeatEvent {
    Received(HeartbeatInfo),
    Timeout(String),
    NodeOnline(String),
    NodeOffline(String),
}
