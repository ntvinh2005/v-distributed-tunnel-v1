use super::node_store::NodeStore;
use blake3;

///new implementation verify node using reverse hash chain preimage
pub fn verify_node(node_store: &NodeStore, node_id: &str, preimage: &str) -> bool {
    if let Some(node) = node_store.get_node(node_id.to_string()) {
        let anchor = &node.anchor;

        let preimage_bytes = match hex::decode(preimage) {
            Ok(bytes) => bytes,
            Err(_) => {
                eprintln!("Invalid hex preimage from client!");
                return false;
            }
        };
        let computed = blake3::hash(&preimage_bytes);
        let computed_hex = hex::encode(computed.as_bytes());
        println!("Server computed: {}", computed_hex);
        println!("Server anchor:   {}", anchor);

        let authenticated = &computed_hex == anchor;

        if authenticated {
            //we go backward: update anchor to be the received preimage
            node_store.set_anchor(node_id, preimage);
            node_store.set_last_login(node_id);
        }

        authenticated
    } else {
        false
    }
}
