use crate::tabs::focus::TabFocus;
use cloud_server::{CloudFolder as ServerCloudFolder, CloudServer, ServerConfig};
use ratatui::crossterm::event::KeyCode;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudFolder {
    pub name: String,
    pub folder_path: PathBuf,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CloudFoldersConfig {
    pub cloudfolders: Vec<CloudFolder>,
}

pub struct ServerState {
    pub cloudfolders: Vec<CloudFolder>,
    pub selected_cloudfolder_index: usize,
    pub creating_cloudfolder: bool,
    pub new_cloudfolder_name: String,
    pub new_cloudfolder_path: String,
    pub cloudfolder_input_field: CloudFolderInputField, // Which field is currently being edited
    pub cloudfolder_creation_error: Option<String>,
    pub server_logs: Vec<String>,
    pub log_scroll_offset: usize,    // For scrolling through logs
    pub focused_panel: FocusedPanel, // Which panel is currently focused
    pub server: Option<CloudServer>,
    pub server_port: Option<u16>,
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
            server_logs: Vec::new(),
            log_scroll_offset: 0,
            focused_panel: FocusedPanel::CloudFolders,
            server: None,
            server_port: None,
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
        state
    }

    pub fn start_creating_cloudfolder(&mut self) {
        self.creating_cloudfolder = true;
        self.new_cloudfolder_name = String::new();
        self.new_cloudfolder_path = String::new();
        self.cloudfolder_input_field = CloudFolderInputField::Name;
        self.cloudfolder_creation_error = None;
    }

    pub fn cancel_creating_cloudfolder(&mut self) {
        self.creating_cloudfolder = false;
        self.new_cloudfolder_name = String::new();
        self.new_cloudfolder_path = String::new();
        self.cloudfolder_input_field = CloudFolderInputField::Name;
        self.cloudfolder_creation_error = None;
    }

    pub fn create_profile(&mut self, _config: &crate::config::Config) {
        let name = self.new_cloudfolder_name.trim();
        let folder_path = self.new_cloudfolder_path.trim();

        if name.is_empty() {
            self.cloudfolder_creation_error = Some("Profile name cannot be empty".to_string());
            return;
        }

        if folder_path.is_empty() {
            self.cloudfolder_creation_error = Some("Profile path cannot be empty".to_string());
            return;
        }

        // Check if profile already exists
        if self.profile_exists(name) {
            self.cloudfolder_creation_error = Some(format!("Profile '{}' already exists", name));
            return;
        }

        // Expand path if it starts with ~
        let expanded_path = self.expand_path(folder_path);
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

        let cloudfolder = CloudFolder {
            name: name.to_string(),
            folder_path: path_buf,
        };

        self.cloudfolders.push(cloudfolder);
        self.save_cloudfolders_to_toml();

        self.creating_cloudfolder = false;
        self.new_cloudfolder_name = String::new();
        self.new_cloudfolder_path = String::new();
        self.cloudfolder_input_field = CloudFolderInputField::Name;
        self.cloudfolder_creation_error = None;
    }

    pub fn start_server(&mut self) {
        // Check if server is already running
        if self.server.is_some() {
            self.add_server_log("⚠️ Server is already running");
            return;
        }

        // Verify all cloudfolders have valid paths
        for profile in &self.cloudfolders {
            if !profile.folder_path.exists() {
                self.add_server_log(&format!(
                    "❌ Profile folder does not exist: {}",
                    profile.folder_path.display()
                ));
                return;
            }

            if !profile.folder_path.is_dir() {
                self.add_server_log(&format!(
                    "❌ Profile path is not a directory: {}",
                    profile.folder_path.display()
                ));
                return;
            }
        }

        let port = 3000; // Fixed port for single server
        let mut server = CloudServer::new();

        // Add all cloudfolders to the server
        for cloudfolder in &self.cloudfolders {
            let server_cloudfolder =
                ServerCloudFolder::new(cloudfolder.name.clone(), cloudfolder.folder_path.clone());
            server.add_cloudfolder(server_cloudfolder);
        }

        match server.start_server(port) {
            Ok(_) => {
                self.server = Some(server);
                self.server_port = Some(port);

                self.add_server_log("🚀 Starting CloudTUI server");
                self.add_server_log(&format!("🌐 Server started on http://127.0.0.1:{}", port));
                self.add_server_log(&format!(
                    "📊 Serving {} cloudfolders",
                    self.cloudfolders.len()
                ));

                let log_messages: Vec<String> = self
                    .cloudfolders
                    .iter()
                    .map(|cloudfolder| {
                        format!(
                            "📁 Cloud Folder '{}': http://127.0.0.1:{}/{}",
                            cloudfolder.name, port, cloudfolder.name
                        )
                    })
                    .collect();

                for message in log_messages {
                    self.add_server_log(&message);
                }

                self.add_server_log(&format!(
                    "🔗 Server status: http://127.0.0.1:{}/api/status",
                    port
                ));
            }
            Err(e) => {
                self.add_server_log(&format!("❌ Failed to start server: {}", e));
            }
        }
    }

    pub fn stop_server(&mut self) {
        if let Some(mut server) = self.server.take() {
            let port = self.server_port.take();
            server.stop_server();

            if let Some(port) = port {
                self.add_server_log(&format!("🛑 Stopped server on port {}", port));
            } else {
                self.add_server_log("🛑 Stopped server");
            }
            self.add_server_log("📴 All connections closed");
        } else {
            self.add_server_log("⚠️ No server running");
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
        self.load_cloudfolders_from_toml();
    }

    pub fn load_cloudfolders_with_config(&mut self, config: &ServerConfig) {
        self.load_cloudfolders_from_toml_with_config(config);
    }

    pub fn get_cloudfolders_toml_path() -> PathBuf {
        let mut path = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("CloudTUI");
        path.push("cloudfolders.toml");
        path
    }

    pub fn load_cloudfolders_from_toml(&mut self) {
        let cloudfolders_toml_path = Self::get_cloudfolders_toml_path();

        if let Ok(content) = std::fs::read_to_string(&cloudfolders_toml_path) {
            if let Ok(cloudfolders_config) = toml::from_str::<CloudFoldersConfig>(&content) {
                self.cloudfolders = cloudfolders_config.cloudfolders;
                return;
            }
        }

        // Fallback to old method if cloudfolders.toml doesn't exist or is invalid
        let config = crate::config::Config::load_or_default();
        let cloudfolders_path = self.expand_path(&config.server_config_path);

        if let Ok(entries) = std::fs::read_dir(&cloudfolders_path) {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    if entry.path().is_dir() {
                        let cloudfolder = CloudFolder {
                            name: name.to_string(),
                            folder_path: entry.path(),
                        };
                        self.cloudfolders.push(cloudfolder);
                    }
                }
            }
        }
    }

    pub fn load_cloudfolders_from_toml_with_config(&mut self, server_config: &ServerConfig) {
        let cloudfolders_toml_path = Self::get_cloudfolders_toml_path();

        if let Ok(content) = std::fs::read_to_string(&cloudfolders_toml_path) {
            if let Ok(cloudfolders_config) = toml::from_str::<CloudFoldersConfig>(&content) {
                self.cloudfolders = cloudfolders_config.cloudfolders;
                return;
            }
        }

        // Fallback to loading from server config path
        let cloudfolders_path = server_config.expand_path();

        if let Ok(entries) = std::fs::read_dir(&cloudfolders_path) {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    if entry.path().is_dir() {
                        let cloudfolder = CloudFolder {
                            name: name.to_string(),
                            folder_path: entry.path(),
                        };
                        self.cloudfolders.push(cloudfolder);
                    }
                }
            }
        }
    }

    pub fn save_cloudfolders_to_toml(&self) {
        let cloudfolders_config = CloudFoldersConfig {
            cloudfolders: self.cloudfolders.clone(),
        };

        if let Ok(content) = toml::to_string_pretty(&cloudfolders_config) {
            let cloudfolders_toml_path = Self::get_cloudfolders_toml_path();

            // Create directory if it doesn't exist
            if let Some(parent) = cloudfolders_toml_path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }

            let _ = std::fs::write(&cloudfolders_toml_path, content);
        }
    }

    pub fn expand_path(&self, path: &str) -> String {
        if path.starts_with("~/") {
            if let Some(home) = dirs::home_dir() {
                let mut expanded_path = home;
                expanded_path.push(&path[2..]);
                return expanded_path.to_string_lossy().to_string();
            }
        }
        path.to_string()
    }

    pub fn profile_exists(&self, name: &str) -> bool {
        self.cloudfolders.iter().any(|p| p.name == name)
    }

    pub fn delete_selected_profile(&mut self) {
        if !self.cloudfolders.is_empty()
            && self.selected_cloudfolder_index < self.cloudfolders.len()
        {
            let _profile_name = self.cloudfolders[self.selected_cloudfolder_index]
                .name
                .clone();

            // Remove from cloudfolders list (no need to delete folder since we're not creating it anymore)
            self.cloudfolders.remove(self.selected_cloudfolder_index);
            self.save_cloudfolders_to_toml();

            // Adjust selected index if needed
            if self.selected_cloudfolder_index >= self.cloudfolders.len()
                && !self.cloudfolders.is_empty()
            {
                self.selected_cloudfolder_index = self.cloudfolders.len() - 1;
            }
        }
    }

    pub fn navigate_profile_up(&mut self) {
        if self.selected_cloudfolder_index > 0 {
            self.selected_cloudfolder_index -= 1;
        }
    }

    pub fn navigate_profile_down(&mut self) {
        if self.selected_cloudfolder_index < self.cloudfolders.len().saturating_sub(1) {
            self.selected_cloudfolder_index += 1;
        }
    }

    pub fn is_server_running(&self) -> bool {
        self.server.is_some()
    }

    pub fn get_server_port(&self) -> Option<u16> {
        self.server_port
    }

    pub fn get_running_servers_count(&self) -> usize {
        if self.server.is_some() {
            1
        } else {
            0
        }
    }

    pub fn scroll_logs_up(&mut self) {
        if self.log_scroll_offset > 0 {
            self.log_scroll_offset -= 1;
        }
    }

    pub fn scroll_logs_down(&mut self) {
        // Don't scroll past the point where newest logs are at the bottom
        // This will be limited by the UI based on available height
        self.log_scroll_offset += 1;
    }

    pub fn scroll_logs_to_bottom(&mut self) {
        self.log_scroll_offset = 0; // Show newest logs at bottom
    }

    /// Check if the user is currently viewing the bottom (newest logs)
    pub fn is_at_bottom(&self) -> bool {
        self.log_scroll_offset == 0
    }
}

impl TabFocus for ServerState {
    fn get_focused_element(&self) -> String {
        match self.focused_panel {
            FocusedPanel::CloudFolders => "Profiles".to_string(),
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
                    FocusedPanel::CloudFolders => self.navigate_profile_down(),
                    FocusedPanel::ServerInfo => self.cycle_focus_forward(),
                    FocusedPanel::ServerLogs => self.scroll_logs_down(),
                }
                true
            }
            KeyCode::Char('k') => {
                match self.focused_panel {
                    FocusedPanel::CloudFolders => self.navigate_profile_up(),
                    FocusedPanel::ServerInfo => self.cycle_focus_forward(),
                    FocusedPanel::ServerLogs => self.scroll_logs_up(),
                }
                true
            }
            KeyCode::Char('g') => {
                // Handle gg sequence (go to top)
                match self.focused_panel {
                    FocusedPanel::CloudFolders => {
                        self.selected_cloudfolder_index = 0;
                    }
                    FocusedPanel::ServerInfo => self.cycle_focus_forward(),
                    FocusedPanel::ServerLogs => self.scroll_logs_to_top(),
                }
                true
            }
            KeyCode::Char('G') => {
                match self.focused_panel {
                    FocusedPanel::CloudFolders => {
                        self.selected_cloudfolder_index = self.cloudfolders.len().saturating_sub(1);
                    }
                    FocusedPanel::ServerInfo => self.cycle_focus_forward(),
                    FocusedPanel::ServerLogs => self.scroll_logs_to_bottom(),
                }
                true
            }
            _ => false,
        }
    }

    fn has_focusable_elements(&self) -> bool {
        true
    }

    fn focusable_elements_count(&self) -> usize {
        3 // Profiles, ServerInfo, ServerLogs
    }
}

impl ServerState {
    pub fn scroll_logs_to_top(&mut self) {
        self.log_scroll_offset = self.server_logs.len().saturating_sub(1); // Show oldest logs at top
    }

    pub fn handle_profile_input(&mut self, key: ratatui::crossterm::event::KeyCode) {
        use ratatui::crossterm::event::KeyCode;

        match key {
            KeyCode::Enter => {
                if self.cloudfolder_input_field == CloudFolderInputField::Name {
                    // Move to path field if name is not empty
                    if !self.new_cloudfolder_name.trim().is_empty() {
                        self.cloudfolder_input_field = CloudFolderInputField::Path;
                    }
                } else {
                    // Try to create profile when in path field
                    self.create_profile(&crate::config::Config::load_or_default());
                }
            }
            KeyCode::Esc => {
                self.cancel_creating_cloudfolder();
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
}
