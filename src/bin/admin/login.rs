use super::node_store::NodeStore;
use argon2::{Argon2, PasswordHash, PasswordVerifier};

pub fn verify_node(node_store: &NodeStore, node_id: &str, password: &str) -> bool {
    if let Some(node) = node_store.get_node(node_id.to_string()) {
        if let Ok(hash) = PasswordHash::new(&node.password_hash) {
            let authenticated = Argon2::default()
                .verify_password(password.as_bytes(), &hash)
                .is_ok();
            if authenticated {
                node_store.set_last_login(node_id);
            }
            authenticated
        } else {
            false
        }
    } else {
        false
    }
}
