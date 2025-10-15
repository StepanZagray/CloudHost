use crate::tabs::focus::TabFocus;
use ratatui::crossterm::event::KeyCode;
use ratatui::widgets::{ListState, ScrollbarState};

#[derive(Default)]
pub struct SettingsState {
    pub password_input: String,
    pub password_confirm: String,
    pub password_mode: PasswordMode,
    pub password_error: Option<String>,
    pub password_success: bool,
    pub creating_password: bool, // Modal state like cloud folder creation
    pub selected_config_folder: Option<ConfigFolder>, // For keyboard navigation
    pub config_folders_list_state: ListState, // For config folders list
    pub config_folders_scroll_state: ScrollbarState, // For scrollbar
}

#[derive(Clone, PartialEq)]
pub enum ConfigFolder {
    ServerConfig,
    TuiConfig,
}

#[derive(Default, PartialEq)]
pub enum PasswordMode {
    #[default]
    Normal,
    Creating,
    Confirming,
}

impl TabFocus for SettingsState {
    fn get_focused_element(&self) -> String {
        if self.creating_password {
            "PasswordModal".to_string()
        } else if let Some(ref folder) = self.selected_config_folder {
            match folder {
                ConfigFolder::ServerConfig => "ServerConfigFolder".to_string(),
                ConfigFolder::TuiConfig => "TuiConfigFolder".to_string(),
            }
        } else {
            "SettingsMain".to_string()
        }
    }

    fn cycle_focus_forward(&mut self) {
        if !self.creating_password {
            let current_selection = self.config_folders_list_state.selected().unwrap_or(0);
            let next_selection = (current_selection + 1) % 2; // We have 2 config folders
            self.config_folders_list_state.select(Some(next_selection));
            self.selected_config_folder = match next_selection {
                0 => Some(ConfigFolder::ServerConfig),
                1 => Some(ConfigFolder::TuiConfig),
                _ => None,
            };
        }
    }

    fn cycle_focus_backward(&mut self) {
        if !self.creating_password {
            let current_selection = self.config_folders_list_state.selected().unwrap_or(0);
            let prev_selection = if current_selection == 0 {
                1
            } else {
                current_selection - 1
            };
            self.config_folders_list_state.select(Some(prev_selection));
            self.selected_config_folder = match prev_selection {
                0 => Some(ConfigFolder::ServerConfig),
                1 => Some(ConfigFolder::TuiConfig),
                _ => None,
            };
        }
    }

    fn handle_navigation(&mut self, key: KeyCode) -> bool {
        if self.creating_password {
            return self.handle_password_input(key);
        }

        match key {
            KeyCode::Enter => {
                if let Some(ref folder) = self.selected_config_folder {
                    self.open_config_folder(folder);
                    return true;
                }
            }
            KeyCode::Tab => {
                self.cycle_focus_forward();
                return true;
            }
            KeyCode::BackTab => {
                self.cycle_focus_backward();
                return true;
            }
            _ => {}
        }

        false
    }
}

impl SettingsState {
    pub fn start_creating_password(&mut self) {
        self.creating_password = true;
        self.password_mode = PasswordMode::Creating;
        self.password_input.clear();
        self.password_confirm.clear();
        self.password_error = None;
        self.password_success = false;
    }

    pub fn clear_password_creation_input(&mut self) {
        self.creating_password = false;
        self.password_mode = PasswordMode::Normal;
        self.password_input.clear();
        self.password_confirm.clear();
        self.password_error = None;
        self.password_success = false;
    }

    pub fn handle_password_input(&mut self, key: KeyCode) -> bool {
        match key {
            KeyCode::Esc => {
                self.clear_password_creation_input();
                return true;
            }
            KeyCode::Enter => {
                if self.password_mode == PasswordMode::Creating {
                    if self.password_input.len() < 8 {
                        self.password_error =
                            Some("Password must be at least 8 characters long".to_string());
                        return true;
                    }
                    self.password_mode = PasswordMode::Confirming;
                    self.password_confirm.clear();
                    return true;
                } else if self.password_mode == PasswordMode::Confirming {
                    if self.password_input == self.password_confirm {
                        // Password creation will be completed by the main app
                        return true;
                    } else {
                        self.password_error = Some("Passwords do not match".to_string());
                        return true;
                    }
                }
            }
            KeyCode::Backspace => {
                if self.password_mode == PasswordMode::Creating {
                    self.password_input.pop();
                    return true;
                } else if self.password_mode == PasswordMode::Confirming {
                    self.password_confirm.pop();
                    return true;
                }
            }
            KeyCode::Char(c) => {
                if self.password_mode == PasswordMode::Creating {
                    self.password_input.push(c);
                    return true;
                } else if self.password_mode == PasswordMode::Confirming {
                    self.password_confirm.push(c);
                    return true;
                }
            }
            _ => {}
        }

        false
    }

    pub fn open_config_folder(&self, folder: &ConfigFolder) {
        let config_path = match folder {
            ConfigFolder::ServerConfig => cloudhost_shared::config_paths::get_server_config_path(),
            ConfigFolder::TuiConfig => cloudhost_shared::config_paths::get_tui_config_path(),
        };

        // Get the parent directory (config folder)
        if let Some(parent_dir) = config_path.parent() {
            if let Err(e) = open::that(parent_dir) {
                log::error!("Failed to open config folder: {}", e);
            }
        }
    }
}
