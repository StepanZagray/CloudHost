use crate::error::{TuiError, TuiResult};
use cloudhost_server::config_paths;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    pub keys: Vec<String>,
    pub tab: String,
}

impl Action {
    pub fn applies_to(&self, current_tab: &str) -> bool {
        self.tab == "any"
            || self
                .tab
                .split(',')
                .map(str::trim)
                .any(|tab| tab == current_tab)
    }
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
                tab: "dashboard".to_string(),
            },
        );
        actions.insert(
            "Open Cloud".to_string(),
            Action {
                keys: vec!["o".to_string()],
                tab: "dashboard".to_string(),
            },
        );
        actions.insert(
            "Add Folder".to_string(),
            Action {
                keys: vec!["a".to_string()],
                tab: "storage".to_string(),
            },
        );
        actions.insert(
            "Create Cloud".to_string(),
            Action {
                keys: vec!["n".to_string()],
                tab: "storage".to_string(),
            },
        );
        actions.insert(
            "Remove Folder / Cloud".to_string(),
            Action {
                keys: vec!["d".to_string(), "x".to_string()],
                tab: "storage".to_string(),
            },
        );
        actions.insert(
            "Select All Folders".to_string(),
            Action {
                keys: vec!["A".to_string()],
                tab: "storage".to_string(),
            },
        );
        actions.insert(
            "Edit".to_string(),
            Action {
                keys: vec!["e".to_string()],
                tab: "storage".to_string(),
            },
        );
        actions.insert(
            "Set Cloud Password".to_string(),
            Action {
                keys: vec!["p".to_string()],
                tab: "dashboard,storage".to_string(),
            },
        );
        actions.insert(
            "Toggle Password Visibility".to_string(),
            Action {
                keys: vec!["<leader>p".to_string()],
                tab: "storage".to_string(),
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
            "Reload Storage Config".to_string(),
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
                keys: vec!["gg".to_string()],
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
        actions.insert(
            "Toggle Help".to_string(),
            Action {
                keys: vec!["?".to_string()],
                tab: "any".to_string(),
            },
        );

        // Additional nvim-style keybinds
        actions.insert(
            "Toggle Selection".to_string(),
            Action {
                keys: vec!["v".to_string()],
                tab: "storage".to_string(),
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

    pub fn get_action_for_key(&self, key: &str, current_tab: &str) -> Option<String> {
        // Sorting makes conflicting user-defined bindings deterministic. The default
        // configuration does not contain conflicts within the same tab.
        let mut matches = self
            .actions
            .iter()
            .filter(|(_, action)| action.keys.iter().any(|binding| binding == key))
            .filter(|(_, action)| action.applies_to(current_tab))
            .map(|(name, action)| (action.tab == "any", name))
            .collect::<Vec<_>>();
        matches.sort();
        matches.first().map(|(_, name)| (*name).clone())
    }

    pub fn get_action(&self, action_name: &str) -> Option<&Action> {
        self.actions.get(action_name)
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
        // Check for actions and tab names introduced by the storage/dashboard rewrite.
        !self.actions.contains_key("Execute Action")
            || !self.actions.get("Execute Action").is_some_and(|action| {
                action.keys.contains(&"<Enter>".to_string()) && action.tab == "settings"
            })
            || !self.actions.contains_key("Toggle Help")
            || !self.actions.contains_key("Open Cloud")
            || !self.actions.contains_key("Add Folder")
            || !self.actions.contains_key("Create Cloud")
            || !self.actions.contains_key("Remove Folder / Cloud")
            || !self.actions.contains_key("Select All Folders")
            || !self.actions.contains_key("Set Cloud Password")
            || !self.actions.contains_key("Reload Storage Config")
            || self.actions.contains_key("Create New")
            || self.actions.contains_key("Add Source / Cloud")
            || self.actions.contains_key("Remove Source")
            || self.actions.contains_key("Remove Cloud")
            || self.actions.contains_key("Delete Folder")
            || self.actions.contains_key("Delete Cloud")
            || self.actions.contains_key("Select All Sources")
            || self.actions.contains_key("Create Password")
            || self.actions.contains_key("Set Password")
            || self.actions.contains_key("Reload Clouds Config")
            || self.actions.get("Toggle Selection").is_some_and(|action| {
                action
                    .keys
                    .iter()
                    .any(|key| key == "<leader>" || key == " ")
            })
            || self
                .actions
                .get("Navigate to Top")
                .is_some_and(|action| action.keys.iter().any(|key| key == "g"))
            || self.actions.values().any(|action| {
                action
                    .tab
                    .split(',')
                    .any(|tab| matches!(tab.trim(), "clouds" | "folders"))
            })
    }

    /// Migrate config to latest version by adding missing actions
    fn migrate_to_latest(&mut self) {
        for action in self.actions.values_mut() {
            action.tab = action
                .tab
                .split(',')
                .map(|tab| match tab.trim() {
                    "clouds" => "dashboard",
                    "folders" => "storage",
                    other => other,
                })
                .collect::<Vec<_>>()
                .join(",");
        }

        self.rename_action("Reload Clouds Config", "Reload Storage Config");

        let create_cloud = ["Create Cloud", "Add Source / Cloud", "Create New"]
            .into_iter()
            .find_map(|name| self.actions.remove(name))
            .unwrap_or(Action {
                keys: vec!["n".to_string()],
                tab: "storage".to_string(),
            });
        self.actions.remove("Add Source / Cloud");
        self.actions.remove("Create New");
        self.actions.insert(
            "Create Cloud".to_string(),
            Action {
                keys: create_cloud.keys,
                tab: "storage".to_string(),
            },
        );
        self.actions
            .entry("Add Folder".to_string())
            .or_insert(Action {
                keys: vec!["a".to_string()],
                tab: "storage".to_string(),
            });

        let remove_item = [
            "Remove Folder / Cloud",
            "Remove Item",
            "Remove Source",
            "Delete Folder",
        ]
        .into_iter()
        .find_map(|name| self.actions.remove(name));
        self.actions.remove("Remove Source");
        self.actions.remove("Delete Folder");
        self.actions.remove("Remove Cloud");
        self.actions.remove("Delete Cloud");
        let mut remove_keys = remove_item
            .map(|action| action.keys)
            .unwrap_or_else(|| vec!["d".to_string(), "x".to_string()]);
        for key in &mut remove_keys {
            if key.chars().count() == 1 {
                *key = key.to_lowercase();
            }
        }
        remove_keys.sort();
        remove_keys.dedup();
        self.actions.insert(
            "Remove Folder / Cloud".to_string(),
            Action {
                keys: remove_keys,
                tab: "storage".to_string(),
            },
        );

        let select_all = ["Select All Folders", "Select All Sources"]
            .into_iter()
            .find_map(|name| self.actions.remove(name));
        self.actions.remove("Select All Sources");
        let mut select_all_keys = select_all
            .map(|action| action.keys)
            .unwrap_or_else(|| vec!["A".to_string()]);
        for key in &mut select_all_keys {
            if key == "a" {
                *key = "A".to_string();
            }
        }
        self.actions.insert(
            "Select All Folders".to_string(),
            Action {
                keys: select_all_keys,
                tab: "storage".to_string(),
            },
        );

        if let Some(action) = self.actions.get_mut("Toggle Selection") {
            action.keys.retain(|key| key != "<leader>" && key != " ");
            if !action.keys.iter().any(|key| key == "v") {
                action.keys.push("v".to_string());
            }
        }
        if let Some(action) = self.actions.get_mut("Navigate to Top") {
            action.keys.retain(|key| key != "g");
            if !action.keys.iter().any(|key| key == "gg") {
                action.keys.push("gg".to_string());
            }
        }

        let mut password_keys = Vec::new();
        for old_name in ["Create Password", "Set Password", "Set Cloud Password"] {
            if let Some(action) = self.actions.remove(old_name) {
                for key in action.keys {
                    if !password_keys.contains(&key) {
                        password_keys.push(key);
                    }
                }
            }
        }
        if password_keys.is_empty() {
            password_keys.push("p".to_string());
        }
        self.actions.insert(
            "Set Cloud Password".to_string(),
            Action {
                keys: password_keys,
                tab: "dashboard,storage".to_string(),
            },
        );

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

        if !self.actions.contains_key("Toggle Help") {
            self.actions.insert(
                "Toggle Help".to_string(),
                Action {
                    keys: vec!["?".to_string()],
                    tab: "any".to_string(),
                },
            );
        }

        if !self.actions.contains_key("Open Cloud") {
            self.actions.insert(
                "Open Cloud".to_string(),
                Action {
                    keys: vec!["o".to_string()],
                    tab: "dashboard".to_string(),
                },
            );
        }

        for (name, action) in Self::default().actions {
            self.actions.entry(name).or_insert(action);
        }
    }

    fn rename_action(&mut self, old_name: &str, new_name: &str) {
        if self.actions.contains_key(new_name) {
            self.actions.remove(old_name);
        } else if let Some(action) = self.actions.remove(old_name) {
            self.actions.insert(new_name.to_string(), action);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_bindings_are_unique_within_each_tab() {
        let config = Config::default();

        for tab in ["dashboard", "storage", "settings"] {
            let mut bindings = HashMap::new();
            for (name, action) in &config.actions {
                if !action.applies_to(tab) {
                    continue;
                }
                for key in &action.keys {
                    assert_eq!(
                        bindings.insert(key, name),
                        None,
                        "{key} is assigned more than once in {tab}"
                    );
                }
            }
        }
    }

    #[test]
    fn defaults_include_discoverability_and_cloud_opening() {
        let config = Config::default();

        assert_eq!(config.get_keys_for_action("Toggle Help"), vec!["?"]);
        assert_eq!(config.get_keys_for_action("Open Cloud"), vec!["o"]);
        assert_eq!(config.get_keys_for_action("Set Cloud Password"), vec!["p"]);
        assert_eq!(config.get_keys_for_action("Add Folder"), vec!["a"]);
        assert_eq!(config.get_keys_for_action("Create Cloud"), vec!["n"]);
        assert_eq!(
            config.get_keys_for_action("Remove Folder / Cloud"),
            vec!["d", "x"]
        );
        assert_eq!(config.get_keys_for_action("Select All Folders"), vec!["A"]);
        assert!(config
            .get_keys_for_action("Toggle Selection")
            .contains(&"v".to_string()));
    }

    #[test]
    fn password_binding_applies_to_dashboard_and_storage() {
        let config = Config::default();

        assert_eq!(
            config.get_action_for_key("p", "dashboard").as_deref(),
            Some("Set Cloud Password")
        );
        assert_eq!(
            config.get_action_for_key("p", "storage").as_deref(),
            Some("Set Cloud Password")
        );
        assert_eq!(config.get_action_for_key("p", "settings"), None);
    }

    #[test]
    fn migration_preserves_keys_and_updates_domain_names() {
        let mut config = Config {
            leader: ",".to_string(),
            actions: HashMap::from([
                (
                    "Create New".to_string(),
                    Action {
                        keys: vec!["c".to_string()],
                        tab: "folders".to_string(),
                    },
                ),
                (
                    "Create Password".to_string(),
                    Action {
                        keys: vec!["P".to_string()],
                        tab: "clouds".to_string(),
                    },
                ),
                (
                    "Reload Clouds Config".to_string(),
                    Action {
                        keys: vec!["R".to_string()],
                        tab: "settings".to_string(),
                    },
                ),
                (
                    "Delete Folder".to_string(),
                    Action {
                        keys: vec!["d".to_string(), "x".to_string()],
                        tab: "folders".to_string(),
                    },
                ),
                (
                    "Delete Cloud".to_string(),
                    Action {
                        keys: vec!["D".to_string(), "X".to_string()],
                        tab: "folders".to_string(),
                    },
                ),
                (
                    "Select All Folders".to_string(),
                    Action {
                        keys: vec!["a".to_string()],
                        tab: "folders".to_string(),
                    },
                ),
            ]),
        };

        assert!(config.needs_migration());
        config.migrate_to_latest();

        assert_eq!(config.leader, ",");
        assert_eq!(config.get_keys_for_action("Add Folder"), vec!["a"]);
        assert_eq!(config.get_keys_for_action("Create Cloud"), vec!["c"]);
        assert_eq!(
            config.get_keys_for_action("Remove Folder / Cloud"),
            vec!["d", "x"]
        );
        assert_eq!(config.get_keys_for_action("Select All Folders"), vec!["A"]);
        assert_eq!(config.get_keys_for_action("Set Cloud Password"), vec!["P"]);
        assert_eq!(
            config.get_keys_for_action("Reload Storage Config"),
            vec!["R"]
        );
        assert_eq!(
            config
                .get_action("Create Cloud")
                .map(|action| action.tab.as_str()),
            Some("storage")
        );
        assert_eq!(
            config
                .get_action("Set Cloud Password")
                .map(|action| action.tab.as_str()),
            Some("dashboard,storage")
        );
    }
}
