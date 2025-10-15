use crate::tabs::focus::TabFocus;
use cloudhost_server::{CloudFolder as ServerCloudFolder, CloudServer, ServerConfig};
use ratatui::crossterm::event::KeyCode;
use ratatui::widgets::{ListState, ScrollbarState};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudFolder {
    pub name: String,
    pub folder_path: PathBuf,
}

pub struct ServerState {
    pub cloudfolders: Vec<CloudFolder>,
    pub selected_cloudfolder_index: usize,
    pub creating_cloudfolder: bool,
    pub new_cloudfolder_name: String,
    pub new_cloudfolder_path: String,
    pub cloudfolder_input_field: CloudFolderInputField, // Which field is currently being edited
    pub cloudfolder_creation_error: Option<String>,
    pub server_start_error: Option<String>,
    pub server_logs: Vec<String>,
    pub log_scroll_offset: usize,    // For scrolling through logs
    pub focused_panel: FocusedPanel, // Which panel is currently focused
    pub server: Option<CloudServer>,
    pub server_port: Option<u16>,
    pub server_logs_list_state: ListState, // For scrollable server logs
    pub server_logs_scroll_state: ScrollbarState, // For scrollbar
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusedPanel {
    CloudFolders,
    ServerInfo,
    ServerLogs,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloudFolderInputField {
    Name,
    Path,
}

impl Default for ServerState {
    fn default() -> Self {
        Self {
            cloudfolders: Vec::new(),
            selected_cloudfolder_index: 0,
            creating_cloudfolder: false,
            new_cloudfolder_name: String::new(),
            new_cloudfolder_path: String::new(),
            cloudfolder_input_field: CloudFolderInputField::Name,
            cloudfolder_creation_error: None,
            server_start_error: None,
            server_logs: Vec::new(),
            log_scroll_offset: 0,
            focused_panel: FocusedPanel::CloudFolders,
            server: None,
            server_port: None,
            server_logs_list_state: ListState::default(),
            server_logs_scroll_state: ScrollbarState::default(),
        }
    }
}

impl ServerState {
    pub fn new() -> Self {
        let mut state = Self::default();
        state.load_cloudfolders();
        state
    }

    pub fn new_with_config(config: &ServerConfig) -> Self {
        let mut state = Self::default();
        state.load_cloudfolders_with_config(config);
        state.server = Some(CloudServer::new());
        state
    }
    pub fn start_creating_cloudfolder(&mut self) {
        self.creating_cloudfolder = true;
    }

    pub fn clear_cloudfolder_creation_input(&mut self) {
        self.creating_cloudfolder = false;
        self.new_cloudfolder_name = String::new();
        self.new_cloudfolder_path = String::new();
        self.cloudfolder_input_field = CloudFolderInputField::Name;
        self.cloudfolder_creation_error = None;
    }

    pub fn create_cloudfolder(&mut self, _config: &crate::config::Config) {
        let name = self.new_cloudfolder_name.trim();
        let folder_path = self.new_cloudfolder_path.trim();

        if name.is_empty() {
            self.cloudfolder_creation_error = Some("Cloudfolder name cannot be empty".to_string());
            return;
        }

        if folder_path.is_empty() {
            self.cloudfolder_creation_error = Some("Cloudfolder path cannot be empty".to_string());
            return;
        }

        // Check if cloudfolder already exists
        if self.cloudfolder_exists(name) {
            self.cloudfolder_creation_error =
                Some(format!("Cloudfolder '{}' already exists", name));
            return;
        }

        // Expand path if it starts with ~
        let expanded_path = if let Some(stripped) = folder_path.strip_prefix("~/") {
            if let Some(home) = dirs::home_dir() {
                home.join(stripped).to_string_lossy().to_string()
            } else {
                folder_path.to_string()
            }
        } else {
            folder_path.to_string()
        };
        let path_buf = PathBuf::from(&expanded_path);

        // Check if the folder exists
        if !path_buf.exists() {
            self.cloudfolder_creation_error =
                Some(format!("Folder '{}' does not exist", expanded_path));
            return;
        }

        if !path_buf.is_dir() {
            self.cloudfolder_creation_error =
                Some(format!("'{}' is not a directory", expanded_path));
            return;
        }

        // Additional validation: check if path is readable
        if let Err(e) = std::fs::read_dir(&path_buf) {
            self.cloudfolder_creation_error =
                Some(format!("Cannot read directory '{}': {}", expanded_path, e));
            return;
        }

        // Create cloudfolder using server's CloudFolder struct
        let server_cloudfolder = ServerCloudFolder::new(name.to_string(), path_buf);

        // Add to server (this will also save to config)
        if let Some(ref mut server) = self.server {
            if let Err(e) = server.add_cloudfolder(server_cloudfolder) {
                self.cloudfolder_creation_error = Some(format!("Failed to add cloudfolder: {}", e));
                return;
            }
        }

        // Reload cloudfolders from server config
        if let Some(ref server) = self.server {
            self.cloudfolders = server
                .get_cloudfolders()
                .iter()
                .map(|cf| CloudFolder {
                    name: cf.name.clone(),
                    folder_path: cf.folder_path.clone(),
                })
                .collect();
        }

        self.clear_cloudfolder_creation_input();
    }

    pub fn start_server(&mut self) {
        // Check if server instance exists
        if self.server.is_none() {
            self.server = Some(CloudServer::new());
        }

        // Check if server is already running
        if let Some(ref server) = self.server {
            if server.is_running() {
                return;
            }
        }

        // Check if password is set
        if let Some(ref server) = self.server {
            if !server.has_password() {
                self.server_start_error = Some("❌ Cannot start server: No password set. Go to Settings tab and press 'p' to create a password.".to_string());
                return;
            }
        }

        // Verify all cloudfolders have valid paths
        for cloudfolder in &self.cloudfolders {
            if !cloudfolder.folder_path.exists() {
                return;
            }

            if !cloudfolder.folder_path.is_dir() {
                return;
            }
        }

        let port = 3000; // Fixed port for single server

        if let Some(ref mut server) = self.server {
            // Add all cloudfolders to the server
            for cloudfolder in &self.cloudfolders {
                let server_cloudfolder = ServerCloudFolder::new(
                    cloudfolder.name.clone(),
                    cloudfolder.folder_path.clone(),
                );
                if let Err(e) = server.add_cloudfolder(server_cloudfolder) {
                    eprintln!("Failed to add cloudfolder to server: {}", e);
                }
            }

            match server.start_server(port) {
                Ok(_) => {
                    // Clear any previous server start errors
                    self.server_start_error = None;
                }

                Err(e) => {
                    // Set error message for display
                    self.server_start_error = Some(format!("❌ Failed to start server: {}", e));
                }
            }
        }
    }

    pub async fn stop_server(&mut self) {
        if let Some(ref mut server) = self.server {
            let _port = self.server_port.take();
            server.stop_server().await;
            // Server will handle its own logging
        }
    }

    pub fn add_server_log(&mut self, message: &str) {
        // Check if user is currently at the bottom (scroll offset is 0)
        let was_at_bottom = self.log_scroll_offset == 0;

        self.server_logs.push(format!(
            "[{}] {}",
            chrono::Local::now().format("%H:%M:%S%.3f"),
            message
        ));

        // Keep only last 20 server log messages
        if self.server_logs.len() > 20 {
            self.server_logs.remove(0);
        }

        // If user was at the bottom, keep them at the bottom
        // If user was scrolled up, don't auto-scroll (keep their position)
        if was_at_bottom {
            self.log_scroll_offset = 0; // Stay at bottom
        }
        // If not at bottom, don't change scroll offset (user stays where they are)
    }

    pub fn load_cloudfolders(&mut self) {
        // Load cloudfolders from server config
        let server_config = cloudhost_server::ServerConfig::load_from_file().unwrap_or_default();
        self.load_cloudfolders_with_config(&server_config);
    }

    pub fn load_cloudfolders_with_config(&mut self, config: &ServerConfig) {
        self.load_cloudfolders_from_toml_with_config(config);
    }

    pub fn load_cloudfolders_from_toml_with_config(&mut self, server_config: &ServerConfig) {
        // Load cloudfolders directly from server config
        self.cloudfolders = server_config
            .cloudfolders
            .iter()
            .map(|cf| CloudFolder {
                name: cf.name.clone(),
                folder_path: cf.folder_path.clone(),
            })
            .collect();
    }

    pub fn cloudfolder_exists(&self, name: &str) -> bool {
        self.cloudfolders.iter().any(|p| p.name == name)
    }

    pub fn delete_selected_cloudfolder(&mut self) {
        if !self.cloudfolders.is_empty()
            && self.selected_cloudfolder_index < self.cloudfolders.len()
        {
            let cloudfolder_name = self.cloudfolders[self.selected_cloudfolder_index]
                .name
                .clone();

            // Remove from server (this will also save to config)
            if let Some(ref mut server) = self.server {
                if let Err(e) = server.remove_cloudfolder(&cloudfolder_name) {
                    // Handle error - could add error display to UI
                    eprintln!("Failed to remove cloudfolder: {}", e);
                    return;
                }
            }

            // Reload cloudfolders from server config
            if let Some(ref server) = self.server {
                self.cloudfolders = server
                    .get_cloudfolders()
                    .iter()
                    .map(|cf| CloudFolder {
                        name: cf.name.clone(),
                        folder_path: cf.folder_path.clone(),
                    })
                    .collect();
            }

            // Adjust selected index if needed
            if self.selected_cloudfolder_index >= self.cloudfolders.len()
                && !self.cloudfolders.is_empty()
            {
                self.selected_cloudfolder_index = self.cloudfolders.len() - 1;
            }
        }
    }

    pub fn navigate_cloudfolder_up(&mut self) {
        if self.selected_cloudfolder_index > 0 {
            self.selected_cloudfolder_index -= 1;
        }
    }

    pub fn navigate_cloudfolder_down(&mut self) {
        if self.selected_cloudfolder_index < self.cloudfolders.len().saturating_sub(1) {
            self.selected_cloudfolder_index += 1;
        }
    }

    pub fn is_server_running(&self) -> bool {
        self.server.as_ref().is_some_and(|s| s.is_running())
    }

    pub fn get_server_port(&self) -> Option<u16> {
        self.server_port
    }

    pub fn scroll_logs_up(&mut self) {
        if let Some(selected) = self.server_logs_list_state.selected() {
            if selected > 0 {
                let new_selected = selected - 1;
                self.server_logs_list_state.select(Some(new_selected));
                self.update_scrollbar_state();
            }
        }
    }

    pub fn scroll_logs_down(&mut self) {
        if let Some(selected) = self.server_logs_list_state.selected() {
            if selected < self.server_logs.len().saturating_sub(1) {
                let new_selected = selected + 1;
                self.server_logs_list_state.select(Some(new_selected));
                self.update_scrollbar_state();
            }
        } else if !self.server_logs.is_empty() {
            self.server_logs_list_state.select(Some(0));
            self.update_scrollbar_state();
        }
    }

    pub fn scroll_logs_to_bottom(&mut self) {
        if !self.server_logs.is_empty() {
            let last_index = self.server_logs.len().saturating_sub(1);
            self.server_logs_list_state.select(Some(last_index));
            self.update_scrollbar_state();
        }
    }

    fn update_scrollbar_state(&mut self) {
        if let Some(selected) = self.server_logs_list_state.selected() {
            self.server_logs_scroll_state = self
                .server_logs_scroll_state
                .content_length(self.server_logs.len())
                .position(selected);
        }
    }

    /// Check if the user is currently viewing the bottom (newest logs)
    pub fn is_at_bottom(&self) -> bool {
        if let Some(selected) = self.server_logs_list_state.selected() {
            selected == self.server_logs.len().saturating_sub(1)
        } else {
            true
        }
    }
}

impl TabFocus for ServerState {
    fn get_focused_element(&self) -> String {
        match self.focused_panel {
            FocusedPanel::CloudFolders => "Cloudfolders".to_string(),
            FocusedPanel::ServerInfo => "ServerInfo".to_string(),
            FocusedPanel::ServerLogs => "ServerLogs".to_string(),
        }
    }

    fn cycle_focus_forward(&mut self) {
        self.focused_panel = match self.focused_panel {
            FocusedPanel::CloudFolders => FocusedPanel::ServerInfo,
            FocusedPanel::ServerInfo => FocusedPanel::ServerLogs,
            FocusedPanel::ServerLogs => FocusedPanel::CloudFolders,
        };
    }

    fn cycle_focus_backward(&mut self) {
        self.focused_panel = match self.focused_panel {
            FocusedPanel::CloudFolders => FocusedPanel::ServerLogs,
            FocusedPanel::ServerInfo => FocusedPanel::CloudFolders,
            FocusedPanel::ServerLogs => FocusedPanel::ServerInfo,
        };
    }

    fn handle_navigation(&mut self, key: KeyCode) -> bool {
        match key {
            KeyCode::Char('j') => {
                match self.focused_panel {
                    FocusedPanel::CloudFolders => {
                        self.navigate_cloudfolder_down();
                        true
                    }
                    FocusedPanel::ServerInfo => false, // No navigation in ServerInfo panel
                    FocusedPanel::ServerLogs => {
                        self.scroll_logs_down();
                        true
                    }
                }
            }
            KeyCode::Char('k') => {
                match self.focused_panel {
                    FocusedPanel::CloudFolders => {
                        self.navigate_cloudfolder_up();
                        true
                    }
                    FocusedPanel::ServerInfo => false, // No navigation in ServerInfo panel
                    FocusedPanel::ServerLogs => {
                        self.scroll_logs_up();
                        true
                    }
                }
            }
            KeyCode::Char('g') => {
                // Handle gg sequence (go to top)
                match self.focused_panel {
                    FocusedPanel::CloudFolders => {
                        self.selected_cloudfolder_index = 0;
                        true
                    }
                    FocusedPanel::ServerInfo => false, // No navigation in ServerInfo panel
                    FocusedPanel::ServerLogs => {
                        self.scroll_logs_to_top();
                        true
                    }
                }
            }
            KeyCode::Char('G') => {
                match self.focused_panel {
                    FocusedPanel::CloudFolders => {
                        self.selected_cloudfolder_index = self.cloudfolders.len().saturating_sub(1);
                        true
                    }
                    FocusedPanel::ServerInfo => false, // No navigation in ServerInfo panel
                    FocusedPanel::ServerLogs => {
                        self.scroll_logs_to_bottom();
                        true
                    }
                }
            }
            _ => false,
        }
    }

    fn has_focusable_elements(&self) -> bool {
        true
    }

    fn focusable_elements_count(&self) -> usize {
        3 // Cloudfolders, ServerInfo, ServerLogs
    }
}

impl ServerState {
    pub fn scroll_logs_to_top(&mut self) {
        if !self.server_logs.is_empty() {
            self.server_logs_list_state.select(Some(0));
            self.update_scrollbar_state();
        }
    }

    pub fn handle_cloudfolder_input(&mut self, key: ratatui::crossterm::event::KeyCode) {
        use ratatui::crossterm::event::KeyCode;

        match key {
            KeyCode::Enter => {
                if self.cloudfolder_input_field == CloudFolderInputField::Name {
                    // Move to path field if name is not empty
                    if !self.new_cloudfolder_name.trim().is_empty() {
                        self.cloudfolder_input_field = CloudFolderInputField::Path;
                    }
                } else {
                    // Try to create cloudfolder when in path field
                    self.create_cloudfolder(&crate::config::Config::load_or_default());
                }
            }
            KeyCode::Esc => {
                self.clear_cloudfolder_creation_input();
            }
            KeyCode::Tab => {
                // Switch between name and path fields
                self.cloudfolder_input_field = match self.cloudfolder_input_field {
                    CloudFolderInputField::Name => CloudFolderInputField::Path,
                    CloudFolderInputField::Path => CloudFolderInputField::Name,
                };
            }
            KeyCode::Char(c) => {
                match self.cloudfolder_input_field {
                    CloudFolderInputField::Name => {
                        self.new_cloudfolder_name.push(c);
                    }
                    CloudFolderInputField::Path => {
                        self.new_cloudfolder_path.push(c);
                    }
                }
                self.cloudfolder_creation_error = None; // Clear error when typing
            }
            KeyCode::Backspace => {
                match self.cloudfolder_input_field {
                    CloudFolderInputField::Name => {
                        self.new_cloudfolder_name.pop();
                    }
                    CloudFolderInputField::Path => {
                        self.new_cloudfolder_path.pop();
                    }
                }
                self.cloudfolder_creation_error = None; // Clear error when editing
            }
            _ => {}
        }
    }

    /// Cycle focus forward through available panels
    pub fn cycle_focus_forward(&mut self) {
        self.focused_panel = match self.focused_panel {
            FocusedPanel::CloudFolders => FocusedPanel::ServerInfo,
            FocusedPanel::ServerInfo => FocusedPanel::ServerLogs,
            FocusedPanel::ServerLogs => FocusedPanel::CloudFolders,
        };
    }

    /// Cycle focus backward through available panels
    pub fn cycle_focus_backward(&mut self) {
        self.focused_panel = match self.focused_panel {
            FocusedPanel::CloudFolders => FocusedPanel::ServerLogs,
            FocusedPanel::ServerInfo => FocusedPanel::CloudFolders,
            FocusedPanel::ServerLogs => FocusedPanel::ServerInfo,
        };
    }

    /// Navigate to previous cloudfolder
    pub fn previous_cloudfolder(&mut self) {
        if !self.cloudfolders.is_empty() {
            self.selected_cloudfolder_index = if self.selected_cloudfolder_index == 0 {
                self.cloudfolders.len() - 1
            } else {
                self.selected_cloudfolder_index - 1
            };
        }
    }

    /// Navigate to next cloudfolder
    pub fn next_cloudfolder(&mut self) {
        if !self.cloudfolders.is_empty() {
            self.selected_cloudfolder_index =
                (self.selected_cloudfolder_index + 1) % self.cloudfolders.len();
        }
    }
}
