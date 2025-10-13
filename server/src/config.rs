use crate::error::ServerResult;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub fn generate_jwt_secret() -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use std::time::{SystemTime, UNIX_EPOCH};

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let mut hasher = DefaultHasher::new();
    timestamp.hash(&mut hasher);
    format!("cloudtui-secret-{:x}", hasher.finish())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub cloudfolders_path: String,
    pub server_port: u16,
    pub password_hash: Option<String>,
    pub password_changed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub jwt_secret: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            cloudfolders_path: "~/.cloudhost/cloudfolders".to_string(),
            server_port: 3000,
            password_hash: None,
            password_changed_at: None,
            jwt_secret: generate_jwt_secret(),
        }
    }
}

impl ServerConfig {
    pub fn expand_path(&self) -> PathBuf {
        let path = self.cloudfolders_path.as_str();
        if let Some(stripped) = path.strip_prefix("~/") {
            if let Some(home) = dirs::home_dir() {
                return home.join(stripped);
            }
        }
        PathBuf::from(path)
    }

    pub fn has_password(&self) -> bool {
        self.password_hash.is_some()
    }

    pub fn set_password(&mut self, password: &str) -> Result<(), bcrypt::BcryptError> {
        let hash = bcrypt::hash(password, bcrypt::DEFAULT_COST)?;
        self.password_hash = Some(hash);
        self.password_changed_at = Some(chrono::Utc::now());
        Ok(())
    }

    pub fn verify_password(&self, password: &str) -> bool {
        if let Some(ref hash) = self.password_hash {
            bcrypt::verify(password, hash).unwrap_or(false)
        } else {
            false
        }
    }

    pub fn load_from_file() -> ServerResult<Self> {
        let config_path = "server/config.toml";
        if std::path::Path::new(config_path).exists() {
            let config_str = std::fs::read_to_string(config_path)?;
            let config: ServerConfig = toml::from_str(&config_str)?;
            Ok(config)
        } else {
            Ok(Self::default())
        }
    }

    pub fn save_to_file(&self) -> ServerResult<()> {
        let config_str = toml::to_string_pretty(self)?;
        std::fs::write("server/config.toml", config_str)?;
        Ok(())
    }
}
