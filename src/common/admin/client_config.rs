use rand_core::RngCore;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Write;

#[derive(Serialize, Deserialize)]
pub struct ClientConfig {
    pub node_id: String,
    pub seed: String, //both of these two props are required for reverse hash chain
    pub current_index: usize,
    pub chain_length: usize,
}

impl ClientConfig {
    pub fn new(node_id: String, seed: String, current_index: usize, chain_length: usize) -> Self {
        Self {
            node_id,
            seed,
            current_index,
            chain_length,
        }
    }

    //This function generate 32bytes random using OS random number generator (which is not so random :'( )
    pub fn generate_seed() -> [u8; 32] {
        let mut seed = [0u8; 32];
        rand::rngs::OsRng.fill_bytes(&mut seed);
        seed
    }

    //Take a byte slice (which is our seed) and return its hexadecimal representation as a string
    pub fn encode_seed(seed: &[u8]) -> String {
        hex::encode(seed)
    }

    pub fn write_toml_file(config: &ClientConfig) -> std::io::Result<()> {
        let toml = toml::to_string(config).unwrap();
        let mut file = File::create("config.toml")?;
        file.write_all(toml.as_bytes())?;
        Ok(())
    }
}
