use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub cloudfolders_path: String,
    pub server_port: u16,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            cloudfolders_path: "~/.cloudtui/cloudfolders".to_string(),
            server_port: 3000,
        }
    }
}

impl ServerConfig {
    pub fn expand_path(&self) -> PathBuf {
        let path = self.cloudfolders_path.as_str();
        if path.starts_with("~/") {
            if let Some(home) = dirs::home_dir() {
                return home.join(&path[2..]);
            }
        }
        PathBuf::from(path)
    }
}
