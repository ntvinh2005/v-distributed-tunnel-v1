use dashmap::DashMap;
use rand::seq::SliceRandom;
use rand::thread_rng;
use sqlx::types::time::OffsetDateTime;
use std::sync::Arc;

#[derive(Clone)]
pub struct Port {
    port: u16,
    assigned: bool,
    assign_to: Option<String>, //This is the node id that this port is assigned to
    assign_at: Option<OffsetDateTime>, //Optional since maybe node that is not connected yet won't have a timestamp
}

pub enum StaticPortAssignResult {
    Success(u16),
    SeedMissing,
    SeedHexInvalid,
    PortInUse(u16),
}

#[derive(Clone)]
pub struct PortPool {
    pool: Arc<DashMap<u16, Port>>, //To sum, it's a pool of ports
}

impl PortPool {
    pub fn new(start: u16, end: u16) -> Self {
        let pool = Arc::new(DashMap::new());
        //At init, all ports are unassigned so assigned = falsem assign_to = None, assign_at = None
        for port in start..=end {
            pool.insert(
                port,
                Port {
                    port,
                    assigned: false,
                    assign_to: None,
                    assign_at: None,
                },
            );
        }
        return Self { pool };
    }

    pub fn assign_random_port(&self, node_id: &str) -> Option<u16> {
        let mut rng = thread_rng();
        let available_ports: Vec<u16> = self
            .pool
            .iter()
            .filter_map(|entry| {
                if !entry.value().assigned {
                    Some(*entry.key())
                } else {
                    None
                }
            })
            .collect(); //basically just check assigned and put all unassigned into a vector

        if available_ports.len() == 0 {
            return None;
        }

        let &random_port = available_ports.choose(&mut rng).unwrap();
        if 5001 <= random_port && random_port <= 5999 {
            let mut port_data = self.pool.get_mut(&random_port).unwrap();
            port_data.assigned = true;
            port_data.assign_to = Some(node_id.to_string());
            port_data.assign_at = Some(OffsetDateTime::now_utc());
            Some(random_port)
        } else {
            None
        }
    }

    fn static_port_from_seed(seed: &[u8]) -> u16 {
        let hash = blake3::hash(seed);
        5000 + (hash.as_bytes()[0] as u16 % 1000)
    }

    pub fn assign_static_port(
        &self,
        node_id: &str,
        seed_hex_opt: Option<&str>,
    ) -> StaticPortAssignResult {
        use StaticPortAssignResult::*;
        //check if seed present
        let seed_hex = match seed_hex_opt {
            Some(s) => s,
            None => return SeedMissing,
        };

        //we decode hex
        let seed_bytes = match hex::decode(seed_hex) {
            Ok(b) => b,
            Err(_) => return SeedHexInvalid,
        };

        let port = Self::static_port_from_seed(&seed_bytes);

        //only assign if that port is available
        if let Some(mut port_data) = self.pool.get_mut(&port) {
            if !port_data.assigned {
                port_data.assigned = true;
                port_data.assign_to = Some(node_id.to_string());
                port_data.assign_at = Some(OffsetDateTime::now_utc());
                Success(port)
            } else {
                PortInUse(port)
            }
        } else {
            // Port not in pool, treat as in use or invalid
            PortInUse(port)
        }
    }

    //Release a port
    //trigger after the client is disconnected
    pub fn release_port(&self, port: u16) {
        let mut target_port = self.pool.get_mut(&port).unwrap();
        target_port.assigned = false;
        target_port.assign_to = None;
        target_port.assign_at = None;
    }
}

//Simple guard struct to ensure the port is always released, even on panic or early return.
pub struct PortGuard {
    pub port_pool: Arc<PortPool>,
    pub port: u16,
    pub node_id: String,
}

impl Drop for PortGuard {
    fn drop(&mut self) {
        self.port_pool.release_port(self.port);
        println!("Released port {} from node '{}'", self.port, self.node_id);
    }
}
