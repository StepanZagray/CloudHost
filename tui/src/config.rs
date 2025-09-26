use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Keybinding {
    pub action: String,
    pub tab: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub leader: String,
    pub profiles_path: String,
    pub keybindings: HashMap<String, Keybinding>,
}

impl Default for Config {
    fn default() -> Self {
        let mut keybindings = HashMap::new();
        keybindings.insert(
            "q".to_string(),
            Keybinding {
                action: "Quit".to_string(),
                tab: "any".to_string(),
            },
        );
        keybindings.insert(
            "gt".to_string(),
            Keybinding {
                action: "Next Tab".to_string(),
                tab: "any".to_string(),
            },
        );
        keybindings.insert(
            "gT".to_string(),
            Keybinding {
                action: "Previous Tab".to_string(),
                tab: "any".to_string(),
            },
        );
        keybindings.insert(
            "s".to_string(),
            Keybinding {
                action: "Start/Stop Server".to_string(),
                tab: "server".to_string(),
            },
        );
        keybindings.insert(
            "n".to_string(),
            Keybinding {
                action: "Create Profile".to_string(),
                tab: "server".to_string(),
            },
        );
        keybindings.insert(
            "<Up>".to_string(),
            Keybinding {
                action: "Previous Profile".to_string(),
                tab: "server".to_string(),
            },
        );
        keybindings.insert(
            "<Down>".to_string(),
            Keybinding {
                action: "Next Profile".to_string(),
                tab: "server".to_string(),
            },
        );

        Self {
            leader: " ".to_string(),
            profiles_path: "~/.cloudtui/profiles".to_string(),
            keybindings,
        }
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
        self.keybindings.get(key).map(|kb| &kb.action)
    }

    pub fn get_keybinding(&self, key: &str) -> Option<&Keybinding> {
        self.keybindings.get(key)
    }

    pub fn is_key_valid_for_tab(&self, key: &str, current_tab: &str) -> bool {
        if let Some(keybinding) = self.keybindings.get(key) {
            keybinding.tab == "any" || keybinding.tab == current_tab
        } else {
            false
        }
    }

    pub fn get_keys_for_action(&self, action: &str) -> Vec<String> {
        self.keybindings
            .iter()
            .filter(|(_, value)| value.action == action)
            .map(|(key, _)| key.clone())
            .collect()
    }
}
