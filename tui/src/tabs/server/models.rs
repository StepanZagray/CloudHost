use crate::tabs::focus::TabFocus;
use cloud_server::{CloudServer, Profile as ServerProfile};
use ratatui::crossterm::event::KeyCode;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Profile {
    pub name: String,
    pub folder_path: PathBuf,
}

pub struct ServerState {
    pub profiles: Vec<Profile>,
    pub selected_profile_index: usize,
    pub creating_profile: bool,
    pub new_profile_name: String,
    pub profile_creation_error: Option<String>,
    pub server_logs: Vec<String>,
    pub log_scroll_offset: usize,    // For scrolling through logs
    pub focused_panel: FocusedPanel, // Which panel is currently focused
    pub running_servers: std::collections::HashMap<String, (u16, CloudServer)>, // profile_name -> (port, server)
    pub next_port: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusedPanel {
    Profiles,
    ServerInfo,
    ServerLogs,
}

impl Default for ServerState {
    fn default() -> Self {
        Self {
            profiles: Vec::new(),
            selected_profile_index: 0,
            creating_profile: false,
            new_profile_name: String::new(),
            profile_creation_error: None,
            server_logs: Vec::new(),
            log_scroll_offset: 0,
            focused_panel: FocusedPanel::Profiles,
            running_servers: std::collections::HashMap::new(),
            next_port: 3000,
        }
    }
}

impl ServerState {
    pub fn new() -> Self {
        let mut state = Self::default();
        state.load_profiles();
        state
    }

    pub fn start_creating_profile(&mut self) {
        self.creating_profile = true;
        self.new_profile_name = String::new();
        self.profile_creation_error = None;
    }

    pub fn cancel_creating_profile(&mut self) {
        self.creating_profile = false;
        self.new_profile_name = String::new();
        self.profile_creation_error = None;
    }

    pub fn create_profile(&mut self, config: &crate::config::Config) {
        if !self.new_profile_name.is_empty() {
            let name = self.new_profile_name.trim();

            // Check if profile already exists
            if self.profile_exists(name) {
                self.profile_creation_error = Some(format!("Profile '{}' already exists", name));
                return;
            }

            // Create profile folder
            let profiles_path = self.expand_path(&config.profiles_path);
            let profile_folder = format!("{}/{}", profiles_path, name);

            if let Err(e) = std::fs::create_dir_all(&profile_folder) {
                self.profile_creation_error =
                    Some(format!("Failed to create profile folder: {}", e));
                return;
            }

            let profile = Profile {
                name: name.to_string(),
                folder_path: profile_folder.into(),
            };

            self.profiles.push(profile);

            self.creating_profile = false;
            self.new_profile_name = String::new();
            self.profile_creation_error = None;
        }
    }

    pub fn start_server(&mut self) {
        if let Some(profile) = self.profiles.get(self.selected_profile_index) {
            let profile_name = profile.name.clone();
            let profile_path = profile.folder_path.clone();

            // Check if server is already running for this profile
            if self.running_servers.contains_key(&profile_name) {
                self.add_server_log(&format!(
                    "⚠️ Server already running for profile: {}",
                    profile_name
                ));
                return;
            }

            // Find next available port
            let mut port = self.next_port;
            while self.running_servers.values().any(|(p, _)| *p == port) {
                port += 1;
            }

            // Convert TUI Profile to Server Profile
            let server_profile = ServerProfile::new(profile_name.clone(), profile_path.clone());
            let mut server = CloudServer::new();

            match server.start_server(server_profile, port) {
                Ok(_) => {
                    self.running_servers
                        .insert(profile_name.clone(), (port, server));
                    self.next_port = port + 1;

                    self.add_server_log(&format!(
                        "🚀 Starting server for profile: {}",
                        profile_name
                    ));
                    self.add_server_log(&format!(
                        "📁 Serving files from: {}",
                        profile_path.display()
                    ));
                    self.add_server_log(&format!("🌐 Server started on http://127.0.0.1:{}", port));
                    self.add_server_log(&format!(
                        "📋 Access files at: http://127.0.0.1:{}/files",
                        port
                    ));
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
    }

    pub fn stop_server(&mut self) {
        if let Some(profile) = self.profiles.get(self.selected_profile_index) {
            let profile_name = profile.name.clone();

            if let Some((port, mut server)) = self.running_servers.remove(&profile_name) {
                server.stop_server();
                self.add_server_log(&format!(
                    "🛑 Stopped server for profile: {} (port {})",
                    profile_name, port
                ));
                self.add_server_log("📴 All connections closed");
            } else {
                self.add_server_log(&format!(
                    "⚠️ No server running for profile: {}",
                    profile_name
                ));
            }
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

    pub fn load_profiles(&mut self) {
        let config = crate::config::Config::load_or_default();
        let profiles_path = self.expand_path(&config.profiles_path);

        if let Ok(entries) = std::fs::read_dir(&profiles_path) {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    if entry.path().is_dir() {
                        let profile = Profile {
                            name: name.to_string(),
                            folder_path: entry.path(),
                        };
                        self.profiles.push(profile);
                    }
                }
            }
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
        self.profiles.iter().any(|p| p.name == name)
    }

    pub fn delete_selected_profile(&mut self) {
        if !self.profiles.is_empty() && self.selected_profile_index < self.profiles.len() {
            let _profile_name = self.profiles[self.selected_profile_index].name.clone();
            let profile_path = self.profiles[self.selected_profile_index]
                .folder_path
                .clone();

            // Try to remove the directory
            match std::fs::remove_dir_all(&profile_path) {
                Ok(_) => {
                    self.profiles.remove(self.selected_profile_index);
                    // Adjust selected index if needed
                    if self.selected_profile_index >= self.profiles.len()
                        && !self.profiles.is_empty()
                    {
                        self.selected_profile_index = self.profiles.len() - 1;
                    }
                }
                Err(_) => {
                    // Handle error silently for now
                }
            }
        }
    }

    pub fn navigate_profile_up(&mut self) {
        if self.selected_profile_index > 0 {
            self.selected_profile_index -= 1;
        }
    }

    pub fn navigate_profile_down(&mut self) {
        if self.selected_profile_index < self.profiles.len().saturating_sub(1) {
            self.selected_profile_index += 1;
        }
    }

    pub fn is_server_running(&self) -> bool {
        if let Some(profile) = self.profiles.get(self.selected_profile_index) {
            self.running_servers.contains_key(&profile.name)
        } else {
            false
        }
    }

    pub fn get_server_port(&self) -> Option<u16> {
        if let Some(profile) = self.profiles.get(self.selected_profile_index) {
            self.running_servers
                .get(&profile.name)
                .map(|(port, _)| *port)
        } else {
            None
        }
    }

    pub fn get_running_servers_count(&self) -> usize {
        self.running_servers.len()
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
            FocusedPanel::Profiles => "Profiles".to_string(),
            FocusedPanel::ServerInfo => "ServerInfo".to_string(),
            FocusedPanel::ServerLogs => "ServerLogs".to_string(),
        }
    }

    fn cycle_focus_forward(&mut self) {
        self.focused_panel = match self.focused_panel {
            FocusedPanel::Profiles => FocusedPanel::ServerInfo,
            FocusedPanel::ServerInfo => FocusedPanel::ServerLogs,
            FocusedPanel::ServerLogs => FocusedPanel::Profiles,
        };
    }

    fn cycle_focus_backward(&mut self) {
        self.focused_panel = match self.focused_panel {
            FocusedPanel::Profiles => FocusedPanel::ServerLogs,
            FocusedPanel::ServerInfo => FocusedPanel::Profiles,
            FocusedPanel::ServerLogs => FocusedPanel::ServerInfo,
        };
    }

    fn handle_navigation(&mut self, key: KeyCode) -> bool {
        match key {
            KeyCode::Char('j') => {
                match self.focused_panel {
                    FocusedPanel::Profiles => self.navigate_profile_down(),
                    FocusedPanel::ServerInfo => self.cycle_focus_forward(),
                    FocusedPanel::ServerLogs => self.scroll_logs_down(),
                }
                true
            }
            KeyCode::Char('k') => {
                match self.focused_panel {
                    FocusedPanel::Profiles => self.navigate_profile_up(),
                    FocusedPanel::ServerInfo => self.cycle_focus_forward(),
                    FocusedPanel::ServerLogs => self.scroll_logs_up(),
                }
                true
            }
            KeyCode::Char('g') => {
                // Handle gg sequence (go to top)
                match self.focused_panel {
                    FocusedPanel::Profiles => {
                        self.selected_profile_index = 0;
                    }
                    FocusedPanel::ServerInfo => self.cycle_focus_forward(),
                    FocusedPanel::ServerLogs => self.scroll_logs_to_top(),
                }
                true
            }
            KeyCode::Char('G') => {
                match self.focused_panel {
                    FocusedPanel::Profiles => {
                        self.selected_profile_index = self.profiles.len().saturating_sub(1);
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
                self.create_profile(&crate::config::Config::load_or_default());
            }
            KeyCode::Esc => {
                self.cancel_creating_profile();
            }
            KeyCode::Char(c) => {
                self.new_profile_name.push(c);
                self.profile_creation_error = None; // Clear error when typing
            }
            KeyCode::Backspace => {
                self.new_profile_name.pop();
                self.profile_creation_error = None; // Clear error when editing
            }
            _ => {}
        }
    }
}
