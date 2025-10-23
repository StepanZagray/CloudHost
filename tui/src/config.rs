use crate::error::{TuiError, TuiResult};
use cloudhost_server::config_paths;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    pub keys: Vec<String>,
    pub tab: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub leader: String,
    pub actions: HashMap<String, Action>,
}

impl Default for Config {
    fn default() -> Self {
        let mut actions = HashMap::new();
        actions.insert(
            "Quit".to_string(),
            Action {
                keys: vec!["q".to_string()],
                tab: "any".to_string(),
            },
        );
        actions.insert(
            "Next Tab".to_string(),
            Action {
                keys: vec!["gt".to_string()],
                tab: "any".to_string(),
            },
        );
        actions.insert(
            "Previous Tab".to_string(),
            Action {
                keys: vec!["gT".to_string()],
                tab: "any".to_string(),
            },
        );
        actions.insert(
            "Start/Stop Cloud".to_string(),
            Action {
                keys: vec!["s".to_string()],
                tab: "clouds".to_string(),
            },
        );
        actions.insert(
            "Create New".to_string(),
            Action {
                keys: vec!["n".to_string()],
                tab: "folders".to_string(),
            },
        );
        actions.insert(
            "Delete Folder".to_string(),
            Action {
                keys: vec!["d".to_string(), "x".to_string()],
                tab: "folders".to_string(),
            },
        );
        actions.insert(
            "Delete Cloud".to_string(),
            Action {
                keys: vec!["D".to_string(), "X".to_string()],
                tab: "folders".to_string(),
            },
        );
        actions.insert(
            "Select All Folders".to_string(),
            Action {
                keys: vec!["a".to_string()],
                tab: "folders".to_string(),
            },
        );
        actions.insert(
            "Edit".to_string(),
            Action {
                keys: vec!["e".to_string()],
                tab: "folders".to_string(),
            },
        );
        actions.insert(
            "Create Password".to_string(),
            Action {
                keys: vec!["p".to_string()],
                tab: "clouds".to_string(),
            },
        );
        actions.insert(
            "Set Password".to_string(),
            Action {
                keys: vec!["p".to_string()],
                tab: "folders".to_string(),
            },
        );
        actions.insert(
            "Toggle Password Visibility".to_string(),
            Action {
                keys: vec!["<leader>p".to_string()],
                tab: "folders".to_string(),
            },
        );
        actions.insert(
            "Reload TUI Config".to_string(),
            Action {
                keys: vec!["<leader>r".to_string()],
                tab: "settings".to_string(),
            },
        );
        actions.insert(
            "Reload Clouds Config".to_string(),
            Action {
                keys: vec!["<leader>c".to_string()],
                tab: "settings".to_string(),
            },
        );
        actions.insert(
            "Reload All Configs".to_string(),
            Action {
                keys: vec!["<leader>R".to_string()],
                tab: "settings".to_string(),
            },
        );

        // Vim-style navigation keys
        actions.insert(
            "Navigate Up".to_string(),
            Action {
                keys: vec!["k".to_string(), "<Up>".to_string()],
                tab: "any".to_string(),
            },
        );
        actions.insert(
            "Navigate Down".to_string(),
            Action {
                keys: vec!["j".to_string(), "<Down>".to_string()],
                tab: "any".to_string(),
            },
        );
        actions.insert(
            "Navigate to Top".to_string(),
            Action {
                keys: vec!["g".to_string(), "gg".to_string()],
                tab: "any".to_string(),
            },
        );
        actions.insert(
            "Navigate to Bottom".to_string(),
            Action {
                keys: vec!["G".to_string()],
                tab: "any".to_string(),
            },
        );

        // Focus management
        actions.insert(
            "Cycle Focus Forward".to_string(),
            Action {
                keys: vec!["<Tab>".to_string()],
                tab: "any".to_string(),
            },
        );
        actions.insert(
            "Cycle Focus Backward".to_string(),
            Action {
                keys: vec!["<S-Tab>".to_string()],
                tab: "any".to_string(),
            },
        );

        // Debug toggle
        actions.insert(
            "Toggle Debug".to_string(),
            Action {
                keys: vec!["<leader>d".to_string()],
                tab: "any".to_string(),
            },
        );

        // Additional nvim-style keybinds
        actions.insert(
            "Toggle Selection".to_string(),
            Action {
                keys: vec!["<leader>".to_string(), " ".to_string()],
                tab: "folders".to_string(),
            },
        );
        actions.insert(
            "Refresh/Reload".to_string(),
            Action {
                keys: vec!["r".to_string(), "<C-r>".to_string()],
                tab: "any".to_string(),
            },
        );
        actions.insert(
            "Execute Action".to_string(),
            Action {
                keys: vec!["<Enter>".to_string()],
                tab: "settings".to_string(),
            },
        );

        Self {
            leader: " ".to_string(),
            actions,
        }
    }
}

impl Config {
    pub fn load() -> TuiResult<Self> {
        let config_path = config_paths::get_tui_config_path();

        match std::fs::read_to_string(&config_path) {
            Ok(config_str) => match toml::from_str::<Config>(&config_str) {
                Ok(config) => Ok(config),
                Err(e) => Err(TuiError::configuration(format!(
                    "Failed to parse TUI config: {}",
                    e
                ))),
            },
            Err(_) => Err(TuiError::configuration(format!(
                "Could not find TUI config at: {:?}",
                config_path
            ))),
        }
    }

    pub fn load_or_default() -> Self {
        match Self::load() {
            Ok(mut config) => {
                // Check if config is missing important keys and migrate if needed
                if config.needs_migration() {
                    config.migrate_to_latest();
                    let _ = config.save_to_file(); // Save the migrated config
                }
                config
            }
            Err(_e) => {
                // TUI will handle its own config logging
                let default_config = Self::default();
                // Try to save the default config for future use
                let _ = default_config.save_to_file();
                default_config
            }
        }
    }

    pub fn save_to_file(&self) -> TuiResult<()> {
        let config_path = config_paths::get_tui_config_path();

        // Ensure the config directory exists
        config_paths::ensure_config_dir().map_err(|e| {
            TuiError::configuration(format!("Failed to create config directory: {}", e))
        })?;

        let config_str = toml::to_string_pretty(self)
            .map_err(|e| TuiError::configuration(format!("Failed to serialize config: {}", e)))?;

        std::fs::write(&config_path, config_str)
            .map_err(|e| TuiError::configuration(format!("Failed to write config file: {}", e)))?;

        Ok(())
    }

    pub fn get_action_for_key(&self, key: &str) -> Option<String> {
        for (action_name, action) in &self.actions {
            if action.keys.contains(&key.to_string()) {
                return Some(action_name.clone());
            }
        }
        None
    }

    pub fn get_action(&self, action_name: &str) -> Option<&Action> {
        self.actions.get(action_name)
    }

    pub fn is_key_valid_for_tab(&self, key: &str, current_tab: &str) -> bool {
        for action in self.actions.values() {
            if action.keys.contains(&key.to_string()) {
                return action.tab == "any" || action.tab == current_tab;
            }
        }
        false
    }

    pub fn get_keys_for_action(&self, action: &str) -> Vec<String> {
        self.actions
            .get(action)
            .map(|action| action.keys.clone())
            .unwrap_or_default()
    }

    pub fn reset_to_default() -> TuiResult<()> {
        let config_path = config_paths::get_tui_config_path();

        // Remove the existing config file if it exists
        if config_path.exists() {
            std::fs::remove_file(&config_path).map_err(|e| {
                TuiError::configuration(format!("Failed to remove config file: {}", e))
            })?;
        }

        // Create and save the default config
        let default_config = Self::default();
        default_config.save_to_file()?;

        Ok(())
    }

    /// Check if the config needs migration (missing important keys)
    fn needs_migration(&self) -> bool {
        // Check if Execute Action for settings tab is missing
        !self.actions.contains_key("Execute Action")
            || !self.actions.get("Execute Action").is_some_and(|action| {
                action.keys.contains(&"<Enter>".to_string()) && action.tab == "settings"
            })
    }

    /// Migrate config to latest version by adding missing actions
    fn migrate_to_latest(&mut self) {
        // Add Execute Action for settings tab if missing
        if !self.actions.contains_key("Execute Action") {
            self.actions.insert(
                "Execute Action".to_string(),
                Action {
                    keys: vec!["<Enter>".to_string()],
                    tab: "settings".to_string(),
                },
            );
        }
    }
}
