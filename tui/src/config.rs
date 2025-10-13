use crate::error::{TuiError, TuiResult};
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
    pub server_config_path: String,
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
                action: "Create Cloud Folder".to_string(),
                tab: "server".to_string(),
            },
        );
        keybindings.insert(
            "<Up>".to_string(),
            Keybinding {
                action: "Previous Cloudfolder".to_string(),
                tab: "server".to_string(),
            },
        );
        keybindings.insert(
            "<Down>".to_string(),
            Keybinding {
                action: "Next Cloudfolder".to_string(),
                tab: "server".to_string(),
            },
        );
        keybindings.insert(
            "p".to_string(),
            Keybinding {
                action: "Create Password".to_string(),
                tab: "settings".to_string(),
            },
        );

        // Vim-style navigation keys
        keybindings.insert(
            "k".to_string(),
            Keybinding {
                action: "Navigate Up".to_string(),
                tab: "any".to_string(),
            },
        );
        keybindings.insert(
            "j".to_string(),
            Keybinding {
                action: "Navigate Down".to_string(),
                tab: "any".to_string(),
            },
        );
        keybindings.insert(
            "g".to_string(),
            Keybinding {
                action: "Navigate to Top".to_string(),
                tab: "any".to_string(),
            },
        );
        keybindings.insert(
            "G".to_string(),
            Keybinding {
                action: "Navigate to Bottom".to_string(),
                tab: "any".to_string(),
            },
        );

        // Focus management
        keybindings.insert(
            "<Tab>".to_string(),
            Keybinding {
                action: "Cycle Focus Forward".to_string(),
                tab: "any".to_string(),
            },
        );
        keybindings.insert(
            "<S-Tab>".to_string(),
            Keybinding {
                action: "Cycle Focus Backward".to_string(),
                tab: "any".to_string(),
            },
        );

        // Debug toggle
        keybindings.insert(
            "<leader>d".to_string(),
            Keybinding {
                action: "Toggle Debug".to_string(),
                tab: "any".to_string(),
            },
        );

        Self {
            leader: " ".to_string(),
            server_config_path: "~/.cloudtui/server.toml".to_string(),
            keybindings,
        }
    }
}

impl Config {
    pub fn load() -> TuiResult<Self> {
        // Try multiple possible locations for TUI config.toml
        let possible_paths = [
            "tui/config.toml",
            "./tui/config.toml",
            "config.toml",
            "./config.toml",
        ];

        for path in &possible_paths {
            match std::fs::read_to_string(path) {
                Ok(config_str) => {
                    // Try to parse as TUI config
                    match toml::from_str::<Config>(&config_str) {
                        Ok(config) => return Ok(config),
                        Err(_) => {
                            // If it fails, it might be a server config file, so skip it
                            continue;
                        }
                    }
                }
                Err(_) => continue,
            }
        }

        Err(TuiError::configuration(
            "Could not find valid TUI config.toml in any expected location",
        ))
    }

    pub fn load_or_default() -> Self {
        Self::load().unwrap_or_else(|_e| {
            // TUI will handle its own config logging
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
