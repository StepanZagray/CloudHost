#[derive(Debug, Clone, PartialEq, Default)]
pub enum CloudFocusedPanel {
    #[default]
    Clouds,
    CloudInfo,
    CloudLogs,
}
use crate::tabs::focus::TabFocus;
use crate::utils::password::PasswordCreationState;
use cloudhost_server::{Cloud, Orchestrator};
use ratatui::crossterm::event::KeyCode;
use ratatui::widgets::{ListState, ScrollbarState};
use std::collections::HashMap;

#[derive(Default)]
pub struct CloudsState {
    pub clouds: Vec<Cloud>,
    pub selected_cloud_index: usize,
    pub cloud_start_error: Option<String>,
    pub cloud_logs: Vec<String>,
    pub log_scroll_offset: usize,
    pub focused_panel: CloudFocusedPanel,
    pub running_clouds: HashMap<String, u16>,
    pub cloud_logs_list_state: ListState,
    pub cloud_logs_scroll_state: ScrollbarState,
    pub clouds_list_state: ListState,
    pub clouds_scroll_state: ScrollbarState,
    // Shared password creation state
    pub password_creation: PasswordCreationState,
}

impl CloudsState {
    pub fn new() -> Self {
        Self {
            clouds: Vec::new(),
            selected_cloud_index: 0,
            cloud_start_error: None,
            cloud_logs: Vec::new(),
            log_scroll_offset: 0,
            focused_panel: CloudFocusedPanel::Clouds,
            running_clouds: HashMap::new(),
            cloud_logs_list_state: ListState::default(),
            cloud_logs_scroll_state: ScrollbarState::default(),
            clouds_list_state: ListState::default(),
            clouds_scroll_state: ScrollbarState::default(),
            password_creation: PasswordCreationState::new(),
        }
    }
}

impl CloudsState {
    pub async fn start_server(&mut self, orchestrator: &mut Orchestrator) {
        if self.clouds.is_empty() || self.selected_cloud_index >= self.clouds.len() {
            self.cloud_start_error = Some("❌ No cloud selected".to_string());
            return;
        }

        let cloud_name = &self.clouds[self.selected_cloud_index].name;

        match orchestrator.start_cloud(cloud_name).await {
            Ok(port) => {
                self.running_clouds.insert(cloud_name.clone(), port);
                self.cloud_logs.push(format!(
                    "✅ Started cloud '{}' on port {}",
                    cloud_name, port
                ));
                self.cloud_start_error = None;
            }
            Err(e) => {
                self.cloud_start_error = Some(format!("❌ {}", e));
            }
        }
    }

    pub async fn stop_server(&mut self, orchestrator: &mut Orchestrator) {
        if self.clouds.is_empty() || self.selected_cloud_index >= self.clouds.len() {
            return;
        }

        let cloud_name = &self.clouds[self.selected_cloud_index].name;

        match orchestrator.stop_cloud(cloud_name).await {
            Ok(_) => {
                self.running_clouds.remove(cloud_name);
                self.cloud_logs
                    .push(format!("🛑 Stopped cloud '{}'", cloud_name));
                self.cloud_start_error = None;
            }
            Err(e) => {
                self.cloud_start_error = Some(format!("❌ {}", e));
            }
        }
    }

    pub async fn stop_all_servers(&mut self, orchestrator: &mut Orchestrator) {
        match orchestrator.stop_all().await {
            Ok(_) => {
                self.running_clouds.clear();
                self.cloud_logs.push("🛑 Stopped all clouds".to_string());
                self.cloud_start_error = None;
            }
            Err(e) => {
                self.cloud_start_error = Some(format!("❌ {}", e));
            }
        }
    }

    pub fn set_password(
        &mut self,
        orchestrator: &mut Orchestrator,
        password: &str,
    ) -> Result<(), String> {
        if self.clouds.is_empty() || self.selected_cloud_index >= self.clouds.len() {
            return Err("No cloud selected".to_string());
        }

        let cloud_name = &self.clouds[self.selected_cloud_index].name;
        if let Err(e) = orchestrator.set_cloud_password(cloud_name, password) {
            let error_msg = format!("Failed to set password: {}", e);
            self.password_creation.password_error = Some(error_msg.clone());
            Err(error_msg)
        } else {
            self.password_creation.password_success = true;
            self.password_creation.clear_password_creation();
            Ok(())
        }
    }

    pub fn has_password(&self, orchestrator: &Orchestrator) -> bool {
        if self.clouds.is_empty() || self.selected_cloud_index >= self.clouds.len() {
            return false;
        }

        let cloud_name = &self.clouds[self.selected_cloud_index].name;
        orchestrator.cloud_has_password(cloud_name)
    }

    pub fn verify_password(&self, orchestrator: &Orchestrator, password: &str) -> bool {
        if self.clouds.is_empty() || self.selected_cloud_index >= self.clouds.len() {
            return false;
        }

        let cloud_name = &self.clouds[self.selected_cloud_index].name;
        orchestrator.verify_cloud_password(cloud_name, password)
    }

    pub fn start_creating_password(&mut self) {
        if self.clouds.is_empty() || self.selected_cloud_index >= self.clouds.len() {
            return;
        }
        self.password_creation.start_creating_password();
    }

    pub fn clear_password_creation(&mut self) {
        self.password_creation.clear_password_creation();
    }

    pub fn handle_password_input(&mut self, key: char) {
        self.password_creation.handle_password_input(key);
    }

    pub fn is_cloud_running(&self, cloud_name: &str) -> bool {
        self.running_clouds.contains_key(cloud_name)
    }

    pub fn get_cloud_port(&self, cloud_name: &str) -> Option<u16> {
        self.running_clouds.get(cloud_name).copied()
    }

    pub fn add_cloud_log(&mut self, message: String) {
        self.cloud_logs.push(message);
        // Keep only last 100 logs
        if self.cloud_logs.len() > 100 {
            self.cloud_logs.remove(0);
        }
    }
}

impl TabFocus for CloudsState {
    fn get_focused_element(&self) -> String {
        match self.focused_panel {
            CloudFocusedPanel::Clouds => "clouds".to_string(),
            CloudFocusedPanel::CloudInfo => "cloud_info".to_string(),
            CloudFocusedPanel::CloudLogs => "cloud_logs".to_string(),
        }
    }

    fn cycle_focus_forward(&mut self) {
        self.focused_panel = match self.focused_panel {
            CloudFocusedPanel::Clouds => CloudFocusedPanel::CloudInfo,
            CloudFocusedPanel::CloudInfo => CloudFocusedPanel::CloudLogs,
            CloudFocusedPanel::CloudLogs => CloudFocusedPanel::Clouds,
        };
    }

    fn cycle_focus_backward(&mut self) {
        self.focused_panel = match self.focused_panel {
            CloudFocusedPanel::Clouds => CloudFocusedPanel::CloudLogs,
            CloudFocusedPanel::CloudInfo => CloudFocusedPanel::Clouds,
            CloudFocusedPanel::CloudLogs => CloudFocusedPanel::CloudInfo,
        };
    }

    fn handle_navigation(&mut self, key: KeyCode) -> bool {
        match key {
            KeyCode::Up | KeyCode::Char('k') => {
                if self.focused_panel == CloudFocusedPanel::Clouds && self.selected_cloud_index > 0
                {
                    self.selected_cloud_index -= 1;
                    self.clouds_list_state
                        .select(Some(self.selected_cloud_index));
                }
                true
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.focused_panel == CloudFocusedPanel::Clouds
                    && self.selected_cloud_index < self.clouds.len().saturating_sub(1)
                {
                    self.selected_cloud_index += 1;
                    self.clouds_list_state
                        .select(Some(self.selected_cloud_index));
                }
                true
            }
            KeyCode::Tab => {
                self.cycle_focus_forward();
                true
            }
            _ => false,
        }
    }
}
