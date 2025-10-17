use crate::tabs::focus::TabFocus;
use ratatui::crossterm::event::KeyCode;
use ratatui::widgets::{ListState, ScrollbarState};

#[derive(Default)]
pub struct SettingsState {
    pub list_state: ListState,
    pub scroll_state: ScrollbarState,
}

impl TabFocus for SettingsState {
    fn get_focused_element(&self) -> String {
        "SettingsList".to_string()
    }

    fn cycle_focus_forward(&mut self) {
        // Settings tab is just one scrollable column, no focus cycling needed
    }

    fn cycle_focus_backward(&mut self) {
        // Settings tab is just one scrollable column, no focus cycling needed
    }

    fn handle_navigation(&mut self, key: KeyCode) -> bool {
        match key {
            KeyCode::Up | KeyCode::Char('k') => {
                if let Some(selected) = self.list_state.selected() {
                    if selected > 0 {
                        self.list_state.select(Some(selected - 1));
                    }
                } else {
                    self.list_state.select(Some(0));
                }
                true
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if let Some(selected) = self.list_state.selected() {
                    self.list_state.select(Some(selected + 1));
                } else {
                    self.list_state.select(Some(0));
                }
                true
            }
            KeyCode::Enter => {
                // Handle opening config files/folders
                self.handle_enter();
                true
            }
            _ => false,
        }
    }
}

impl SettingsState {
    pub fn new() -> Self {
        Self {
            list_state: ListState::default(),
            scroll_state: ScrollbarState::default(),
        }
    }

    pub fn handle_enter(&self) {
        if let Some(selected) = self.list_state.selected() {
            match selected {
                0 => {
                    // Open TUI config file
                    let config_path = cloudhost_server::config_paths::get_tui_config_path();
                    if let Err(e) = open::that(&config_path) {
                        log::error!("Failed to open TUI config file: {}", e);
                    }
                }
                3 => {
                    // Open clouds config file
                    let config_path = cloudhost_server::config_paths::get_clouds_config_path();
                    if let Err(e) = open::that(&config_path) {
                        log::error!("Failed to open clouds config file: {}", e);
                    }
                }
                6 => {
                    // Reset TUI config to default
                    match crate::config::Config::reset_to_default() {
                        Ok(_) => {
                            log::info!("TUI config reset to default successfully");
                            // Note: The app will need to be restarted to see the changes
                        }
                        Err(e) => {
                            log::error!("Failed to reset TUI config to default: {}", e);
                        }
                    }
                }
                _ => {}
            }
        }
    }
}
