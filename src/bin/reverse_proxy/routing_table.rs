use dashmap::DashMap;

//For example, host will be api.example.com
//path will be 127.0.0.1:8080
// RoutingTable {
//   table: {
//     "api.example.com": vec![
//       ("/v1/", "127.0.0.1:8000".to_string()),
//       ("/admin", "127.0.0.1:8001".to_string()),
//       ("/", "127.0.0.1:8002".to_string()), // catch-all for this host
//     ],
//     "admin.example.com": vec![
//       ("/", "127.0.0.1:9000".to_string())
//     ]
//   },
//   default_backend: Some("127.0.0.1:8080".to_string())
// }

#[derive(Clone)]
pub struct RoutingTable {
    pub table: DashMap<String, Vec<(String, String)>>,
    pub default_backend_addr: Vec<String>,
}

impl RoutingTable {
    /// Create a new, empty routing table.
    ///
    /// The default backend address is set to `"127.0.0.1:8080"` as known as localhost.
    /// Nothing better than home
    pub fn new() -> Self {
        Self {
            table: DashMap::new(),
            default_backend_addr: ["127.0.0.1:8080".to_string()].to_vec(),
        }
    }

    /// Insert a new rule into the routing table.
    ///
    /// Given a host, path, and a backend address, this function will
    /// insert the path and backend address into the host's list of
    /// rules. If the host does not exist, it will be created. If the
    /// host does exist, the rule will be added to the existing list of
    /// rules.
    pub fn insert_rule(&self, host: String, path: String, backend_addr: String) {
        let rule = (path, backend_addr);
        self.table.entry(host).or_insert(Vec::new()).push(rule); //insert if there no host key, push if there is 
    }

    /// Remove a rule from the routing table.
    ///
    /// Given a host, and a path, this function will remove the path from the host's
    /// list of rules. If the host does not exist, this function does nothing.
    pub fn remove_rule(&self, host: String, path: String) {
        if let Some(mut rules) = self.table.get_mut(&host) {
            rules.retain(|(prefix, _)| prefix != &path); //only keep rules that do not match the path
        }
    }

    /// Given a host, return a vector of (path_prefix, backend_addr) that are associated with that host.
    /// If no host is found, return None.
    pub fn lookup(&self, host: String) -> Option<Vec<(String, String)>> {
        self.table.get(&host).map(|ref_val| ref_val.value().clone())
    }

    //find the host in the map.
    //iterate over the path rules (sorted by length descending).
    //find the first path_prefix where path.starts_with(path_prefix).
    //return the corresponding backend address.
    //fallback: If no rule matches, use default_backend.
    pub fn lookup_with_path(&self, host: String, path: String) -> Option<String> {
        if let Some(rules) = self.table.get(&host) {
            let mut sorted_rules = rules.clone();
            sorted_rules.sort_by_key(|(prefix, _)| std::cmp::Reverse(prefix.len()));

            for (prefix, backend) in sorted_rules {
                //if there is prefix, return backend
                if path.starts_with(prefix.as_str()) {
                    return Some(backend);
                }
            }
        }

        //if no rule matches, use default backend
        Some(self.default_backend_addr[0].clone())
    }

    pub fn update_backend_addr(&mut self, host: String, path: String, new_backend_addr: String) {
        if let Some(mut rules) = self.table.get_mut(&host) {
            for (prefix, backend) in rules.iter_mut() {
                if *prefix == path {
                    *backend = new_backend_addr;
                    break;
                }
            }
        }
    }
}

pub fn setup_routing_table() -> RoutingTable {
    let routing_table = RoutingTable::new();
    routing_table.insert_rule(
        "api.example.com".to_string(),
        "/v1/".to_string(),
        "laptop_1:8080".to_string(),
    );
    routing_table.insert_rule(
        "api.example.com".to_string(),
        "/admin".to_string(),
        "laptop_1:8001".to_string(),
    );
    routing_table.insert_rule(
        "api.example.com".to_string(),
        "/".to_string(),
        "laptop_1:8002".to_string(),
    );
    routing_table.insert_rule(
        "admin.example.com".to_string(),
        "/".to_string(),
        "laptop_1:9000".to_string(),
    );
    routing_table
}
