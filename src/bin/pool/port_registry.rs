use dashmap::DashMap;
use quinn::Connection;
use std::sync::Arc;

#[derive(Clone)]
pub struct NodeInfo {
    pub conn: Connection,
    pub node_id: String,
}

#[derive(Clone)]
pub struct PortRegistry {
    registry: Arc<DashMap<u16, NodeInfo>>,
}

impl PortRegistry {
    pub fn new() -> Self {
        Self {
            registry: Arc::new(DashMap::new()),
        }
    }

    pub fn insert(&self, port: u16, node_info: NodeInfo) {
        self.registry.insert(port, node_info);
    }

    pub fn get(&self, port: &u16) -> Option<NodeInfo> {
        self.registry.get(port).map(|entry| entry.clone())
    }

    pub fn remove(&self, port: &u16) {
        self.registry.remove(port);
    }
}
