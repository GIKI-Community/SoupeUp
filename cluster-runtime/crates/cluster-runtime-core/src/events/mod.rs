use std::sync::Arc;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

/// Strongly typed cluster events for the internal event bus.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload", rename_all = "camelCase")]
pub enum ClusterEvent {
    NodeConnected { node_id: String },
    NodeDisconnected { node_id: String },
    PluginInstalled { plugin_id: String },
    PluginRemoved { plugin_id: String },
    JobStarted { job_id: String },
    JobFinished { job_id: String, success: bool },
    MetricUpdated { metric_name: String },
}

pub trait EventHandler: Send + Sync {
    fn handle(&self, event: &ClusterEvent);
}

type HandlerRef = Arc<dyn EventHandler>;

/// Thread-safe publish/subscribe event bus.
pub struct EventBus {
    handlers: RwLock<Vec<HandlerRef>>,
    history: RwLock<Vec<ClusterEvent>>,
    max_history: usize,
}

impl EventBus {
    pub fn new(max_history: usize) -> Self {
        Self {
            handlers: RwLock::new(Vec::new()),
            history: RwLock::new(Vec::new()),
            max_history,
        }
    }

    pub fn subscribe(&self, handler: HandlerRef) {
        self.handlers.write().push(handler);
    }

    pub fn publish(&self, event: ClusterEvent) {
        {
            let mut history = self.history.write();
            history.push(event.clone());
            if history.len() > self.max_history {
                let drain_count = history.len() - self.max_history;
                history.drain(0..drain_count);
            }
        }

        let handlers = self.handlers.read();
        for handler in handlers.iter() {
            handler.handle(&event);
        }
    }

    pub fn recent_events(&self, limit: usize) -> Vec<ClusterEvent> {
        let history = self.history.read();
        let start = history.len().saturating_sub(limit);
        history[start..].to_vec()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new(100)
    }
}
