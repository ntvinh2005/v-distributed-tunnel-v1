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

    /// Retrieves the `NodeInfo` associated with a given node ID.
    ///
    /// ### Arguments
    ///
    /// * `node_id` - A string slice representing the node ID.
    ///
    /// ### Returns
    ///
    /// An `Option<NodeInfo>` which is `Some` if a `NodeInfo` with the specified node ID exists
    /// in the registry, and `None` otherwise.
    pub fn get_by_node_id(&self, node_id: &str) -> Option<NodeInfo> {
        self.registry
            .iter()
            .find(|kv| kv.value().node_id == node_id)
            .map(|kv| kv.value().clone())
    }
}
