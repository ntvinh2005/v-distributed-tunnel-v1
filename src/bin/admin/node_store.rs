use argon2::{
    Argon2,
    password_hash::{PasswordHasher, SaltString},
};
use dashmap::DashMap;
use rand_core::OsRng;
use time::OffsetDateTime;

use super::password_gen;

#[derive(Clone)]
pub struct Node {
    pub node_id: String,
    pub password_hash: String,
    pub created_at: OffsetDateTime,
    pub last_login: Option<OffsetDateTime>,
}

#[derive(Clone)]
pub struct NodeStore {
    nodes: DashMap<String, Node>,
}

impl NodeStore {
    pub fn new() -> Self {
        Self {
            nodes: DashMap::new(),
        }
    }
    pub fn add_node(&self, node_id: String) -> String {
        let password = password_gen::generate_password();
        let argon2 = Argon2::default();
        let salt = SaltString::generate(&mut OsRng);
        let password_hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .expect("Password hashing failed")
            .to_string();
        let node = Node {
            node_id: node_id.clone(),
            password_hash: password_hash,
            created_at: OffsetDateTime::now_utc(),
            last_login: None,
        };
        self.nodes.insert(node_id, node);
        return password;
    }

    pub fn remove_node(&self, node_id: String) {
        self.nodes.remove(&node_id);
    }

    pub fn get_node(&self, node_id: String) -> Option<Node> {
        self.nodes.get(&node_id).map(|node| node.clone())
    }

    pub fn list_nodes(&self) -> Vec<Node> {
        self.nodes.iter().map(|node| node.value().clone()).collect()
    }
    pub fn set_last_login(&self, node_id: &str) {
        if let Some(mut entry) = self.nodes.get_mut(node_id) {
            entry.last_login = Some(OffsetDateTime::now_utc());
        }
    }
}
