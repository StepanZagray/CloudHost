use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub keybindings: HashMap<String, String>,
}

impl Default for Config {
    fn default() -> Self {
        let mut keybindings = HashMap::new();
        keybindings.insert("q".to_string(), "Quit".to_string());
        keybindings.insert("gt".to_string(), "Next Tab".to_string());
        keybindings.insert("gT".to_string(), "Previous Tab".to_string());

        Self { keybindings }
    }
}

impl Config {
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let config_str = std::fs::read_to_string("config.toml")?;
        let config: Config = toml::from_str(&config_str)?;
        Ok(config)
    }

    pub fn load_or_default() -> Self {
        Self::load().unwrap_or_else(|_| {
            println!("Warning: Could not load config.toml, using default keybindings");
            Self::default()
        })
    }

    pub fn get_action(&self, key: &str) -> Option<&String> {
        self.keybindings.get(key)
    }

    pub fn get_keys_for_action(&self, action: &str) -> Vec<String> {
        self.keybindings
            .iter()
            .filter(|(_, value)| value.as_str() == action)
            .map(|(key, _)| key.clone())
            .collect()
    }
}
