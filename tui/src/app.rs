use ratatui::crossterm::event::KeyCode;
use strum::IntoEnumIterator;

use crate::tabs::{dashboard, focus::TabFocus, settings, storage, SelectedTab};
use cloudhost_server::debug_stream::DebugMessage;

// Timeout for key sequences (like Vim's timeoutlen)
const KEY_SEQUENCE_TIMEOUT_MS: u64 = 1000; // 1 second
const NOTICE_TIMEOUT_MS: u64 = 6_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoticeKind {
    Info,
    Success,
    Warning,
    Error,
}

#[derive(Debug, Clone)]
pub struct Notice {
    pub kind: NoticeKind,
    pub message: String,
    shown_at: std::time::Instant,
}

#[derive(Debug, Clone)]
enum ConfirmationAction {
    RemoveFolder(String),
    RemoveCloud(String),
    ResetTuiConfig,
}

#[derive(Debug, Clone)]
pub struct Confirmation {
    pub title: String,
    pub description: String,
    pub confirm_label: String,
    action: ConfirmationAction,
}

#[derive(Default)]
pub struct App {
    pub state: AppState,
    pub selected_tab: SelectedTab,
    pub config: crate::config::Config,
    pub input_state: InputState,
    pub debug_mode: bool,
    pub debug_info: Vec<String>,
    pub notice: Option<Notice>,
    pub pending_confirmation: Option<Confirmation>,
    pub help_open: bool,
    pub search: Option<ListSearch>,
    pub activity: Vec<DebugMessage>,
    activity_cloud: Option<String>,

    // Shared orchestrator instance owns folder, cloud, and server management.
    pub orchestrator: cloudhost_server::Orchestrator,

    // Tab states
    pub dashboard: dashboard::state::DashboardState,
    pub storage: storage::state::StorageState,
    pub settings: settings::state::SettingsState,
}

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum AppState {
    #[default]
    Running,
    Quitting,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum InputState {
    #[default]
    Normal,
    KeySequence(String, std::time::Instant), // Stores the current key sequence and when it started
}

/// Identifies the currently searchable list. Search is scoped to the focused
/// list so a query never hides items in unrelated panels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SearchTarget {
    DashboardServices,
    DashboardActivity,
    StorageFolders,
    StorageClouds,
    Settings,
    CloudEditFolders,
}

#[derive(Debug, Clone)]
pub struct ListSearch {
    target: SearchTarget,
    query: String,
    /// The original index of the pending result. It is committed only on Enter.
    selected_index: Option<usize>,
    /// Needed for lists whose selection is otherwise stored only in `ListState`.
    previous_selected_index: Option<usize>,
}

impl App {
    pub fn new() -> Self {
        let config = crate::config::Config::load_or_default();

        // Load server config

        // Create orchestrator instance
        let orchestrator = cloudhost_server::Orchestrator::new();

        let mut app = Self {
            config: config.clone(),
            orchestrator,
            dashboard: dashboard::state::DashboardState::new(),
            storage: storage::state::StorageState::default(),
            settings: settings::state::SettingsState::new(),
            ..Default::default()
        };

        // Load folders and clouds into their tab-specific projections.
        app.sync_from_orchestrator();

        app
    }

    fn search_target_for_current_context(&self) -> Option<SearchTarget> {
        // The cloud edit dialog has its own selectable folder list. Other modal
        // fields should keep receiving `/` as ordinary text input.
        if self.storage.editing_cloud {
            return (self.storage.cloud_edit_focus == storage::state::CloudEditFocus::Folders)
                .then_some(SearchTarget::CloudEditFolders);
        }

        if self.dashboard.password_creation.creating_password
            || self.storage.adding_folder
            || self.storage.creating_cloud
            || self.storage.password_creation.creating_password
            || self.storage.editing_folder
        {
            return None;
        }

        match self.selected_tab {
            SelectedTab::Dashboard => match self.dashboard.focused_panel {
                dashboard::state::DashboardPanel::Services => Some(SearchTarget::DashboardServices),
                dashboard::state::DashboardPanel::Activity => Some(SearchTarget::DashboardActivity),
                dashboard::state::DashboardPanel::Overview => None,
            },
            SelectedTab::Storage => match self.storage.focused_panel {
                storage::state::StoragePanel::Folders => Some(SearchTarget::StorageFolders),
                storage::state::StoragePanel::Clouds => Some(SearchTarget::StorageClouds),
            },
            SelectedTab::Settings => Some(SearchTarget::Settings),
        }
    }

    fn selected_index_for_search_target(&self, target: SearchTarget) -> Option<usize> {
        match target {
            SearchTarget::DashboardServices => self
                .dashboard
                .clouds
                .get(self.dashboard.selected_cloud_index)
                .map(|_| self.dashboard.selected_cloud_index),
            SearchTarget::DashboardActivity => self
                .dashboard
                .activity_list_state
                .selected()
                .filter(|index| *index < self.activity.len()),
            SearchTarget::StorageFolders | SearchTarget::CloudEditFolders => self
                .storage
                .folders
                .get(self.storage.selected_folder_index)
                .map(|_| self.storage.selected_folder_index),
            SearchTarget::StorageClouds => self
                .storage
                .clouds
                .get(self.storage.selected_cloud_index)
                .map(|_| self.storage.selected_cloud_index),
            SearchTarget::Settings => self
                .settings
                .list_state
                .selected()
                .filter(|index| *index < settings::state::SettingsAction::ALL.len()),
        }
    }

    pub(crate) fn filtered_indices(&self, target: SearchTarget) -> Vec<usize> {
        let query = self
            .search
            .as_ref()
            .filter(|search| search.target == target)
            .map(|search| search.query.to_lowercase())
            .unwrap_or_default();
        let matches = |text: String| text.to_lowercase().contains(&query);

        match target {
            SearchTarget::DashboardServices => self
                .dashboard
                .clouds
                .iter()
                .enumerate()
                .filter_map(|(index, cloud)| matches(cloud.name.clone()).then_some(index))
                .collect(),
            SearchTarget::DashboardActivity => self
                .activity
                .iter()
                .enumerate()
                .filter_map(|(index, log)| {
                    matches(format!(
                        "{} {} {} {}",
                        log.timestamp.format("%H:%M:%S"),
                        log.level,
                        log.source,
                        log.message
                    ))
                    .then_some(index)
                })
                .collect(),
            SearchTarget::StorageFolders | SearchTarget::CloudEditFolders => self
                .storage
                .folders
                .iter()
                .enumerate()
                .filter_map(|(index, folder)| matches(folder.name.clone()).then_some(index))
                .collect(),
            SearchTarget::StorageClouds => self
                .storage
                .clouds
                .iter()
                .enumerate()
                .filter_map(|(index, cloud)| matches(cloud.name.clone()).then_some(index))
                .collect(),
            SearchTarget::Settings => settings::state::SettingsAction::ALL
                .iter()
                .enumerate()
                .filter_map(|(index, action)| {
                    matches(format!("{} {}", action.title(), action.description())).then_some(index)
                })
                .collect(),
        }
    }

    pub(crate) fn search_title_suffix(&self, target: SearchTarget) -> String {
        self.search
            .as_ref()
            .filter(|search| search.target == target)
            .map(|search| format!(" · /{}█", search.query))
            .unwrap_or_default()
    }

    pub(crate) fn search_selected_position(&self, target: SearchTarget) -> Option<usize> {
        let selected_index = self
            .search
            .as_ref()
            .filter(|search| search.target == target)
            .and_then(|search| search.selected_index)?;
        self.filtered_indices(target)
            .iter()
            .position(|index| *index == selected_index)
    }

    pub(crate) fn search_selected_index(&self, target: SearchTarget) -> Option<usize> {
        self.search
            .as_ref()
            .filter(|search| search.target == target)
            .and_then(|search| search.selected_index)
    }

    pub(crate) fn is_searching(&self, target: SearchTarget) -> bool {
        self.search
            .as_ref()
            .is_some_and(|search| search.target == target)
    }

    fn start_list_search(&mut self) -> bool {
        let Some(target) = self.search_target_for_current_context() else {
            return false;
        };
        let previous_selected_index = self.selected_index_for_search_target(target);
        let indices = self.filtered_indices(target);
        let selected_index = previous_selected_index
            .filter(|index| indices.contains(index))
            .or_else(|| indices.first().copied());
        self.search = Some(ListSearch {
            target,
            query: String::new(),
            selected_index,
            previous_selected_index,
        });
        self.input_state = InputState::Normal;
        true
    }

    fn reconcile_search_selection(&mut self) {
        let Some(search) = self.search.as_ref() else {
            return;
        };
        let target = search.target;
        let selected_index = search.selected_index;
        let indices = self.filtered_indices(target);
        let selected_index = selected_index
            .filter(|index| indices.contains(index))
            .or_else(|| indices.first().copied());
        if let Some(search) = self.search.as_mut() {
            search.selected_index = selected_index;
        }
    }

    fn move_search_selection(&mut self, next: bool) {
        let Some(target) = self.search.as_ref().map(|search| search.target) else {
            return;
        };
        let indices = self.filtered_indices(target);
        let selected_index = self
            .search
            .as_ref()
            .and_then(|search| search.selected_index);
        let position = selected_index
            .and_then(|index| indices.iter().position(|candidate| *candidate == index))
            .unwrap_or(0);
        let next_index = if indices.is_empty() {
            None
        } else if next {
            indices.get((position + 1).min(indices.len() - 1)).copied()
        } else {
            indices.get(position.saturating_sub(1)).copied()
        };
        if let Some(search) = self.search.as_mut() {
            search.selected_index = next_index;
        }
    }

    fn finish_list_search(&mut self, commit: bool) {
        let Some(search) = self.search.take() else {
            return;
        };
        let selected_index = if commit {
            search.selected_index
        } else {
            search.previous_selected_index
        };

        match search.target {
            SearchTarget::DashboardServices => {
                if commit {
                    if let Some(index) =
                        selected_index.filter(|index| *index < self.dashboard.clouds.len())
                    {
                        self.dashboard.selected_cloud_index = index;
                    }
                }
                self.dashboard.clouds_list_state.select(
                    self.dashboard
                        .clouds
                        .get(self.dashboard.selected_cloud_index)
                        .map(|_| self.dashboard.selected_cloud_index),
                );
            }
            SearchTarget::DashboardActivity => {
                self.dashboard
                    .activity_list_state
                    .select(selected_index.filter(|index| *index < self.activity.len()));
            }
            SearchTarget::StorageFolders | SearchTarget::CloudEditFolders => {
                if commit {
                    if let Some(index) =
                        selected_index.filter(|index| *index < self.storage.folders.len())
                    {
                        self.storage.selected_folder_index = index;
                    }
                }
                self.storage.folders_list_state.select(
                    self.storage
                        .folders
                        .get(self.storage.selected_folder_index)
                        .map(|_| self.storage.selected_folder_index),
                );
            }
            SearchTarget::StorageClouds => {
                if commit {
                    if let Some(index) =
                        selected_index.filter(|index| *index < self.storage.clouds.len())
                    {
                        self.storage.selected_cloud_index = index;
                        self.storage.password_display_state =
                            storage::state::PasswordDisplayState::Hidden;
                    }
                }
                self.storage.clouds_list_state.select(
                    self.storage
                        .clouds
                        .get(self.storage.selected_cloud_index)
                        .map(|_| self.storage.selected_cloud_index),
                );
            }
            SearchTarget::Settings => {
                self.settings.list_state.select(
                    selected_index
                        .filter(|index| *index < settings::state::SettingsAction::ALL.len()),
                );
            }
        }
    }

    fn handle_list_search_key(&mut self, key: KeyCode) -> bool {
        if self.search.is_none() {
            return false;
        }

        match key {
            KeyCode::Esc => self.finish_list_search(false),
            KeyCode::Enter => self.finish_list_search(true),
            KeyCode::Backspace => {
                if let Some(search) = self.search.as_mut() {
                    search.query.pop();
                }
                self.reconcile_search_selection();
            }
            KeyCode::Up => self.move_search_selection(false),
            KeyCode::Down => self.move_search_selection(true),
            KeyCode::Char(character) => {
                if let Some(search) = self.search.as_mut() {
                    search.query.push(character);
                }
                self.reconcile_search_selection();
            }
            _ => {}
        }
        true
    }

    fn sync_from_orchestrator(&mut self) {
        let selected_folder_names: std::collections::HashSet<String> = self
            .storage
            .selected_folders
            .iter()
            .filter_map(|index| {
                self.storage
                    .folders
                    .get(*index)
                    .map(|folder| folder.name.clone())
            })
            .collect();
        let selected_folder_name = self
            .storage
            .folders
            .get(self.storage.selected_folder_index)
            .map(|folder| folder.name.clone());
        let selected_manage_cloud_name = self
            .storage
            .clouds
            .get(self.storage.selected_cloud_index)
            .map(|cloud| cloud.name.clone());
        let selected_server_cloud_name = self
            .dashboard
            .clouds
            .get(self.dashboard.selected_cloud_index)
            .map(|cloud| cloud.name.clone());

        let folders = self.orchestrator.get_cloud_folders();
        let clouds = self.orchestrator.get_clouds();

        self.storage.folders = folders;
        self.storage.selected_folders = self
            .storage
            .folders
            .iter()
            .enumerate()
            .filter_map(|(index, folder)| {
                selected_folder_names
                    .contains(&folder.name)
                    .then_some(index)
            })
            .collect();
        self.storage.selected_folder_index = selected_folder_name
            .and_then(|name| {
                self.storage
                    .folders
                    .iter()
                    .position(|folder| folder.name == name)
            })
            .unwrap_or(0)
            .min(self.storage.folders.len().saturating_sub(1));
        self.storage.folders_list_state.select(
            (!self.storage.folders.is_empty()).then_some(self.storage.selected_folder_index),
        );

        self.storage.clouds = clouds.clone();
        self.storage.selected_cloud_index = selected_manage_cloud_name
            .and_then(|name| {
                self.storage
                    .clouds
                    .iter()
                    .position(|cloud| cloud.name == name)
            })
            .unwrap_or(0)
            .min(self.storage.clouds.len().saturating_sub(1));
        self.storage
            .clouds_list_state
            .select((!self.storage.clouds.is_empty()).then_some(self.storage.selected_cloud_index));

        self.dashboard.clouds = clouds;
        self.dashboard.selected_cloud_index = selected_server_cloud_name
            .and_then(|name| {
                self.dashboard
                    .clouds
                    .iter()
                    .position(|cloud| cloud.name == name)
            })
            .unwrap_or(0)
            .min(self.dashboard.clouds.len().saturating_sub(1));
        self.dashboard.clouds_list_state.select(
            (!self.dashboard.clouds.is_empty()).then_some(self.dashboard.selected_cloud_index),
        );
        self.dashboard.running_clouds = self.orchestrator.get_running_clouds();
    }

    fn start_adding_folder(&mut self) {
        if self.selected_tab == SelectedTab::Storage {
            self.storage.adding_folder = true;
            self.storage.new_folder_name.clear();
            self.storage.new_folder_path.clear();
            self.storage.folder_input_field = storage::state::FolderInputField::Name;
            self.storage.folder_creation_error = None;
        }
    }

    fn start_creating_cloud(&mut self) {
        if self.selected_tab == SelectedTab::Storage {
            let selected_count = self.storage.selected_folder_count();
            if selected_count == 0 {
                self.show_notice(
                    NoticeKind::Warning,
                    "Select at least one folder before creating a cloud.",
                );
                return;
            }
            self.storage.start_creating_cloud();
        }
    }

    fn request_remove_focused_item(&mut self) {
        match self.storage.focused_panel {
            storage::state::StoragePanel::Folders => self.request_remove_folder(),
            storage::state::StoragePanel::Clouds => self.request_remove_cloud(),
        }
    }

    fn request_remove_folder(&mut self) {
        if self.selected_tab == SelectedTab::Storage
            && self.storage.focused_panel == storage::state::StoragePanel::Folders
            && !self.storage.folders.is_empty()
            && self.storage.selected_folder_index < self.storage.folders.len()
        {
            let folder_name = self.storage.folders[self.storage.selected_folder_index]
                .name
                .clone();
            self.pending_confirmation = Some(Confirmation {
                title: "Remove folder?".to_string(),
                description: format!(
                    "‘{}’ will be removed from CloudHost. Files in the local folder will not be deleted.",
                    folder_name
                ),
                confirm_label: "remove folder".to_string(),
                action: ConfirmationAction::RemoveFolder(folder_name),
            });
        } else {
            self.show_notice(NoticeKind::Warning, "Select a folder to remove.");
        }
    }

    fn request_remove_cloud(&mut self) {
        if self.selected_tab == SelectedTab::Storage
            && self.storage.focused_panel == storage::state::StoragePanel::Clouds
            && !self.storage.clouds.is_empty()
            && self.storage.selected_cloud_index < self.storage.clouds.len()
        {
            let cloud_name = self.storage.clouds[self.storage.selected_cloud_index]
                .name
                .clone();
            self.pending_confirmation = Some(Confirmation {
                title: "Remove cloud?".to_string(),
                description: format!(
                    "‘{}’ and its CloudHost password will be removed. Any running server will be stopped; local folders and files will stay intact.",
                    cloud_name
                ),
                confirm_label: "remove cloud".to_string(),
                action: ConfirmationAction::RemoveCloud(cloud_name),
            });
        } else {
            self.show_notice(NoticeKind::Warning, "Select a cloud to remove.");
        }
    }

    fn remove_folder(&mut self, folder_name: &str) {
        if let Err(error) = self.orchestrator.remove_cloud_folder(folder_name) {
            self.add_debug(&format!("Failed to remove folder: {error}"));
            self.show_notice(
                NoticeKind::Error,
                format!("Could not remove folder: {error}"),
            );
            return;
        }

        self.sync_from_orchestrator();
        self.show_notice(
            NoticeKind::Success,
            format!("Removed folder ‘{folder_name}’. Its files were left untouched."),
        );
        self.add_debug(&format!("Removed folder '{folder_name}'"));
    }

    fn remove_cloud(&mut self, cloud_name: &str) {
        if let Err(error) = self.orchestrator.remove_cloud(cloud_name) {
            self.add_debug(&format!("Failed to remove cloud: {error}"));
            self.show_notice(
                NoticeKind::Error,
                format!("Could not remove cloud: {error}"),
            );
            return;
        }

        self.sync_from_orchestrator();
        self.show_notice(
            NoticeKind::Success,
            format!("Removed cloud ‘{cloud_name}’."),
        );
        self.add_debug(&format!("Deleted cloud '{cloud_name}'"));
    }

    fn start_setting_cloud_password(&mut self) {
        match self.selected_tab {
            SelectedTab::Dashboard if !self.dashboard.clouds.is_empty() => {
                self.dashboard.start_creating_password();
            }
            SelectedTab::Storage
                if self.storage.focused_panel == storage::state::StoragePanel::Clouds
                    && self.storage.selected_cloud_index < self.storage.clouds.len() =>
            {
                self.storage.password_creation.start_creating_password();
            }
            SelectedTab::Dashboard | SelectedTab::Storage => {
                self.show_notice(NoticeKind::Warning, "Select a cloud first.");
            }
            SelectedTab::Settings => {}
        }
    }

    fn toggle_cloud_password_display(&mut self) {
        if self.selected_tab == SelectedTab::Storage
            && self.storage.focused_panel == crate::tabs::storage::state::StoragePanel::Clouds
            && !self.storage.clouds.is_empty()
            && self.storage.selected_cloud_index < self.storage.clouds.len()
        {
            self.storage.toggle_password_display();
            let new_state = match self.storage.password_display_state {
                crate::tabs::storage::state::PasswordDisplayState::Hidden => "hidden",
                crate::tabs::storage::state::PasswordDisplayState::Visible => "visible",
            };
            self.add_debug(&format!("Password display toggled to: {}", new_state));
        }
    }

    fn select_all_folders(&mut self) {
        if self.selected_tab == SelectedTab::Storage
            && self.storage.focused_panel == storage::state::StoragePanel::Folders
        {
            self.storage.select_all_folders();
            let count = self.storage.selected_folder_count();
            self.add_debug(&format!("Selected all {count} folders"));
            self.show_notice(NoticeKind::Info, format!("Selected all {count} folders."));
        } else {
            self.show_notice(
                NoticeKind::Warning,
                "Focus the Folders panel to select folders.",
            );
        }
    }

    fn start_editing_storage_item(&mut self) {
        if self.selected_tab == SelectedTab::Storage {
            match self.storage.focused_panel {
                storage::state::StoragePanel::Folders => {
                    if !self.storage.folders.is_empty() {
                        self.storage.start_editing_folder();
                    } else {
                        self.add_debug("No folders to edit");
                    }
                }
                storage::state::StoragePanel::Clouds => {
                    if !self.storage.clouds.is_empty() {
                        self.storage.start_editing_cloud();
                    } else {
                        self.add_debug("No clouds to edit");
                    }
                }
            }
        }
    }
    fn complete_cloud_creation(&mut self) {
        if self.selected_tab == SelectedTab::Storage && self.storage.creating_cloud {
            let cloud_name = self.storage.new_cloud_name.trim().to_string();

            if cloud_name.is_empty() {
                self.storage.cloud_creation_error = Some("Cloud name cannot be empty".to_string());
                return;
            }

            let selected_folder_names = self.storage.selected_folder_names();

            if selected_folder_names.is_empty() {
                self.storage.cloud_creation_error =
                    Some("Select at least one folder before creating a cloud.".to_string());
                return;
            }

            // Build the cloud from the selected folders.
            let folders: Vec<cloudhost_server::CloudFolder> = selected_folder_names
                .iter()
                .filter_map(|name| {
                    self.storage
                        .folders
                        .iter()
                        .find(|f| &f.name == name)
                        .map(|f| {
                            cloudhost_server::CloudFolder::new(
                                f.name.clone(),
                                f.folder_path.clone(),
                            )
                        })
                })
                .collect();

            if folders.is_empty() {
                self.storage.cloud_creation_error = Some("No valid folders selected".to_string());
                return;
            }

            let cloud = cloudhost_server::Cloud::new(cloud_name.clone(), folders);

            if let Err(e) = self.orchestrator.add_cloud(cloud) {
                self.storage.cloud_creation_error = Some(e.to_string());
                return;
            }

            // Reload from orchestrator to update the cloud list
            self.sync_from_orchestrator();

            self.add_debug(&format!(
                "Created cloud '{}' with {} folders",
                cloud_name,
                self.storage.selected_folder_count()
            ));
            self.show_notice(
                NoticeKind::Success,
                format!("Created cloud ‘{cloud_name}’. Set a password to finish."),
            );

            // Clear folder selections after creating the cloud.
            self.storage.clear_folder_selection();

            // Start password creation for the new cloud using shared component
            self.storage.password_creation.start_creating_password();
        }
    }

    fn complete_cloud_password_creation(&mut self) {
        if self.selected_tab == SelectedTab::Storage
            && self
                .storage
                .password_creation
                .is_password_creation_complete()
        {
            let password = self.storage.password_creation.get_password().to_string();

            // Determine if this is for a new cloud or existing cloud
            let cloud_name = if !self.storage.new_cloud_name.is_empty() {
                // New cloud creation
                self.storage.new_cloud_name.trim().to_string()
            } else if self.storage.selected_cloud_index < self.storage.clouds.len() {
                // Existing cloud password setting
                self.storage.clouds[self.storage.selected_cloud_index]
                    .name
                    .clone()
            } else {
                return; // No valid cloud
            };

            // Set the password for the cloud
            if let Err(e) = self.orchestrator.set_cloud_password(&cloud_name, &password) {
                self.storage.password_creation.password_error = Some(e.to_string());
                return;
            }

            self.add_debug(&format!(
                "Password set successfully for cloud '{}'",
                cloud_name
            ));
            self.show_notice(
                NoticeKind::Success,
                format!("Password updated for cloud ‘{cloud_name}’."),
            );

            // Clear password creation state
            self.storage.password_creation.clear_password_creation();

            // If this was for a new cloud, clear cloud creation state completely
            if !self.storage.new_cloud_name.is_empty() {
                self.storage.clear_cloud_creation();
            }
        }
    }

    fn handle_folder_creation_input(&mut self, key: KeyCode) -> bool {
        match key {
            KeyCode::Esc => {
                self.storage.adding_folder = false;
                self.storage.folder_creation_error = None;
                true
            }
            KeyCode::Enter => {
                match self.storage.folder_input_field {
                    storage::state::FolderInputField::Name => {
                        if !self.storage.new_folder_name.is_empty() {
                            self.storage.folder_input_field =
                                storage::state::FolderInputField::Path;
                        }
                    }
                    storage::state::FolderInputField::Path => {
                        if !self.storage.new_folder_path.is_empty() {
                            self.complete_folder_creation();
                        }
                    }
                }
                true
            }
            KeyCode::Tab | KeyCode::BackTab => {
                self.storage.folder_input_field = match self.storage.folder_input_field {
                    storage::state::FolderInputField::Name => {
                        storage::state::FolderInputField::Path
                    }
                    storage::state::FolderInputField::Path => {
                        storage::state::FolderInputField::Name
                    }
                };
                true
            }
            KeyCode::Backspace => {
                match self.storage.folder_input_field {
                    storage::state::FolderInputField::Name => {
                        self.storage.new_folder_name.pop();
                    }
                    storage::state::FolderInputField::Path => {
                        self.storage.new_folder_path.pop();
                    }
                }
                true
            }
            KeyCode::Char(c) => {
                match self.storage.folder_input_field {
                    storage::state::FolderInputField::Name => {
                        self.storage.new_folder_name.push(c);
                    }
                    storage::state::FolderInputField::Path => {
                        self.storage.new_folder_path.push(c);
                    }
                }
                true
            }
            _ => false,
        }
    }

    fn handle_cloud_creation_input(&mut self, key: KeyCode) -> bool {
        // If we're in password creation mode, handle password input
        if self.storage.password_creation.creating_password {
            let char_key = match key {
                KeyCode::Char(c) => c,
                KeyCode::Enter => '\n',
                KeyCode::Esc => '\x1b',
                KeyCode::Backspace => '\x08',
                KeyCode::Tab | KeyCode::BackTab => return true,
                _ => return false,
            };
            self.storage
                .password_creation
                .handle_password_input(char_key);

            // Check if password creation is complete
            if self
                .storage
                .password_creation
                .is_password_creation_complete()
            {
                self.complete_cloud_password_creation();
            }
            return true;
        }

        // Handle normal cloud creation input
        match key {
            KeyCode::Esc => {
                self.storage.clear_cloud_creation();
                true
            }
            KeyCode::Enter => {
                if !self.storage.new_cloud_name.is_empty() {
                    self.complete_cloud_creation();
                }
                true
            }
            KeyCode::Backspace => {
                self.storage.new_cloud_name.pop();
                true
            }
            KeyCode::Tab | KeyCode::BackTab => true,
            KeyCode::Char(c) => {
                self.storage.new_cloud_name.push(c);
                true
            }
            _ => false,
        }
    }

    fn handle_password_creation_input(&mut self, key: KeyCode) -> bool {
        let char_key = match key {
            KeyCode::Char(c) => c,
            KeyCode::Enter => '\n',
            KeyCode::Esc => '\x1b',
            KeyCode::Backspace => '\x08',
            KeyCode::Tab | KeyCode::BackTab => return true,
            _ => return false,
        };

        self.storage
            .password_creation
            .handle_password_input(char_key);

        // Check if password creation is complete
        if self
            .storage
            .password_creation
            .is_password_creation_complete()
        {
            self.complete_cloud_password_creation();
        }

        true
    }

    fn complete_folder_creation(&mut self) {
        let folder_name = self.storage.new_folder_name.trim().to_string();
        let folder_path = std::path::PathBuf::from(self.storage.new_folder_path.trim());

        if folder_name.is_empty() {
            self.storage.folder_creation_error = Some("Folder name cannot be empty".to_string());
            return;
        }

        if !folder_path.exists() {
            self.storage.folder_creation_error =
                Some(format!("Path '{}' does not exist", folder_path.display()));
            return;
        }

        if !folder_path.is_dir() {
            self.storage.folder_creation_error =
                Some(format!("'{}' is not a directory", folder_path.display()));
            return;
        }

        let folder = cloudhost_server::CloudFolder::new(folder_name.clone(), folder_path.clone());

        if let Err(e) = self.orchestrator.add_cloud_folder(folder) {
            self.storage.folder_creation_error = Some(e.to_string());
            return;
        }

        // Reload from orchestrator
        self.sync_from_orchestrator();

        self.add_debug(&format!(
            "Added folder '{}' at path {}",
            folder_name,
            folder_path.display()
        ));
        self.show_notice(
            NoticeKind::Success,
            format!("Added folder ‘{folder_name}’."),
        );
        self.storage.adding_folder = false;
        self.storage.folder_creation_error = None;
    }

    fn handle_folder_edit_input(&mut self, key: KeyCode) -> bool {
        match key {
            KeyCode::Esc => {
                self.storage.clear_folder_edit();
                true
            }
            KeyCode::Enter => {
                match self.storage.edit_folder_input_field {
                    storage::state::FolderInputField::Name => {
                        if !self.storage.edit_folder_name.is_empty() {
                            self.storage.edit_folder_input_field =
                                storage::state::FolderInputField::Path;
                        }
                    }
                    storage::state::FolderInputField::Path => {
                        if !self.storage.edit_folder_path.is_empty() {
                            self.complete_folder_edit();
                        }
                    }
                }
                true
            }
            KeyCode::Tab | KeyCode::BackTab => {
                self.storage.edit_folder_input_field = match self.storage.edit_folder_input_field {
                    storage::state::FolderInputField::Name => {
                        storage::state::FolderInputField::Path
                    }
                    storage::state::FolderInputField::Path => {
                        storage::state::FolderInputField::Name
                    }
                };
                true
            }
            KeyCode::Backspace => {
                match self.storage.edit_folder_input_field {
                    storage::state::FolderInputField::Name => {
                        self.storage.edit_folder_name.pop();
                    }
                    storage::state::FolderInputField::Path => {
                        self.storage.edit_folder_path.pop();
                    }
                }
                true
            }
            KeyCode::Char(c) => {
                match self.storage.edit_folder_input_field {
                    storage::state::FolderInputField::Name => {
                        self.storage.edit_folder_name.push(c);
                    }
                    storage::state::FolderInputField::Path => {
                        self.storage.edit_folder_path.push(c);
                    }
                }
                true
            }
            _ => false,
        }
    }

    fn handle_cloud_edit_input(&mut self, key: KeyCode) -> bool {
        match key {
            KeyCode::Esc => {
                self.storage.clear_cloud_edit();
                true
            }
            KeyCode::Enter => {
                if !self.storage.edit_cloud_name.is_empty() {
                    self.complete_cloud_edit();
                }
                true
            }
            KeyCode::Tab | KeyCode::BackTab => {
                // Toggle between name editing and folder navigation.
                self.storage.cloud_edit_focus = match self.storage.cloud_edit_focus {
                    crate::tabs::storage::state::CloudEditFocus::Name => {
                        crate::tabs::storage::state::CloudEditFocus::Folders
                    }
                    crate::tabs::storage::state::CloudEditFocus::Folders => {
                        crate::tabs::storage::state::CloudEditFocus::Name
                    }
                };
                true
            }
            KeyCode::Backspace => {
                if self.storage.cloud_edit_focus
                    == crate::tabs::storage::state::CloudEditFocus::Name
                {
                    self.storage.edit_cloud_name.pop();
                }
                true
            }
            KeyCode::Char(c) => {
                match self.storage.cloud_edit_focus {
                    crate::tabs::storage::state::CloudEditFocus::Name => {
                        if c.to_string() == self.config.leader {
                            // The leader key toggles folder selection in this modal.
                            if !self.storage.folders.is_empty() {
                                let current_index = self.storage.selected_folder_index;
                                self.storage.toggle_cloud_folder_selection(current_index);
                            }
                        } else {
                            self.storage.edit_cloud_name.push(c);
                        }
                        true
                    }
                    crate::tabs::storage::state::CloudEditFocus::Folders => {
                        let is_toggle_key = c.to_string() == self.config.leader
                            || self
                                .config
                                .get_keys_for_action("Toggle Selection")
                                .iter()
                                .any(|configured| configured == &c.to_string());
                        if is_toggle_key {
                            // Toggle the folder currently highlighted in the edit list.
                            if !self.storage.folders.is_empty() {
                                let current_index = self.storage.selected_folder_index;
                                self.storage.toggle_cloud_folder_selection(current_index);
                            }
                            true
                        } else if c == 'j' || c == 'k' {
                            // Handle j/k navigation
                            self.storage.handle_folders_navigation(key);
                            true
                        } else {
                            false // Let other keys be handled by navigation
                        }
                    }
                }
            }
            KeyCode::Up | KeyCode::Down
                if self.storage.cloud_edit_focus
                    == crate::tabs::storage::state::CloudEditFocus::Folders =>
            {
                // Allow navigation in the folder list within the cloud edit modal.
                self.storage.handle_folders_navigation(key);
                true
            }
            _ => false,
        }
    }

    fn complete_folder_edit(&mut self) {
        let new_name = self.storage.edit_folder_name.trim().to_string();
        let new_path = std::path::PathBuf::from(self.storage.edit_folder_path.trim());
        let old_name = self.storage.edit_folder_original_name.clone();

        if new_name.is_empty() {
            self.storage.folder_edit_error = Some("Folder name cannot be empty".to_string());
            return;
        }

        if !new_path.exists() {
            self.storage.folder_edit_error =
                Some(format!("Path '{}' does not exist", new_path.display()));
            return;
        }

        if !new_path.is_dir() {
            self.storage.folder_edit_error =
                Some(format!("'{}' is not a directory", new_path.display()));
            return;
        }

        let updated_folder = cloudhost_server::CloudFolder::new(new_name.clone(), new_path.clone());

        if let Err(e) = self
            .orchestrator
            .update_cloud_folder(&old_name, updated_folder)
        {
            self.storage.folder_edit_error = Some(e.to_string());
            return;
        }

        // Reload from orchestrator
        self.sync_from_orchestrator();

        self.add_debug(&format!("Updated folder '{}' to '{}'", old_name, new_name));
        self.show_notice(NoticeKind::Success, format!("Updated folder ‘{new_name}’."));
        self.storage.clear_folder_edit();
    }

    fn complete_cloud_edit(&mut self) {
        let new_name = self.storage.edit_cloud_name.trim().to_string();
        let old_name = self.storage.edit_cloud_original_name.clone();

        if new_name.is_empty() {
            self.storage.cloud_edit_error = Some("Cloud name cannot be empty".to_string());
            return;
        }

        let folder_names = self.storage.selected_cloud_folder_names();

        if folder_names.is_empty() {
            self.storage.cloud_edit_error = Some("Cloud must have at least one folder".to_string());
            return;
        }

        let folders: Vec<cloudhost_server::CloudFolder> = folder_names
            .iter()
            .filter_map(|name| {
                self.storage
                    .folders
                    .iter()
                    .find(|f| &f.name == name)
                    .map(|f| {
                        cloudhost_server::CloudFolder::new(f.name.clone(), f.folder_path.clone())
                    })
            })
            .collect();

        if folders.is_empty() {
            self.storage.cloud_edit_error = Some("No valid folders selected".to_string());
            return;
        }

        // Get the old cloud to preserve password and JWT secret
        let old_cloud = self.orchestrator.get_cloud(&old_name);

        let new_cloud = if let Some(old_cloud_data) = old_cloud {
            // Preserve password and JWT secret
            cloudhost_server::Cloud {
                name: new_name.clone(),
                cloud_folders: folders,
                password: old_cloud_data.password,
                password_changed_at: old_cloud_data.password_changed_at,
                jwt_secret: old_cloud_data.jwt_secret,
            }
        } else {
            cloudhost_server::Cloud::new(new_name.clone(), folders)
        };

        if let Err(e) = self.orchestrator.update_cloud(&old_name, new_cloud) {
            self.storage.cloud_edit_error = Some(e.to_string());
            return;
        }

        // Check if the updated cloud has a password set
        if !self.orchestrator.cloud_has_password(&new_name) {
            self.storage.cloud_edit_error = Some(
                "Cloud updated successfully, but no password is set. Please set a password before starting the cloud.".to_string()
            );
            return;
        }

        // Reload from orchestrator
        self.sync_from_orchestrator();

        self.add_debug(&format!("Updated cloud '{}' to '{}'", old_name, new_name));
        self.show_notice(NoticeKind::Success, format!("Updated cloud ‘{new_name}’."));
        self.storage.clear_cloud_edit();
    }

    pub fn next_tab(&mut self) {
        let old_tab = self.selected_tab;
        self.selected_tab = self.selected_tab.next();
        self.add_debug(&format!(
            "next_tab: {:?} -> {:?}",
            old_tab, self.selected_tab
        ));
    }

    pub fn previous_tab(&mut self) {
        let old_tab = self.selected_tab;
        self.selected_tab = self.selected_tab.previous();
        self.add_debug(&format!(
            "previous_tab: {:?} -> {:?}",
            old_tab, self.selected_tab
        ));
    }

    pub fn goto_tab(&mut self, index: usize) {
        let total_tabs = SelectedTab::iter().count();
        let wrapped_index = index % total_tabs;
        if let Some(tab) = SelectedTab::from_repr(wrapped_index) {
            self.selected_tab = tab;
        }
    }

    pub fn quit(&mut self) {
        self.state = AppState::Quitting;
    }

    pub fn show_notice(&mut self, kind: NoticeKind, message: impl Into<String>) {
        self.notice = Some(Notice {
            kind,
            message: message.into(),
            shown_at: std::time::Instant::now(),
        });
    }

    fn dismiss_notice_if_expired(&mut self) {
        if self
            .notice
            .as_ref()
            .is_some_and(|notice| notice.shown_at.elapsed().as_millis() > NOTICE_TIMEOUT_MS as u128)
        {
            self.notice = None;
        }
    }

    async fn handle_overlay_key(&mut self, key: KeyCode) -> bool {
        if self.help_open {
            if matches!(key, KeyCode::Esc | KeyCode::Char('?')) {
                self.help_open = false;
            }
            return true;
        }

        if self.pending_confirmation.is_some() {
            match key {
                KeyCode::Enter | KeyCode::Char('y' | 'Y') => self.confirm_pending_action().await,
                KeyCode::Esc | KeyCode::Char('n' | 'N') => {
                    self.pending_confirmation = None;
                    self.show_notice(NoticeKind::Info, "Action cancelled.");
                }
                _ => {}
            }
            return true;
        }

        false
    }

    async fn confirm_pending_action(&mut self) {
        let Some(confirmation) = self.pending_confirmation.take() else {
            return;
        };

        match confirmation.action {
            ConfirmationAction::RemoveFolder(name) => self.remove_folder(&name),
            ConfirmationAction::RemoveCloud(name) => {
                if self.orchestrator.is_cloud_running(&name) {
                    if let Err(error) = self.orchestrator.stop_cloud(&name).await {
                        self.show_notice(
                            NoticeKind::Error,
                            format!("Could not stop ‘{name}’ before removing it: {error}"),
                        );
                        return;
                    }
                    self.dashboard.running_clouds.remove(&name);
                    self.add_debug(&format!("Stopped cloud '{name}' before removal"));
                }
                self.remove_cloud(&name);
            }
            ConfirmationAction::ResetTuiConfig => match crate::config::Config::reset_to_default() {
                Ok(()) => {
                    self.config = crate::config::Config::load_or_default();
                    self.show_notice(
                        NoticeKind::Success,
                        "Restored the default TUI configuration.",
                    );
                }
                Err(error) => self.show_notice(
                    NoticeKind::Error,
                    format!("Could not reset the TUI configuration: {error}"),
                ),
            },
        }
    }

    pub fn toggle_debug(&mut self) {
        self.debug_mode = !self.debug_mode;
        if self.debug_mode {
            self.add_debug("Debug mode enabled");
        } else {
            self.debug_info.clear();
        }
    }

    pub fn add_debug(&mut self, message: &str) {
        if self.debug_mode {
            self.debug_info.push(format!(
                "[{}] {}",
                chrono::Local::now().format("%H:%M:%S%.3f"),
                message
            ));
            // Keep only last 10 debug messages
            if self.debug_info.len() > 10 {
                self.debug_info.remove(0);
            }
        }
    }

    pub async fn refresh_activity(&mut self) {
        let previous_count = self.activity.len();
        let searching_activity = self.is_searching(SearchTarget::DashboardActivity);
        let selected_cloud = self
            .dashboard
            .clouds
            .get(self.dashboard.selected_cloud_index)
            .map(|cloud| cloud.name.clone());
        let cloud_changed = self.activity_cloud != selected_cloud;
        let follow_latest = cloud_changed
            || self
                .dashboard
                .activity_list_state
                .selected()
                .is_none_or(|selected| selected >= previous_count.saturating_sub(1));

        self.activity = match selected_cloud.as_deref() {
            Some(cloud_name) => self.orchestrator.get_cloud_debug_logs(cloud_name).await,
            None => Vec::new(),
        };
        self.activity_cloud = selected_cloud;

        if self.activity.len() > 100 {
            self.activity.drain(0..self.activity.len() - 100);
        }

        if !searching_activity {
            if self.activity.is_empty() {
                self.dashboard.activity_list_state.select(None);
            } else if follow_latest {
                self.dashboard
                    .activity_list_state
                    .select(Some(self.activity.len() - 1));
            } else if let Some(selected) = self.dashboard.activity_list_state.selected() {
                self.dashboard
                    .activity_list_state
                    .select(Some(selected.min(self.activity.len() - 1)));
            }
        }
    }

    // Tab-specific focus management
    pub fn cycle_focus_forward(&mut self) {
        match self.selected_tab {
            SelectedTab::Dashboard => self.dashboard.cycle_focus_forward(),
            SelectedTab::Storage => self.storage.cycle_focus_forward(),
            SelectedTab::Settings => self.settings.cycle_focus_forward(),
        }
    }

    pub fn cycle_focus_backward(&mut self) {
        match self.selected_tab {
            SelectedTab::Dashboard => self.dashboard.cycle_focus_backward(),
            SelectedTab::Storage => self.storage.cycle_focus_backward(),
            SelectedTab::Settings => self.settings.cycle_focus_backward(),
        }
    }

    pub fn get_current_focused_element(&self) -> String {
        match self.selected_tab {
            SelectedTab::Dashboard => self.dashboard.get_focused_element(),
            SelectedTab::Storage => self.storage.get_focused_element(),
            SelectedTab::Settings => self.settings.get_focused_element(),
        }
    }

    async fn complete_dashboard_password_creation(&mut self) {
        if self.dashboard.clouds.is_empty()
            || self.dashboard.selected_cloud_index >= self.dashboard.clouds.len()
        {
            self.dashboard.password_creation.password_error = Some("No cloud selected".to_string());
            return;
        }

        let cloud_name = self.dashboard.clouds[self.dashboard.selected_cloud_index]
            .name
            .clone();

        let password = self.dashboard.password_creation.get_password().to_string();

        if let Err(e) = self
            .dashboard
            .set_password(&mut self.orchestrator, &password)
        {
            self.dashboard.password_creation.password_error = Some(e);
        } else {
            self.dashboard.clear_password_creation();
            self.dashboard.password_creation.password_success = true;
            // Clear any server start errors since password is now set
            self.dashboard.cloud_start_error = None;
            self.add_debug(&format!(
                "Password set successfully for cloud '{}'",
                cloud_name
            ));
            self.show_notice(
                NoticeKind::Success,
                format!("Password updated for cloud ‘{cloud_name}’."),
            );

            // If server is running, restart it to pick up the new AuthState
            let was_running = self.dashboard.running_clouds.contains_key(&cloud_name);

            if was_running {
                self.add_debug("Stopping server to apply password changes");
                self.dashboard.stop_server(&mut self.orchestrator).await;
            }

            // Recreate the orchestrator instance to pick up the new config
            self.add_debug("Recreating orchestrator instance with new config");
            // Orchestrator is already initialized in App
            // Reload clouds from orchestrator
            self.sync_from_orchestrator();

            // If server was running, restart it automatically
            if was_running {
                self.add_debug("Restarting server with new password");
                self.dashboard.start_server(&mut self.orchestrator).await;
                self.add_debug("Server restart initiated with new password");
            }
        }
    }

    // Tab-specific navigation methods
    pub fn handle_tab_navigation(&mut self, key: ratatui::crossterm::event::KeyCode) -> bool {
        match self.selected_tab {
            SelectedTab::Dashboard
                if self.dashboard.focused_panel == dashboard::state::DashboardPanel::Activity =>
            {
                self.dashboard
                    .handle_activity_navigation(key, self.activity.len())
            }
            SelectedTab::Dashboard => self.dashboard.handle_navigation(key),
            SelectedTab::Storage => self.storage.handle_navigation(key),
            SelectedTab::Settings => self.settings.handle_navigation(key),
        }
    }

    pub fn reload_tui_config(&mut self) -> Result<(), String> {
        self.config = crate::config::Config::load().map_err(|error| error.to_string())?;
        Ok(())
    }

    pub async fn reload_storage_config(&mut self) -> Result<(), String> {
        self.orchestrator
            .reload_config()
            .await
            .map_err(|error| error.to_string())?;
        self.sync_from_orchestrator();
        Ok(())
    }

    pub async fn reload_all_configs(&mut self) -> Result<(), String> {
        self.reload_tui_config()?;
        self.reload_storage_config().await
    }

    async fn execute_settings_action(&mut self) {
        use crate::tabs::settings::state::SettingsAction;

        match self.settings.selected_action() {
            SettingsAction::OpenTuiConfig => {
                let path = cloudhost_server::config_paths::get_tui_config_path();
                let result = if path.exists() {
                    Ok(())
                } else {
                    self.config
                        .save_to_file()
                        .map_err(|error| error.to_string())
                }
                .and_then(|()| {
                    crate::external::open_config(&path).map_err(|error| error.to_string())
                });
                match result {
                    Ok(()) => self.show_notice(NoticeKind::Info, "Opened the TUI configuration."),
                    Err(error) => self.show_notice(
                        NoticeKind::Error,
                        format!("Could not open {}: {error}", path.display()),
                    ),
                }
            }
            SettingsAction::OpenStorageConfig => {
                let path = cloudhost_server::config_paths::get_clouds_config_path();
                let result = if path.exists() {
                    Ok(())
                } else {
                    self.orchestrator
                        .clouds_config
                        .save_to_file()
                        .map_err(|error| error.to_string())
                }
                .and_then(|()| {
                    crate::external::open_config(&path).map_err(|error| error.to_string())
                });
                match result {
                    Ok(()) => {
                        self.show_notice(NoticeKind::Info, "Opened the storage configuration.")
                    }
                    Err(error) => self.show_notice(
                        NoticeKind::Error,
                        format!("Could not open {}: {error}", path.display()),
                    ),
                }
            }
            SettingsAction::ReloadTuiConfig => match self.reload_tui_config() {
                Ok(()) => self.show_notice(NoticeKind::Success, "Reloaded TUI configuration."),
                Err(error) => self.show_notice(
                    NoticeKind::Error,
                    format!("Could not reload TUI configuration: {error}"),
                ),
            },
            SettingsAction::ReloadStorageConfig => match self.reload_storage_config().await {
                Ok(()) => self.show_notice(NoticeKind::Success, "Reloaded storage configuration."),
                Err(error) => self.show_notice(
                    NoticeKind::Error,
                    format!("Could not reload storage configuration: {error}"),
                ),
            },
            SettingsAction::ReloadAllConfigs => match self.reload_all_configs().await {
                Ok(()) => self.show_notice(NoticeKind::Success, "Reloaded all configuration."),
                Err(error) => self.show_notice(
                    NoticeKind::Error,
                    format!("Could not reload configuration: {error}"),
                ),
            },
            SettingsAction::ResetTuiConfig => {
                self.pending_confirmation = Some(Confirmation {
                    title: "Reset TUI configuration?".to_string(),
                    description: "Your custom keybindings and TUI preferences will be replaced with CloudHost defaults.".to_string(),
                    confirm_label: "reset configuration".to_string(),
                    action: ConfirmationAction::ResetTuiConfig,
                });
            }
        }
    }

    pub async fn handle_dynamic_key(
        &mut self,
        key: ratatui::crossterm::event::KeyCode,
        modifiers: ratatui::crossterm::event::KeyModifiers,
    ) {
        if self.handle_overlay_key(key).await {
            return;
        }

        // Search owns all input until it is confirmed or cancelled, so query
        // characters cannot accidentally trigger a normal shortcut.
        if self.handle_list_search_key(key) {
            return;
        }

        if key == KeyCode::Char('/') && self.start_list_search() {
            return;
        }

        // Convert key to string for config lookup
        let key_str = match key {
            KeyCode::Char(c) => {
                // Check for Ctrl combinations
                if c.is_ascii_control() {
                    let ctrl_char = (c as u8 + 96) as char; // Convert control char to letter
                    format!("<Ctrl>{}", ctrl_char)
                } else {
                    c.to_string()
                }
            }
            KeyCode::Up => "<Up>".to_string(),
            KeyCode::Down => "<Down>".to_string(),
            KeyCode::Left => "<Left>".to_string(),
            KeyCode::Right => "<Right>".to_string(),
            KeyCode::Enter => "<Enter>".to_string(),
            KeyCode::Esc => "<Esc>".to_string(),
            KeyCode::Backspace => "<Backspace>".to_string(),
            KeyCode::Tab => {
                // Check for Shift+Tab
                if modifiers.contains(ratatui::crossterm::event::KeyModifiers::SHIFT) {
                    "<S-Tab>".to_string()
                } else {
                    "<Tab>".to_string()
                }
            }
            KeyCode::BackTab => "<S-Tab>".to_string(),
            _ => return,
        };

        // Get current tab name
        let current_tab = match self.selected_tab {
            SelectedTab::Dashboard => "dashboard",
            SelectedTab::Storage => "storage",
            SelectedTab::Settings => "settings",
        };

        self.add_debug(&format!("Key: {} -> tab: {}", key_str, current_tab));
        self.add_debug(&format!("Input state: {:?}", self.input_state));

        // Modal input takes precedence over global shortcuts.
        if self.dashboard.password_creation.creating_password {
            let char_key = match key {
                KeyCode::Char(c) => c,
                KeyCode::Enter => '\n',
                KeyCode::Esc => '\x1b',
                KeyCode::Backspace => '\x08',
                _ => return,
            };
            self.dashboard.handle_password_input(char_key);
            // If password creation is complete, handle it
            if self
                .dashboard
                .password_creation
                .is_password_creation_complete()
            {
                self.complete_dashboard_password_creation().await;
            }
            return;
        }

        // Handle folder creation modal.
        if self.storage.adding_folder && self.handle_folder_creation_input(key) {
            return;
        }

        // Handle cloud creation modal
        if self.storage.creating_cloud && self.handle_cloud_creation_input(key) {
            return;
        }

        // Handle password creation modal (for existing clouds)
        if self.storage.password_creation.creating_password
            && !self.storage.creating_cloud
            && self.handle_password_creation_input(key)
        {
            return;
        }

        // Handle folder edit modal.
        if self.storage.editing_folder && self.handle_folder_edit_input(key) {
            return;
        }

        // Handle cloud edit modal
        if self.storage.editing_cloud && self.handle_cloud_edit_input(key) {
            return;
        }

        // Handle leader key sequences first
        if key_str == self.config.leader {
            self.input_state =
                InputState::KeySequence("<leader>".to_string(), std::time::Instant::now());
            self.add_debug(&format!("Leader key ('{}') pressed", key_str));
            return;
        }

        // Handle multi-key sequences (like gt, gT, <leader>d, ft, etc.)
        if let InputState::KeySequence(ref seq, start_time) = self.input_state {
            // Check if the sequence has timed out
            if start_time.elapsed().as_millis() > KEY_SEQUENCE_TIMEOUT_MS as u128 {
                // Timeout reached, execute the single key if it exists
                if let Some(action) = self.config.get_action_for_key(seq, current_tab) {
                    self.execute_action(&action).await;
                }
                self.input_state = InputState::Normal;
                return;
            }

            if seq == "<leader>" {
                let leader_key = format!("<leader>{}", key_str);
                if let Some(action) = self.config.get_action_for_key(&leader_key, current_tab) {
                    self.execute_action(&action).await;
                }
                self.input_state = InputState::Normal;
                return;
            } else {
                // Try to complete the sequence with the current key
                let complete_key = format!("{}{}", seq, key_str);
                if let Some(action) = self.config.get_action_for_key(&complete_key, current_tab) {
                    self.execute_action(&action).await;
                }
                self.input_state = InputState::Normal;
                return;
            }
        }

        // Check if this key could start a multi-key sequence
        // Look for any keybinding that starts with this key
        // Skip special keys that are complete by themselves (like <Up>, <Down>, <Enter>)
        // but allow multi-key special keys (like <Ctrl>c, <Alt>f, etc.)
        let potential_sequences: Vec<String> = self
            .config
            .actions
            .values()
            .filter(|action| action.applies_to(current_tab))
            .flat_map(|action| &action.keys)
            .filter(|k| {
                k.starts_with(&key_str)
                    && k.len() > 1
                    && // Skip if it's a single special key (starts with <, ends with >, and doesn't contain another <)
                    !(k.starts_with('<') && k.ends_with('>') && !k[1..k.len() - 1].contains('<'))
            })
            .cloned()
            .collect();

        if !potential_sequences.is_empty() {
            self.input_state = InputState::KeySequence(key_str.clone(), std::time::Instant::now());
            self.add_debug(&format!(
                "Started '{}' sequence, potential: {:?}",
                key_str, potential_sequences
            ));
            return;
        }

        if let Some(action) = self.config.get_action_for_key(&key_str, current_tab) {
            self.execute_action(&action).await;
        } else {
            self.add_debug(&format!("No keybinding found for key '{}'", key_str));
        }
    }

    async fn execute_action(&mut self, action: &str) {
        match action {
            "Quit" => self.quit(),
            "Next Tab" => self.next_tab(),
            "Previous Tab" => self.previous_tab(),
            "Toggle Debug" => self.toggle_debug(),
            "Toggle Help" => self.help_open = !self.help_open,
            "Start/Stop Cloud" => {
                let cloud_name = self
                    .dashboard
                    .clouds
                    .get(self.dashboard.selected_cloud_index)
                    .map(|cloud| cloud.name.clone());
                if let Some(cloud_name) = cloud_name {
                    let is_running = self.dashboard.is_cloud_running(&cloud_name);
                    if is_running {
                        self.dashboard.stop_server(&mut self.orchestrator).await;
                    } else {
                        self.dashboard.start_server(&mut self.orchestrator).await;
                    }

                    if let Some(error) = self.dashboard.cloud_start_error.clone() {
                        self.show_notice(NoticeKind::Error, error);
                    } else if is_running {
                        self.show_notice(NoticeKind::Info, format!("Stopped ‘{cloud_name}’."));
                    } else {
                        self.show_notice(NoticeKind::Success, format!("Started ‘{cloud_name}’."));
                    }
                } else {
                    self.show_notice(NoticeKind::Warning, "Create and select a cloud first.");
                }
            }
            "Open Cloud" => {
                let cloud_name = self
                    .dashboard
                    .clouds
                    .get(self.dashboard.selected_cloud_index)
                    .map(|cloud| cloud.name.clone());
                match cloud_name
                    .as_deref()
                    .and_then(|name| self.orchestrator.get_cloud_server_url(name))
                {
                    Some(url) => match crate::external::open_detached(&url) {
                        Ok(()) => self.show_notice(NoticeKind::Info, format!("Opened {url}")),
                        Err(error) => self.show_notice(
                            NoticeKind::Error,
                            format!("Could not open {url}: {error}"),
                        ),
                    },
                    None => self.show_notice(
                        NoticeKind::Warning,
                        "Start the selected cloud before opening it in a browser.",
                    ),
                }
            }
            "Add Folder" => {
                self.start_adding_folder();
            }
            "Create Cloud" => {
                self.start_creating_cloud();
            }
            "Remove Folder / Cloud" => {
                self.request_remove_focused_item();
            }
            "Set Cloud Password" => {
                self.start_setting_cloud_password();
            }
            "Select All Folders" => {
                self.select_all_folders();
            }
            "Edit" => {
                self.start_editing_storage_item();
            }
            "Toggle Password Visibility" => {
                self.toggle_cloud_password_display();
            }
            "Cycle Focus Forward" => self.cycle_focus_forward(),
            "Cycle Focus Backward" => self.cycle_focus_backward(),
            "Navigate Up" => {
                self.handle_tab_navigation(ratatui::crossterm::event::KeyCode::Char('k'));
            }
            "Navigate Down" => {
                self.handle_tab_navigation(ratatui::crossterm::event::KeyCode::Char('j'));
            }
            "Navigate to Top" => {
                self.handle_tab_navigation(ratatui::crossterm::event::KeyCode::Char('g'));
            }
            "Navigate to Bottom" => {
                self.handle_tab_navigation(ratatui::crossterm::event::KeyCode::Char('G'));
            }
            "Toggle Selection" => {
                if self.selected_tab == SelectedTab::Storage
                    && self.storage.focused_panel
                        == crate::tabs::storage::state::StoragePanel::Folders
                    && !self.storage.folders.is_empty()
                {
                    self.storage
                        .toggle_folder_selection(self.storage.selected_folder_index);
                    self.add_debug(&format!(
                        "Toggled selection for folder at index {}",
                        self.storage.selected_folder_index
                    ));
                }
            }
            "Refresh/Reload" => {
                // Reload data from orchestrator
                self.sync_from_orchestrator();
                self.add_debug("Refreshed data from orchestrator");
                self.show_notice(NoticeKind::Success, "Refreshed cloud data.");
            }
            "Reload TUI Config" => match self.reload_tui_config() {
                Ok(()) => self.show_notice(NoticeKind::Success, "Reloaded TUI configuration."),
                Err(error) => self.show_notice(
                    NoticeKind::Error,
                    format!("Could not reload TUI configuration: {error}"),
                ),
            },
            "Reload Storage Config" => match self.reload_storage_config().await {
                Ok(()) => self.show_notice(NoticeKind::Success, "Reloaded storage configuration."),
                Err(error) => self.show_notice(
                    NoticeKind::Error,
                    format!("Could not reload storage configuration: {error}"),
                ),
            },
            "Reload All Configs" => match self.reload_all_configs().await {
                Ok(()) => self.show_notice(NoticeKind::Success, "Reloaded all configuration."),
                Err(error) => self.show_notice(
                    NoticeKind::Error,
                    format!("Could not reload configuration: {error}"),
                ),
            },
            "Execute Action" => {
                if self.selected_tab == SelectedTab::Settings {
                    self.execute_settings_action().await;
                }
            }
            _ => {
                self.add_debug(&format!("Unknown action: {}", action));
            }
        }
    }

    /// Check if any pending key sequences have timed out and execute them
    pub async fn check_timeouts(&mut self) {
        self.dismiss_notice_if_expired();

        if let InputState::KeySequence(ref seq, start_time) = self.input_state {
            if start_time.elapsed().as_millis() > KEY_SEQUENCE_TIMEOUT_MS as u128 {
                // Timeout reached, execute the single key if it exists
                let current_tab = match self.selected_tab {
                    SelectedTab::Dashboard => "dashboard",
                    SelectedTab::Storage => "storage",
                    SelectedTab::Settings => "settings",
                };
                if let Some(action) = self.config.get_action_for_key(seq, current_tab) {
                    self.execute_action(&action).await;
                }
                self.input_state = InputState::Normal;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{backend::TestBackend, Terminal};

    fn screen_text(terminal: &Terminal<TestBackend>) -> String {
        terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect()
    }

    #[test]
    fn app_renders_each_dashboard_in_a_compact_terminal() {
        let backend = TestBackend::new(62, 18);
        let mut terminal = Terminal::new(backend).expect("test terminal should initialize");
        let mut app = App::default();

        for tab in [
            SelectedTab::Dashboard,
            SelectedTab::Storage,
            SelectedTab::Settings,
        ] {
            app.selected_tab = tab;
            terminal
                .draw(|frame| frame.render_widget(&mut app, frame.area()))
                .expect("dashboard should render");
        }

        app.help_open = true;
        terminal
            .draw(|frame| frame.render_widget(&mut app, frame.area()))
            .expect("help overlay should render");

        app.help_open = false;
        app.pending_confirmation = Some(Confirmation {
            title: "Confirm".to_string(),
            description: "A destructive action needs a clear confirmation.".to_string(),
            confirm_label: "continue".to_string(),
            action: ConfirmationAction::ResetTuiConfig,
        });
        terminal
            .draw(|frame| frame.render_widget(&mut app, frame.area()))
            .expect("confirmation overlay should render");
    }

    #[test]
    fn tabs_express_distinct_runtime_and_storage_concepts() {
        let backend = TestBackend::new(100, 26);
        let mut terminal = Terminal::new(backend).expect("test terminal should initialize");
        let mut app = App {
            selected_tab: SelectedTab::Dashboard,
            ..App::default()
        };
        terminal
            .draw(|frame| frame.render_widget(&mut app, frame.area()))
            .expect("dashboard should render");
        let dashboard = screen_text(&terminal);
        assert!(dashboard.contains("Dashboard"));
        assert!(dashboard.contains("Services"));
        assert!(dashboard.contains("Overview"));
        assert!(dashboard.contains("Activity"));

        app.selected_tab = SelectedTab::Storage;
        terminal
            .draw(|frame| frame.render_widget(&mut app, frame.area()))
            .expect("storage should render");
        let storage = screen_text(&terminal);
        assert!(storage.contains("Storage"));
        assert!(storage.contains("Folders"));
        assert!(storage.contains("Clouds"));
        assert!(storage.contains("Folder details"));

        app.selected_tab = SelectedTab::Settings;
        terminal
            .draw(|frame| frame.render_widget(&mut app, frame.area()))
            .expect("settings should render");
        let settings = screen_text(&terminal);
        assert!(settings.contains("Open storage config"));
    }

    #[test]
    fn folder_search_filters_live_and_commits_only_on_enter() {
        let mut app = App::default();
        app.selected_tab = SelectedTab::Storage;
        app.storage.folders = vec![
            cloudhost_server::CloudFolder::new("Archive".to_string(), "/tmp/archive".into()),
            cloudhost_server::CloudFolder::new("Reports".to_string(), "/tmp/reports".into()),
            cloudhost_server::CloudFolder::new("Photos".to_string(), "/tmp/photos".into()),
        ];
        app.storage.selected_folder_index = 0;
        app.storage.folders_list_state.select(Some(0));

        assert!(app.start_list_search());
        for character in ['r', 'e', 'p'] {
            assert!(app.handle_list_search_key(KeyCode::Char(character)));
        }

        assert_eq!(app.filtered_indices(SearchTarget::StorageFolders), vec![1]);
        assert_eq!(
            app.search_selected_index(SearchTarget::StorageFolders),
            Some(1)
        );
        assert_eq!(app.storage.selected_folder_index, 0);

        assert!(app.handle_list_search_key(KeyCode::Enter));
        assert!(app.search.is_none());
        assert_eq!(app.storage.selected_folder_index, 1);
        assert_eq!(app.storage.folders_list_state.selected(), Some(1));
    }

    #[test]
    fn cancelling_a_search_restores_the_previous_activity_selection() {
        use chrono::Utc;
        use cloudhost_server::debug_stream::{DebugMessage, LogLevel};

        let mut app = App::default();
        app.selected_tab = SelectedTab::Dashboard;
        app.dashboard.focused_panel = dashboard::state::DashboardPanel::Activity;
        app.activity = vec![
            DebugMessage {
                timestamp: Utc::now(),
                level: LogLevel::Info,
                source: "server".to_string(),
                message: "cloud started".to_string(),
            },
            DebugMessage {
                timestamp: Utc::now(),
                level: LogLevel::Warning,
                source: "server".to_string(),
                message: "disk space low".to_string(),
            },
        ];
        app.dashboard.activity_list_state.select(Some(0));

        assert!(app.start_list_search());
        for character in "disk".chars() {
            assert!(app.handle_list_search_key(KeyCode::Char(character)));
        }
        assert_eq!(
            app.filtered_indices(SearchTarget::DashboardActivity),
            vec![1]
        );

        assert!(app.handle_list_search_key(KeyCode::Esc));
        assert!(app.search.is_none());
        assert_eq!(app.dashboard.activity_list_state.selected(), Some(0));
    }
}
