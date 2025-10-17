/// Password creation logic and state management
/// This module contains all the business logic for password creation,
/// separate from the UI components.
/// Password creation modes
#[derive(Debug, Clone, PartialEq, Default)]
pub enum PasswordMode {
    #[default]
    Creating,
    Confirming,
}

/// Password creation state that can be used across different parts of the application
#[derive(Default)]
pub struct PasswordCreationState {
    pub creating_password: bool,
    pub password_input: String,
    pub password_confirm: String,
    pub password_mode: PasswordMode,
    pub password_error: Option<String>,
    pub password_success: bool,
}

impl PasswordCreationState {
    pub fn new() -> Self {
        Self {
            creating_password: false,
            password_input: String::new(),
            password_confirm: String::new(),
            password_mode: PasswordMode::default(),
            password_error: None,
            password_success: false,
        }
    }

    /// Start the password creation process
    pub fn start_creating_password(&mut self) {
        self.creating_password = true;
        self.password_input.clear();
        self.password_confirm.clear();
        self.password_mode = PasswordMode::Creating;
        self.password_error = None;
        self.password_success = false;
    }

    /// Clear the password creation state
    pub fn clear_password_creation(&mut self) {
        self.creating_password = false;
        self.password_input.clear();
        self.password_confirm.clear();
        self.password_mode = PasswordMode::Creating;
        self.password_error = None;
        self.password_success = false;
    }

    /// Handle password input from user
    pub fn handle_password_input(&mut self, key: char) {
        match key {
            '\n' | '\r' => {
                match self.password_mode {
                    PasswordMode::Creating => {
                        if self.password_input.len() < 8 {
                            self.password_error =
                                Some("Password must be at least 8 characters".to_string());
                            return;
                        }
                        self.password_mode = PasswordMode::Confirming;
                        self.password_confirm.clear();
                    }
                    PasswordMode::Confirming => {
                        if self.password_input == self.password_confirm {
                            // Password confirmed, will be set by the caller
                        } else {
                            self.password_error = Some("Passwords do not match".to_string());
                            self.password_confirm.clear();
                        }
                    }
                }
            }
            '\x08' | '\x7f' => {
                // Backspace
                match self.password_mode {
                    PasswordMode::Creating => {
                        self.password_input.pop();
                    }
                    PasswordMode::Confirming => {
                        self.password_confirm.pop();
                    }
                }
                self.password_error = None;
            }
            '\x1b' => {
                // Escape - cancel password creation
                self.clear_password_creation();
            }
            c if c.is_ascii_graphic() || c == ' ' => {
                // Add character
                match self.password_mode {
                    PasswordMode::Creating => {
                        if self.password_input.len() < 50 {
                            self.password_input.push(c);
                        }
                    }
                    PasswordMode::Confirming => {
                        if self.password_confirm.len() < 50 {
                            self.password_confirm.push(c);
                        }
                    }
                }
                self.password_error = None;
            }
            _ => {}
        }
    }

    /// Check if password creation is complete and valid
    pub fn is_password_creation_complete(&self) -> bool {
        self.password_mode == PasswordMode::Confirming
            && self.password_input == self.password_confirm
            && self.password_input.len() >= 8
    }

    /// Get the current password being created
    pub fn get_password(&self) -> &str {
        &self.password_input
    }

    /// Check if we're currently in password creation mode
    pub fn is_creating_password(&self) -> bool {
        self.creating_password
    }

    /// Get the current password mode
    pub fn get_password_mode(&self) -> &PasswordMode {
        &self.password_mode
    }

    /// Get the current error message, if any
    pub fn get_error(&self) -> Option<&String> {
        self.password_error.as_ref()
    }

    /// Get the masked password for display
    pub fn get_masked_password(&self) -> String {
        "*".repeat(self.password_input.len())
    }

    /// Get the masked confirmation password for display
    pub fn get_masked_confirm(&self) -> String {
        "*".repeat(self.password_confirm.len())
    }
}
