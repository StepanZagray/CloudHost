use crate::tabs::focus::TabFocus;
use ratatui::crossterm::event::KeyCode;
use ratatui::widgets::{ListState, ScrollbarState};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsAction {
    OpenTuiConfig,
    OpenStorageConfig,
    ReloadTuiConfig,
    ReloadStorageConfig,
    ReloadAllConfigs,
    ResetTuiConfig,
}

impl SettingsAction {
    pub const ALL: [Self; 6] = [
        Self::OpenTuiConfig,
        Self::OpenStorageConfig,
        Self::ReloadTuiConfig,
        Self::ReloadStorageConfig,
        Self::ReloadAllConfigs,
        Self::ResetTuiConfig,
    ];

    pub const fn title(self) -> &'static str {
        match self {
            Self::OpenTuiConfig => "Open TUI config",
            Self::OpenStorageConfig => "Open storage config",
            Self::ReloadTuiConfig => "Reload TUI config",
            Self::ReloadStorageConfig => "Reload storage config",
            Self::ReloadAllConfigs => "Reload all configs",
            Self::ResetTuiConfig => "Reset TUI config",
        }
    }

    pub const fn description(self) -> &'static str {
        match self {
            Self::OpenTuiConfig => "Edit keybindings and TUI preferences in your default editor.",
            Self::OpenStorageConfig => "Edit the saved folder and cloud definitions.",
            Self::ReloadTuiConfig => "Apply the latest TUI keybindings without restarting.",
            Self::ReloadStorageConfig => "Reload saved folders and clouds, then refresh the UI.",
            Self::ReloadAllConfigs => "Reload both configuration files.",
            Self::ResetTuiConfig => "Restore the default TUI configuration after confirmation.",
        }
    }

    pub const fn config_action(self) -> Option<&'static str> {
        match self {
            Self::OpenTuiConfig | Self::OpenStorageConfig | Self::ResetTuiConfig => None,
            Self::ReloadTuiConfig => Some("Reload TUI Config"),
            Self::ReloadStorageConfig => Some("Reload Storage Config"),
            Self::ReloadAllConfigs => Some("Reload All Configs"),
        }
    }

    pub const fn is_destructive(self) -> bool {
        matches!(self, Self::ResetTuiConfig)
    }
}

#[derive(Default)]
pub struct SettingsState {
    pub list_state: ListState,
    pub scroll_state: ScrollbarState,
}

impl TabFocus for SettingsState {
    fn get_focused_element(&self) -> String {
        "SettingsList".to_string()
    }

    fn cycle_focus_forward(&mut self) {}

    fn cycle_focus_backward(&mut self) {}

    fn handle_navigation(&mut self, key: KeyCode) -> bool {
        let current = self.list_state.selected().unwrap_or(0);
        let last = SettingsAction::ALL.len().saturating_sub(1);

        match key {
            KeyCode::Up | KeyCode::Char('k') => {
                self.list_state.select(Some(current.saturating_sub(1)));
                true
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.list_state.select(Some((current + 1).min(last)));
                true
            }
            KeyCode::Char('g') => {
                self.list_state.select(Some(0));
                true
            }
            KeyCode::Char('G') => {
                self.list_state.select(Some(last));
                true
            }
            _ => false,
        }
    }
}

impl SettingsState {
    pub fn new() -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        Self {
            list_state,
            scroll_state: ScrollbarState::default(),
        }
    }

    pub fn selected_action(&self) -> SettingsAction {
        let index = self
            .list_state
            .selected()
            .unwrap_or(0)
            .min(SettingsAction::ALL.len().saturating_sub(1));
        SettingsAction::ALL[index]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn navigation_stays_within_available_actions() {
        let mut state = SettingsState::new();

        for _ in 0..20 {
            state.handle_navigation(KeyCode::Down);
        }
        assert_eq!(state.selected_action(), SettingsAction::ResetTuiConfig);

        for _ in 0..20 {
            state.handle_navigation(KeyCode::Up);
        }
        assert_eq!(state.selected_action(), SettingsAction::OpenTuiConfig);
    }
}
