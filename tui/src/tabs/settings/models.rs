use crate::tabs::focus::TabFocus;
use ratatui::crossterm::event::KeyCode;

#[derive(Default)]
pub struct SettingsState {
    pub password_input: String,
    pub password_confirm: String,
    pub password_mode: PasswordMode,
    pub password_error: Option<String>,
    pub password_success: bool,
    pub creating_password: bool, // Modal state like cloud folder creation
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
        "SettingsMain".to_string()
    }

    fn cycle_focus_forward(&mut self) {
        // Settings tab has only one focusable element
    }

    fn cycle_focus_backward(&mut self) {
        // Settings tab has only one focusable element
    }

    fn handle_navigation(&mut self, _key: KeyCode) -> bool {
        // Settings navigation not implemented yet
        false
    }

    fn has_focusable_elements(&self) -> bool {
        true
    }

    fn focusable_elements_count(&self) -> usize {
        1 // Just the main settings area
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
        use ratatui::crossterm::event::KeyCode;

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
}
