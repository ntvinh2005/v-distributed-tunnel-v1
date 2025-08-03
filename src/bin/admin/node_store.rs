use dashmap::DashMap;
use time::OffsetDateTime;
use v_distributed_tunnel_v1::common::admin::client_config::ClientConfig;

#[derive(Clone)]
pub struct Node {
    pub node_id: String,
    pub seed: String,
    pub current_index: usize,
    pub anchor: String,
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
        const CHAIN_LENGTH: usize = 100;
        let seed = ClientConfig::generate_seed();
        let seed_str = ClientConfig::encode_seed(&seed);
        let config = ClientConfig::new(
            node_id.clone(),
            seed_str.clone(),
            CHAIN_LENGTH - 1,
            CHAIN_LENGTH,
        );
        ClientConfig::write_toml_file(&config).unwrap();
        let mut hash = hex::decode(&seed_str).unwrap();
        for _ in 0..CHAIN_LENGTH {
            hash = blake3::hash(&hash).as_bytes().to_vec();
        }
        let anchor = hex::encode(&hash);
        let node = Node {
            node_id: node_id.clone(),
            seed: seed_str.clone(),
            anchor: anchor.clone(), //initially it is h_0 = hash(seed). Updated everytime login successfully
            current_index: CHAIN_LENGTH - 1,
            created_at: OffsetDateTime::now_utc(),
            last_login: None,
        };
        self.nodes.insert(node_id, node);
        return seed_str; //For the sake of debugging for this function, we return the seed, but won't be anymore in prod.
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

    fn rotate_seed(&self, node_id: &str) -> Option<String> {
        if let Some(mut entry) = self.nodes.get_mut(node_id) {
            const CHAIN_LENGTH: usize = 100;
            let new_seed = ClientConfig::generate_seed();
            let new_seed_str = ClientConfig::encode_seed(&new_seed);
            let mut hash = hex::decode(&new_seed_str).unwrap();
            for _ in 0..CHAIN_LENGTH {
                hash = blake3::hash(&hash).as_bytes().to_vec();
            }
            let anchor = hex::encode(&hash);

            //we update info of the value of entry in node_id key.
            entry.seed = new_seed_str.clone();
            entry.anchor = anchor.clone(); //initially it is h_0 = hash(seed). Updated everytime login successfully
            entry.current_index = CHAIN_LENGTH - 1;
            entry.created_at = OffsetDateTime::now_utc();
            Some(new_seed_str.clone());
        }
        None
    }

    pub fn set_anchor(&self, node_id: &str, anchor: &str) -> Option<String> {
        if let Some(mut entry) = self.nodes.get_mut(node_id) {
            entry.anchor = anchor.to_string();
            entry.current_index -= 1;
            if entry.current_index == 0 {
                return self.rotate_seed(node_id);
            }
            return None;
        }
        None
    }

    pub fn get_seed(&self, node_id: &str) -> Option<String> {
        self.nodes.get(node_id).map(|node| node.seed.clone())
    }
}
