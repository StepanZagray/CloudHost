#[derive(Debug, Clone, PartialEq, Default)]
pub enum DashboardPanel {
    #[default]
    Services,
    Overview,
    Activity,
}
use crate::tabs::focus::TabFocus;
use crate::utils::password::PasswordCreationState;
use cloudhost_server::{Cloud, Orchestrator};
use ratatui::crossterm::event::KeyCode;
use ratatui::widgets::{ListState, ScrollbarState};
use std::collections::HashMap;

#[derive(Default)]
pub struct DashboardState {
    pub clouds: Vec<Cloud>,
    pub selected_cloud_index: usize,
    pub cloud_start_error: Option<String>,
    pub focused_panel: DashboardPanel,
    pub running_clouds: HashMap<String, u16>,
    pub activity_list_state: ListState,
    pub activity_scroll_state: ScrollbarState,
    pub clouds_list_state: ListState,
    pub clouds_scroll_state: ScrollbarState,
    // Shared password creation state
    pub password_creation: PasswordCreationState,
}

impl DashboardState {
    pub fn new() -> Self {
        Self {
            clouds: Vec::new(),
            selected_cloud_index: 0,
            cloud_start_error: None,
            focused_panel: DashboardPanel::Services,
            running_clouds: HashMap::new(),
            activity_list_state: ListState::default(),
            activity_scroll_state: ScrollbarState::default(),
            clouds_list_state: ListState::default(),
            clouds_scroll_state: ScrollbarState::default(),
            password_creation: PasswordCreationState::new(),
        }
    }
}

impl DashboardState {
    pub async fn start_server(&mut self, orchestrator: &mut Orchestrator) {
        if self.clouds.is_empty() || self.selected_cloud_index >= self.clouds.len() {
            self.cloud_start_error = Some("❌ No cloud selected".to_string());
            return;
        }

        let cloud_name = &self.clouds[self.selected_cloud_index].name;

        match orchestrator.start_cloud(cloud_name).await {
            Ok(port) => {
                self.running_clouds.insert(cloud_name.clone(), port);
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

    pub fn handle_activity_navigation(&mut self, key: KeyCode, log_count: usize) -> bool {
        if self.focused_panel != DashboardPanel::Activity {
            return false;
        }

        if log_count == 0 {
            return matches!(key, KeyCode::Up | KeyCode::Down | KeyCode::Char('j' | 'k'));
        }

        let current = self
            .activity_list_state
            .selected()
            .unwrap_or(log_count.saturating_sub(1));
        let next = match key {
            KeyCode::Up | KeyCode::Char('k') => current.saturating_sub(1),
            KeyCode::Down | KeyCode::Char('j') => (current + 1).min(log_count - 1),
            KeyCode::Char('g') => 0,
            KeyCode::Char('G') => log_count - 1,
            _ => return false,
        };
        self.activity_list_state.select(Some(next));
        true
    }
}

impl TabFocus for DashboardState {
    fn get_focused_element(&self) -> String {
        match self.focused_panel {
            DashboardPanel::Services => "services".to_string(),
            DashboardPanel::Overview => "overview".to_string(),
            DashboardPanel::Activity => "activity".to_string(),
        }
    }

    fn cycle_focus_forward(&mut self) {
        self.focused_panel = match self.focused_panel {
            DashboardPanel::Services => DashboardPanel::Overview,
            DashboardPanel::Overview => DashboardPanel::Activity,
            DashboardPanel::Activity => DashboardPanel::Services,
        };
    }

    fn cycle_focus_backward(&mut self) {
        self.focused_panel = match self.focused_panel {
            DashboardPanel::Services => DashboardPanel::Activity,
            DashboardPanel::Overview => DashboardPanel::Services,
            DashboardPanel::Activity => DashboardPanel::Overview,
        };
    }

    fn handle_navigation(&mut self, key: KeyCode) -> bool {
        match key {
            KeyCode::Up | KeyCode::Char('k') => {
                if self.focused_panel == DashboardPanel::Services && self.selected_cloud_index > 0 {
                    self.selected_cloud_index -= 1;
                    self.clouds_list_state
                        .select(Some(self.selected_cloud_index));
                }
                true
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.focused_panel == DashboardPanel::Services
                    && self.selected_cloud_index < self.clouds.len().saturating_sub(1)
                {
                    self.selected_cloud_index += 1;
                    self.clouds_list_state
                        .select(Some(self.selected_cloud_index));
                }
                true
            }
            KeyCode::Char('g') => {
                if self.focused_panel == DashboardPanel::Services && !self.clouds.is_empty() {
                    self.selected_cloud_index = 0;
                    self.clouds_list_state.select(Some(0));
                }
                true
            }
            KeyCode::Char('G') => {
                if self.focused_panel == DashboardPanel::Services && !self.clouds.is_empty() {
                    self.selected_cloud_index = self.clouds.len() - 1;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn activity_navigation_respects_bounds() {
        let mut state = DashboardState::new();
        state.focused_panel = DashboardPanel::Activity;
        state.activity_list_state.select(Some(2));

        assert!(state.handle_activity_navigation(KeyCode::Down, 3));
        assert_eq!(state.activity_list_state.selected(), Some(2));

        assert!(state.handle_activity_navigation(KeyCode::Char('g'), 3));
        assert_eq!(state.activity_list_state.selected(), Some(0));

        assert!(state.handle_activity_navigation(KeyCode::Char('G'), 3));
        assert_eq!(state.activity_list_state.selected(), Some(2));
    }
}
