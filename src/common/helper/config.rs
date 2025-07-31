use crate::common::admin::client_config::ClientConfig;
use std::fs;

pub fn load_config(path: &str) -> ClientConfig {
    let data = fs::read_to_string(path).expect("Failed to read config file");
    toml::from_str(&data).expect("Invalid config TOML")
}

pub fn save_config(path: &str, config: &ClientConfig) {
    let toml = toml::to_string(config).expect("Failed to serialize config");
    fs::write(path, toml).expect("Failed to write config file");
}
