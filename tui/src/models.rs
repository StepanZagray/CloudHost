use ratatui::{
    buffer::Buffer,
    crossterm::event::KeyCode,
    layout::Rect,
    prelude::Stylize,
    text::Line,
    widgets::{Tabs, Widget},
};
use strum::IntoEnumIterator;

use crate::tabs::{clouds, focus::TabFocus, folders, settings, SelectedTab};
use cloudhost_server::debug_stream::DebugMessage;

// Timeout for key sequences (like Vim's timeoutlen)
const KEY_SEQUENCE_TIMEOUT_MS: u64 = 1000; // 1 second

#[derive(Default)]
pub struct App {
    pub state: AppState,
    pub selected_tab: SelectedTab,
    pub config: crate::config::Config,
    pub input_state: InputState,
    pub pending_number: Option<usize>,
    pub debug_mode: bool,
    pub debug_info: Vec<String>,
    pub cloud_logs: Vec<DebugMessage>,
    pub debug_receiver:
        Option<std::sync::Arc<std::sync::Mutex<Vec<cloudhost_server::debug_stream::DebugMessage>>>>,

    // Shared orchestrator instance - owns all cloud/folder/server management
    pub orchestrator: cloudhost_server::Orchestrator,

    // Tab states
    pub clouds_state: clouds::models::CloudsState,
    pub folders_state: folders::models::FoldersState,
    pub settings_state: settings::models::SettingsState,
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
    NumberPrefix,
    KeySequence(String, std::time::Instant), // Stores the current key sequence and when it started
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
            clouds_state: clouds::models::CloudsState::new(),
            folders_state: folders::models::FoldersState::default(),
            settings_state: settings::models::SettingsState::new(),
            debug_receiver: None,
            ..Default::default()
        };

        // Load folders and clouds from orchestrator into the folders state
        app.load_folders_from_orchestrator();

        app
    }

    fn load_folders_from_orchestrator(&mut self) {
        self.folders_state.cloud_folders = self.orchestrator.get_cloud_folders();
        self.folders_state.clouds = self.orchestrator.get_clouds();
        // Also update clouds state
        self.clouds_state.clouds = self.orchestrator.get_clouds();
    }

    fn start_creating_folder(&mut self) {
        if self.selected_tab == SelectedTab::Folders {
            self.folders_state.creating_folder = true;
            self.folders_state.new_folder_name.clear();
            self.folders_state.new_folder_path.clear();
            self.folders_state.folder_input_field = folders::models::FolderInputField::Name;
            self.folders_state.folder_creation_error = None;
        }
    }

    fn start_creating_folder_or_cloud(&mut self) {
        if self.selected_tab == SelectedTab::Folders {
            match self.folders_state.focused_panel {
                folders::models::FocusedPanel::Folders => {
                    self.start_creating_folder();
                }
                folders::models::FocusedPanel::Clouds => {
                    let selected_count = self.folders_state.get_selected_folders_count();
                    if selected_count == 0 {
                        self.folders_state.cloud_creation_error = Some(
                            "❌ Cannot create cloud: No folders selected.\n💡 To create a cloud:\n   1. Focus on the Folders panel (use Tab or 'h')\n   2. Select folders using Space bar\n   3. Return to Clouds panel (use Tab or 'l')\n   4. Press 'n' to create a new cloud".to_string()
                        );
                        return;
                    }
                    self.folders_state.start_creating_cloud();
                }
                folders::models::FocusedPanel::Info => {
                    // Info panel doesn't support creation
                    self.add_debug(
                        "Cannot create from info panel. Focus on folders or groups panel first.",
                    );
                }
            }
        }
    }

    fn delete_selected_folder(&mut self) {
        if self.selected_tab == SelectedTab::Folders
            && !self.folders_state.cloud_folders.is_empty()
            && self.folders_state.selected_folder_index < self.folders_state.cloud_folders.len()
        {
            let folder_name = self.folders_state.cloud_folders
                [self.folders_state.selected_folder_index]
                .name
                .clone();

            if let Err(e) = self.orchestrator.remove_cloud_folder(&folder_name) {
                self.add_debug(&format!("Failed to remove folder: {}", e));
                return;
            }

            // Reload from orchestrator
            self.load_folders_from_orchestrator();

            // Adjust selected index if needed
            if self.folders_state.selected_folder_index >= self.folders_state.cloud_folders.len()
                && !self.folders_state.cloud_folders.is_empty()
            {
                self.folders_state.selected_folder_index =
                    self.folders_state.cloud_folders.len() - 1;
            }

            self.add_debug(&format!("Deleted folder '{}'", folder_name));
        }
    }

    fn delete_selected_cloud(&mut self) {
        if self.selected_tab == SelectedTab::Folders
            && !self.folders_state.clouds.is_empty()
            && self.folders_state.selected_cloud_index < self.folders_state.clouds.len()
        {
            let cloud_name = self.folders_state.clouds[self.folders_state.selected_cloud_index]
                .name
                .clone();

            if let Err(e) = self.orchestrator.remove_cloud(&cloud_name) {
                self.add_debug(&format!("Failed to remove cloud: {}", e));
                return;
            }

            // Reload from orchestrator
            self.load_folders_from_orchestrator();

            // Adjust selected index if needed
            if self.folders_state.selected_cloud_index >= self.folders_state.clouds.len()
                && !self.folders_state.clouds.is_empty()
            {
                self.folders_state.selected_cloud_index = self.folders_state.clouds.len() - 1;
            }

            self.add_debug(&format!("Deleted cloud '{}'", cloud_name));
        }
    }

    fn start_setting_cloud_password(&mut self) {
        if self.selected_tab == SelectedTab::Folders
            && !self.folders_state.clouds.is_empty()
            && self.folders_state.selected_cloud_index < self.folders_state.clouds.len()
        {
            self.folders_state
                .password_creation
                .start_creating_password();
        }
    }

    fn toggle_cloud_password_display(&mut self) {
        if self.selected_tab == SelectedTab::Folders
            && self.folders_state.focused_panel
                == crate::tabs::folders::models::FocusedPanel::Clouds
            && !self.folders_state.clouds.is_empty()
            && self.folders_state.selected_cloud_index < self.folders_state.clouds.len()
        {
            self.folders_state.toggle_password_display();
            let new_state = match self.folders_state.password_display_state {
                crate::tabs::folders::models::PasswordDisplayState::Hidden => "hidden",
                crate::tabs::folders::models::PasswordDisplayState::Visible => "visible",
            };
            self.add_debug(&format!("Password display toggled to: {}", new_state));
        }
    }

    fn select_all_folders(&mut self) {
        if self.selected_tab == SelectedTab::Folders {
            self.folders_state.select_all_folders();
            let count = self.folders_state.get_selected_folders_count();
            self.add_debug(&format!("Selected all {} folders", count));
        }
    }

    fn start_editing(&mut self) {
        if self.selected_tab == SelectedTab::Folders {
            match self.folders_state.focused_panel {
                folders::models::FocusedPanel::Folders => {
                    if !self.folders_state.cloud_folders.is_empty() {
                        self.folders_state.start_editing_folder();
                    } else {
                        self.add_debug("No folders to edit");
                    }
                }
                folders::models::FocusedPanel::Clouds => {
                    if !self.folders_state.clouds.is_empty() {
                        self.folders_state.start_editing_cloud();
                    } else {
                        self.add_debug("No clouds to edit");
                    }
                }
                folders::models::FocusedPanel::Info => {
                    self.add_debug("Cannot edit from info panel");
                }
            }
        }
    }
    fn complete_cloud_creation(&mut self) {
        if self.selected_tab == SelectedTab::Folders && self.folders_state.creating_cloud {
            let cloud_name = self.folders_state.new_cloud_name.trim().to_string();

            if cloud_name.is_empty() {
                self.folders_state.cloud_creation_error =
                    Some("Cloud name cannot be empty".to_string());
                return;
            }

            let selected_folder_names = self.folders_state.get_selected_folder_names();

            if selected_folder_names.is_empty() {
                self.folders_state.cloud_creation_error = Some("⚠️  No cloud folders selected! Please select at least one cloud folder using the <leader> key before creating a cloud.".to_string());
                return;
            }

            // Build cloud from selected folders
            let folders: Vec<cloudhost_server::CloudFolder> = selected_folder_names
                .iter()
                .filter_map(|name| {
                    self.folders_state
                        .cloud_folders
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
                self.folders_state.cloud_creation_error =
                    Some("No valid folders selected".to_string());
                return;
            }

            let cloud = cloudhost_server::Cloud::new(cloud_name.clone(), folders);

            if let Err(e) = self.orchestrator.add_cloud(cloud) {
                self.folders_state.cloud_creation_error = Some(e.to_string());
                return;
            }

            // Reload from orchestrator to update the cloud list
            self.load_folders_from_orchestrator();

            self.add_debug(&format!(
                "Created cloud '{}' with {} folders",
                cloud_name,
                self.folders_state.get_selected_folders_count()
            ));

            // Clear folder selections after creating cloud
            self.folders_state.clear_folder_selections();

            // Start password creation for the new cloud using shared component
            self.folders_state
                .password_creation
                .start_creating_password();
        }
    }

    fn complete_cloud_password_creation(&mut self) {
        if self.selected_tab == SelectedTab::Folders
            && self
                .folders_state
                .password_creation
                .is_password_creation_complete()
        {
            let password = self
                .folders_state
                .password_creation
                .get_password()
                .to_string();

            // Determine if this is for a new cloud or existing cloud
            let cloud_name = if !self.folders_state.new_cloud_name.is_empty() {
                // New cloud creation
                self.folders_state.new_cloud_name.trim().to_string()
            } else if self.folders_state.selected_cloud_index < self.folders_state.clouds.len() {
                // Existing cloud password setting
                self.folders_state.clouds[self.folders_state.selected_cloud_index]
                    .name
                    .clone()
            } else {
                return; // No valid cloud
            };

            // Set the password for the cloud
            if let Err(e) = self.orchestrator.set_cloud_password(&cloud_name, &password) {
                self.folders_state.password_creation.password_error = Some(e.to_string());
                return;
            }

            self.add_debug(&format!(
                "Password set successfully for cloud '{}'",
                cloud_name
            ));

            // Clear password creation state
            self.folders_state
                .password_creation
                .clear_password_creation();

            // If this was for a new cloud, clear cloud creation state completely
            if !self.folders_state.new_cloud_name.is_empty() {
                self.folders_state.clear_cloud_creation();
            }
        }
    }

    fn handle_folder_creation_input(&mut self, key: KeyCode) -> bool {
        match key {
            KeyCode::Esc => {
                self.folders_state.creating_folder = false;
                self.folders_state.folder_creation_error = None;
                true
            }
            KeyCode::Enter => {
                match self.folders_state.folder_input_field {
                    folders::models::FolderInputField::Name => {
                        if !self.folders_state.new_folder_name.is_empty() {
                            self.folders_state.folder_input_field =
                                folders::models::FolderInputField::Path;
                        }
                    }
                    folders::models::FolderInputField::Path => {
                        if !self.folders_state.new_folder_path.is_empty() {
                            self.complete_folder_creation();
                        }
                    }
                }
                true
            }
            KeyCode::Tab => {
                self.folders_state.folder_input_field = match self.folders_state.folder_input_field
                {
                    folders::models::FolderInputField::Name => {
                        folders::models::FolderInputField::Path
                    }
                    folders::models::FolderInputField::Path => {
                        folders::models::FolderInputField::Name
                    }
                };
                true
            }
            KeyCode::Backspace => {
                match self.folders_state.folder_input_field {
                    folders::models::FolderInputField::Name => {
                        self.folders_state.new_folder_name.pop();
                    }
                    folders::models::FolderInputField::Path => {
                        self.folders_state.new_folder_path.pop();
                    }
                }
                true
            }
            KeyCode::Char(c) => {
                match self.folders_state.folder_input_field {
                    folders::models::FolderInputField::Name => {
                        self.folders_state.new_folder_name.push(c);
                    }
                    folders::models::FolderInputField::Path => {
                        self.folders_state.new_folder_path.push(c);
                    }
                }
                true
            }
            _ => false,
        }
    }

    fn handle_cloud_creation_input(&mut self, key: KeyCode) -> bool {
        // If we're in password creation mode, handle password input
        if self.folders_state.password_creation.creating_password {
            let char_key = match key {
                KeyCode::Char(c) => c,
                KeyCode::Enter => '\n',
                KeyCode::Esc => '\x1b',
                KeyCode::Backspace => '\x08',
                _ => return false,
            };
            self.folders_state
                .password_creation
                .handle_password_input(char_key);

            // Check if password creation is complete
            if self
                .folders_state
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
                self.folders_state.clear_cloud_creation();
                true
            }
            KeyCode::Enter => {
                if !self.folders_state.new_cloud_name.is_empty() {
                    self.complete_cloud_creation();
                }
                true
            }
            KeyCode::Backspace => {
                self.folders_state.new_cloud_name.pop();
                true
            }
            KeyCode::Char(c) => {
                self.folders_state.new_cloud_name.push(c);
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
            _ => return false,
        };

        self.folders_state
            .password_creation
            .handle_password_input(char_key);

        // Check if password creation is complete
        if self
            .folders_state
            .password_creation
            .is_password_creation_complete()
        {
            self.complete_cloud_password_creation();
        }

        true
    }

    fn complete_folder_creation(&mut self) {
        let folder_name = self.folders_state.new_folder_name.trim().to_string();
        let folder_path = std::path::PathBuf::from(self.folders_state.new_folder_path.trim());

        if folder_name.is_empty() {
            self.folders_state.folder_creation_error =
                Some("Folder name cannot be empty".to_string());
            return;
        }

        // Validate folder path exists
        if !folder_path.exists() {
            self.folders_state.folder_creation_error =
                Some(format!("Folder '{}' does not exist", folder_path.display()));
            return;
        }

        if !folder_path.is_dir() {
            self.folders_state.folder_creation_error =
                Some(format!("'{}' is not a directory", folder_path.display()));
            return;
        }

        let folder = cloudhost_server::CloudFolder::new(folder_name.clone(), folder_path.clone());

        if let Err(e) = self.orchestrator.add_cloud_folder(folder) {
            self.folders_state.folder_creation_error = Some(e.to_string());
            return;
        }

        // Reload from orchestrator
        self.load_folders_from_orchestrator();

        self.add_debug(&format!(
            "Created folder '{}' at path {}",
            folder_name,
            folder_path.display()
        ));
        self.folders_state.creating_folder = false;
        self.folders_state.folder_creation_error = None;
    }

    fn handle_folder_edit_input(&mut self, key: KeyCode) -> bool {
        match key {
            KeyCode::Esc => {
                self.folders_state.clear_folder_edit();
                true
            }
            KeyCode::Enter => {
                match self.folders_state.edit_folder_input_field {
                    folders::models::FolderInputField::Name => {
                        if !self.folders_state.edit_folder_name.is_empty() {
                            self.folders_state.edit_folder_input_field =
                                folders::models::FolderInputField::Path;
                        }
                    }
                    folders::models::FolderInputField::Path => {
                        if !self.folders_state.edit_folder_path.is_empty() {
                            self.complete_folder_edit();
                        }
                    }
                }
                true
            }
            KeyCode::Tab => {
                self.folders_state.edit_folder_input_field =
                    match self.folders_state.edit_folder_input_field {
                        folders::models::FolderInputField::Name => {
                            folders::models::FolderInputField::Path
                        }
                        folders::models::FolderInputField::Path => {
                            folders::models::FolderInputField::Name
                        }
                    };
                true
            }
            KeyCode::Backspace => {
                match self.folders_state.edit_folder_input_field {
                    folders::models::FolderInputField::Name => {
                        self.folders_state.edit_folder_name.pop();
                    }
                    folders::models::FolderInputField::Path => {
                        self.folders_state.edit_folder_path.pop();
                    }
                }
                true
            }
            KeyCode::Char(c) => {
                match self.folders_state.edit_folder_input_field {
                    folders::models::FolderInputField::Name => {
                        self.folders_state.edit_folder_name.push(c);
                    }
                    folders::models::FolderInputField::Path => {
                        self.folders_state.edit_folder_path.push(c);
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
                self.folders_state.clear_cloud_edit();
                true
            }
            KeyCode::Enter => {
                if !self.folders_state.edit_cloud_name.is_empty() {
                    self.complete_cloud_edit();
                }
                true
            }
            KeyCode::Tab => {
                // Toggle between name editing and folder navigation
                self.folders_state.cloud_edit_focus = match self.folders_state.cloud_edit_focus {
                    crate::tabs::folders::models::CloudEditFocus::Name => {
                        crate::tabs::folders::models::CloudEditFocus::Folders
                    }
                    crate::tabs::folders::models::CloudEditFocus::Folders => {
                        crate::tabs::folders::models::CloudEditFocus::Name
                    }
                };
                true
            }
            KeyCode::Backspace => {
                if self.folders_state.cloud_edit_focus
                    == crate::tabs::folders::models::CloudEditFocus::Name
                {
                    self.folders_state.edit_cloud_name.pop();
                }
                true
            }
            KeyCode::Char(c) => {
                match self.folders_state.cloud_edit_focus {
                    crate::tabs::folders::models::CloudEditFocus::Name => {
                        if c.to_string() == self.config.leader {
                            // Leader key in cloud edit modal toggles folder selection
                            if !self.folders_state.cloud_folders.is_empty() {
                                let current_index = self.folders_state.selected_folder_index;
                                self.folders_state
                                    .toggle_cloud_folder_selection(current_index);
                            }
                        } else {
                            self.folders_state.edit_cloud_name.push(c);
                        }
                        true
                    }
                    crate::tabs::folders::models::CloudEditFocus::Folders => {
                        if c.to_string() == self.config.leader {
                            // Leader key toggles folder selection
                            if !self.folders_state.cloud_folders.is_empty() {
                                let current_index = self.folders_state.selected_folder_index;
                                self.folders_state
                                    .toggle_cloud_folder_selection(current_index);
                            }
                            true
                        } else if c == 'j' || c == 'k' {
                            // Handle j/k navigation
                            self.folders_state.handle_folders_navigation(key);
                            true
                        } else {
                            false // Let other keys be handled by navigation
                        }
                    }
                }
            }
            KeyCode::Up | KeyCode::Down => {
                if self.folders_state.cloud_edit_focus
                    == crate::tabs::folders::models::CloudEditFocus::Folders
                {
                    // Allow navigation in folder list within cloud edit modal
                    self.folders_state.handle_folders_navigation(key);
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    fn complete_folder_edit(&mut self) {
        let new_name = self.folders_state.edit_folder_name.trim().to_string();
        let new_path = std::path::PathBuf::from(self.folders_state.edit_folder_path.trim());
        let old_name = self.folders_state.edit_folder_original_name.clone();

        if new_name.is_empty() {
            self.folders_state.folder_edit_error = Some("Folder name cannot be empty".to_string());
            return;
        }

        // Validate folder path exists
        if !new_path.exists() {
            self.folders_state.folder_edit_error =
                Some(format!("Folder '{}' does not exist", new_path.display()));
            return;
        }

        if !new_path.is_dir() {
            self.folders_state.folder_edit_error =
                Some(format!("'{}' is not a directory", new_path.display()));
            return;
        }

        let new_folder = cloudhost_server::CloudFolder::new(new_name.clone(), new_path.clone());

        if let Err(e) = self.orchestrator.update_cloud_folder(&old_name, new_folder) {
            self.folders_state.folder_edit_error = Some(e.to_string());
            return;
        }

        // Reload from orchestrator
        self.load_folders_from_orchestrator();

        self.add_debug(&format!("Updated folder '{}' to '{}'", old_name, new_name));
        self.folders_state.clear_folder_edit();
    }

    fn complete_cloud_edit(&mut self) {
        let new_name = self.folders_state.edit_cloud_name.trim().to_string();
        let old_name = self.folders_state.edit_cloud_original_name.clone();

        if new_name.is_empty() {
            self.folders_state.cloud_edit_error = Some("Cloud name cannot be empty".to_string());
            return;
        }

        let folder_names = self.folders_state.get_edit_cloud_selected_folder_names();

        if folder_names.is_empty() {
            self.folders_state.cloud_edit_error =
                Some("Cloud must have at least one folder".to_string());
            return;
        }

        // Build cloud from selected folders
        let folders: Vec<cloudhost_server::CloudFolder> = folder_names
            .iter()
            .filter_map(|name| {
                self.folders_state
                    .cloud_folders
                    .iter()
                    .find(|f| &f.name == name)
                    .map(|f| {
                        cloudhost_server::CloudFolder::new(f.name.clone(), f.folder_path.clone())
                    })
            })
            .collect();

        if folders.is_empty() {
            self.folders_state.cloud_edit_error = Some("No valid folders selected".to_string());
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
            self.folders_state.cloud_edit_error = Some(e.to_string());
            return;
        }

        // Check if the updated cloud has a password set
        if !self.orchestrator.cloud_has_password(&new_name) {
            self.folders_state.cloud_edit_error = Some(
                "Cloud updated successfully, but no password is set. Please set a password before starting the cloud.".to_string()
            );
            return;
        }

        // Reload from orchestrator
        self.load_folders_from_orchestrator();

        self.add_debug(&format!("Updated cloud '{}' to '{}'", old_name, new_name));
        self.folders_state.clear_cloud_edit();
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

    pub async fn update_cloud_logs(&mut self) {
        // Clear existing logs
        self.cloud_logs.clear();
        self.clouds_state.cloud_logs.clear();

        // Get the selected cloud name and fetch its logs
        if !self.clouds_state.clouds.is_empty()
            && self.clouds_state.selected_cloud_index < self.clouds_state.clouds.len()
        {
            let cloud_name = &self.clouds_state.clouds[self.clouds_state.selected_cloud_index].name;

            // Get debug logs for the specific cloud
            let messages = self.orchestrator.get_cloud_debug_logs(cloud_name).await;

            for message in messages {
                self.cloud_logs.push(message.clone());
                // Also add formatted string to clouds_state for display
                let log_entry = format!(
                    "[{}] [{}] {}: {}",
                    message.timestamp.format("%H:%M:%S"),
                    message.level,
                    message.source,
                    message.message
                );
                self.clouds_state.cloud_logs.push(log_entry);
            }
        }

        // Keep only last 100 logs
        if self.cloud_logs.len() > 100 {
            self.cloud_logs.truncate(100);
        }
        if self.clouds_state.cloud_logs.len() > 100 {
            self.clouds_state.cloud_logs.truncate(100);
        }
    }

    // Tab-specific focus management
    pub fn cycle_focus_forward(&mut self) {
        match self.selected_tab {
            SelectedTab::Clouds => self.clouds_state.cycle_focus_forward(),
            SelectedTab::Folders => self.folders_state.cycle_focus_forward(),
            SelectedTab::Settings => self.settings_state.cycle_focus_forward(),
        }
    }

    pub fn cycle_focus_backward(&mut self) {
        match self.selected_tab {
            SelectedTab::Clouds => self.clouds_state.cycle_focus_backward(),
            SelectedTab::Folders => self.folders_state.cycle_focus_backward(),
            SelectedTab::Settings => self.settings_state.cycle_focus_backward(),
        }
    }

    pub fn get_current_focused_element(&self) -> String {
        match self.selected_tab {
            SelectedTab::Clouds => self.clouds_state.get_focused_element(),
            SelectedTab::Folders => self.folders_state.get_focused_element(),
            SelectedTab::Settings => self.settings_state.get_focused_element(),
        }
    }

    async fn complete_password_creation(&mut self) {
        // Set the password for the selected cloud (from clouds tab)
        if self.clouds_state.clouds.is_empty()
            || self.clouds_state.selected_cloud_index >= self.clouds_state.clouds.len()
        {
            self.clouds_state.password_creation.password_error =
                Some("No cloud selected".to_string());
            return;
        }

        let cloud_name = self.clouds_state.clouds[self.clouds_state.selected_cloud_index]
            .name
            .clone();

        let password = self
            .clouds_state
            .password_creation
            .get_password()
            .to_string();

        if let Err(e) = self
            .clouds_state
            .set_password(&mut self.orchestrator, &password)
        {
            self.clouds_state.password_creation.password_error = Some(e);
        } else {
            self.clouds_state.clear_password_creation();
            self.clouds_state.password_creation.password_success = true;
            // Clear any server start errors since password is now set
            self.clouds_state.cloud_start_error = None;
            self.add_debug(&format!(
                "Password set successfully for cloud '{}'",
                cloud_name
            ));

            // If server is running, restart it to pick up the new AuthState
            let was_running = self.clouds_state.running_clouds.contains_key(&cloud_name);

            if was_running {
                self.add_debug("Stopping server to apply password changes");
                self.clouds_state.stop_server(&mut self.orchestrator).await;
            }

            // Recreate the orchestrator instance to pick up the new config
            self.add_debug("Recreating orchestrator instance with new config");
            // Orchestrator is already initialized in App
            // Reload clouds from orchestrator
            self.load_folders_from_orchestrator();

            // If server was running, restart it automatically
            if was_running {
                self.add_debug("Restarting server with new password");
                self.clouds_state.start_server(&mut self.orchestrator).await;
                self.add_debug("Server restart initiated with new password");
            }
        }
    }

    // Tab-specific navigation methods
    pub fn handle_tab_navigation(&mut self, key: ratatui::crossterm::event::KeyCode) -> bool {
        match self.selected_tab {
            SelectedTab::Clouds => self.clouds_state.handle_navigation(key),
            SelectedTab::Folders => self.folders_state.handle_navigation(key),
            SelectedTab::Settings => self.settings_state.handle_navigation(key),
        }
    }

    pub fn reload_tui_config(&mut self) {
        self.config = crate::config::Config::load().unwrap();
    }
    pub async fn reload_clouds_config(&mut self) {
        if let Err(e) = self.orchestrator.reload_config().await {
            self.add_debug(&format!("Failed to reload clouds config: {}", e));
        } else {
            // Reload data from orchestrator to reflect changes
            self.load_folders_from_orchestrator();
        }
    }
    pub async fn reload_all_configs(&mut self) {
        self.reload_tui_config();
        self.reload_clouds_config().await;
    }

    pub async fn handle_dynamic_key(
        &mut self,
        key: ratatui::crossterm::event::KeyCode,
        modifiers: ratatui::crossterm::event::KeyModifiers,
    ) {
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
            SelectedTab::Clouds => "clouds",
            SelectedTab::Folders => "folders",
            SelectedTab::Settings => "settings",
        };

        self.add_debug(&format!("Key: {} -> tab: {}", key_str, current_tab));
        self.add_debug(&format!("Input state: {:?}", self.input_state));

        // Handle special cases first (cloud management)
        // Clouds are managed in the folders tab, not here

        // Handle password creation modal (now on clouds tab)
        if self.clouds_state.password_creation.creating_password {
            let char_key = match key {
                KeyCode::Char(c) => c,
                KeyCode::Enter => '\n',
                KeyCode::Esc => '\x1b',
                KeyCode::Backspace => '\x08',
                _ => return,
            };
            self.clouds_state.handle_password_input(char_key);
            // If password creation is complete, handle it
            if self
                .clouds_state
                .password_creation
                .is_password_creation_complete()
            {
                self.complete_password_creation().await;
            }
            return;
        }

        // Handle folder creation modal
        if self.folders_state.creating_folder && self.handle_folder_creation_input(key) {
            return;
        }

        // Handle cloud creation modal
        if self.folders_state.creating_cloud && self.handle_cloud_creation_input(key) {
            return;
        }

        // Handle password creation modal (for existing clouds)
        if self.folders_state.password_creation.creating_password
            && !self.folders_state.creating_cloud
            && self.handle_password_creation_input(key)
        {
            return;
        }

        // Handle folder edit modal
        if self.folders_state.editing_folder && self.handle_folder_edit_input(key) {
            return;
        }

        // Handle cloud edit modal
        if self.folders_state.editing_cloud && self.handle_cloud_edit_input(key) {
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
                if let Some(action) = self.config.get_action_for_key(seq) {
                    if self.config.is_key_valid_for_tab(seq, current_tab) {
                        self.execute_action(&action).await;
                    }
                }
                self.input_state = InputState::Normal;
                return;
            }

            if seq == "<leader>" {
                let leader_key = format!("<leader>{}", key_str);
                if let Some(action) = self.config.get_action_for_key(&leader_key) {
                    if self.config.is_key_valid_for_tab(&leader_key, current_tab) {
                        self.execute_action(&action).await;
                    }
                }
                self.input_state = InputState::Normal;
                return;
            } else {
                // Try to complete the sequence with the current key
                let complete_key = format!("{}{}", seq, key_str);
                if let Some(action) = self.config.get_action_for_key(&complete_key) {
                    if self.config.is_key_valid_for_tab(&complete_key, current_tab) {
                        self.execute_action(&action).await;
                    }
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

        // Check if key is valid for current tab and process it
        if let Some(action) = self.config.get_action_for_key(&key_str) {
            if self.config.is_key_valid_for_tab(&key_str, current_tab) {
                self.execute_action(&action).await;
            } else {
                self.add_debug(&format!(
                    "Key '{}' not valid for tab '{}'",
                    key_str, current_tab
                ));
            }
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
            "Start/Stop Cloud" => {
                if !self.clouds_state.clouds.is_empty()
                    && self.clouds_state.selected_cloud_index < self.clouds_state.clouds.len()
                    && self.clouds_state.is_cloud_running(
                        &self.clouds_state.clouds[self.clouds_state.selected_cloud_index].name,
                    )
                {
                    self.clouds_state.stop_server(&mut self.orchestrator).await;
                } else {
                    self.clouds_state.start_server(&mut self.orchestrator).await;
                }
            }
            "Create New" => {
                self.start_creating_folder_or_cloud();
            }
            "Delete Folder" => {
                self.delete_selected_folder();
            }
            "Delete Cloud" => {
                self.delete_selected_cloud();
            }
            "Set Password" => {
                self.start_setting_cloud_password();
            }
            "Select All Folders" => {
                self.select_all_folders();
            }
            "Edit" => {
                self.start_editing();
            }
            "Toggle Password Visibility" => {
                self.toggle_cloud_password_display();
            }
            "Create Password" => {
                if self.selected_tab == SelectedTab::Clouds {
                    if !self.clouds_state.clouds.is_empty() {
                        self.clouds_state.start_creating_password();
                        let cloud_name =
                            &self.clouds_state.clouds[self.clouds_state.selected_cloud_index].name;
                        self.add_debug(&format!("Creating password for cloud '{}'", cloud_name));
                    } else {
                        self.add_debug(
                            "No clouds available. Create a cloud first in the Folders tab.",
                        );
                    }
                } else {
                    self.add_debug("Password creation moved to Clouds tab. Switch to Clouds tab and press 'p'.");
                }
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
                if self.selected_tab == SelectedTab::Folders
                    && self.folders_state.focused_panel
                        == crate::tabs::folders::models::FocusedPanel::Folders
                    && !self.folders_state.cloud_folders.is_empty()
                {
                    self.folders_state
                        .toggle_folder_selection(self.folders_state.selected_folder_index);
                    self.add_debug(&format!(
                        "Toggled selection for folder at index {}",
                        self.folders_state.selected_folder_index
                    ));
                }
            }
            "Refresh/Reload" => {
                // Reload data from orchestrator
                self.load_folders_from_orchestrator();
                self.add_debug("Refreshed data from orchestrator");
            }
            "Reload TUI Config" => {
                self.reload_tui_config();
                self.add_debug("TUI config reloaded successfully");
            }
            "Reload Clouds Config" => {
                self.reload_clouds_config().await;
                self.add_debug("Clouds config reloaded successfully");
            }
            "Reload All Configs" => {
                self.reload_all_configs().await;
                self.add_debug("All configs reloaded successfully");
            }
            "Execute Action" => {
                // Handle Enter key in settings tab
                if self.selected_tab == SelectedTab::Settings {
                    self.settings_state.handle_enter();
                    self.add_debug("Executed settings action");
                }
            }
            _ => {
                self.add_debug(&format!("Unknown action: {}", action));
            }
        }
    }

    /// Check if any pending key sequences have timed out and execute them
    pub async fn check_timeouts(&mut self) {
        if let InputState::KeySequence(ref seq, start_time) = self.input_state {
            if start_time.elapsed().as_millis() > KEY_SEQUENCE_TIMEOUT_MS as u128 {
                // Timeout reached, execute the single key if it exists
                if let Some(action) = self.config.get_action_for_key(seq) {
                    let current_tab = match self.selected_tab {
                        SelectedTab::Clouds => "clouds",
                        SelectedTab::Folders => "folders",
                        SelectedTab::Settings => "settings",
                    };
                    if self.config.is_key_valid_for_tab(seq, current_tab) {
                        self.execute_action(&action).await;
                    }
                }
                self.input_state = InputState::Normal;
            }
        }
    }
}

impl Widget for &mut App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        use ratatui::layout::{Constraint, Layout};

        if self.debug_mode {
            // Debug mode: show debug panel
            let vertical = Layout::vertical([
                Constraint::Length(1),
                Constraint::Min(0),
                Constraint::Length(1),
                Constraint::Length(12), // Debug panel
            ]);
            let [header_area, inner_area, footer_area, debug_area] = vertical.areas(area);

            let horizontal = Layout::horizontal([Constraint::Min(0), Constraint::Length(20)]);
            let [tabs_area, title_area] = horizontal.areas(header_area);

            render_title(title_area, buf);
            self.render_tabs(tabs_area, buf);
            self.selected_tab.render_tab(self, inner_area, buf);
            self.render_footer(footer_area, buf);
            self.render_debug_panel(debug_area, buf);
        } else {
            // Normal mode
            let vertical = Layout::vertical([
                Constraint::Length(1),
                Constraint::Min(0),
                Constraint::Length(1),
            ]);
            let [header_area, inner_area, footer_area] = vertical.areas(area);

            let horizontal = Layout::horizontal([Constraint::Min(0), Constraint::Length(20)]);
            let [tabs_area, title_area] = horizontal.areas(header_area);

            render_title(title_area, buf);
            self.render_tabs(tabs_area, buf);
            self.selected_tab.render_tab(self, inner_area, buf);
            self.render_footer(footer_area, buf);
        }
    }
}

impl App {
    pub fn render_tabs(&self, area: Rect, buf: &mut Buffer) {
        let titles = SelectedTab::iter().map(SelectedTab::title);
        let highlight_style = (
            ratatui::style::Color::default(),
            self.selected_tab.palette().c700,
        );
        let selected_tab_index = self.selected_tab as usize;
        Tabs::new(titles)
            .highlight_style(highlight_style)
            .select(selected_tab_index)
            .padding("", "")
            .divider(" ")
            .render(area, buf);
    }

    pub fn render_debug_panel(&self, area: Rect, buf: &mut Buffer) {
        use ratatui::style::{Color, Style};
        use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};

        // Create a scrollable area for server logs
        let cloud_logs_area = Rect {
            x: area.x,
            y: area.y,
            width: area.width,
            height: area.height.saturating_sub(3), // Leave space for header
        };

        let header_area = Rect {
            x: area.x,
            y: area.y + area.height.saturating_sub(3),
            width: area.width,
            height: 3,
        };

        // Render header with log count
        let header_text = format!("TUI Debug ({} messages)", self.debug_info.len());
        let header = Paragraph::new(header_text)
            .block(Block::default().borders(Borders::ALL).title("Debug Panel"))
            .style(Style::default().fg(Color::Cyan));
        header.render(header_area, buf);

        // Only show TUI debug messages in debug panel (server logs are in server tab)
        let mut all_items = Vec::new();

        // Add TUI debug messages
        for info in &self.debug_info {
            all_items.push(
                ListItem::new(format!("[TUI] {}", info)).style(Style::default().fg(Color::Magenta)),
            );
        }

        // Create scrollable list
        let list = List::new(all_items).block(Block::default().borders(Borders::ALL).title("Logs"));

        ratatui::widgets::Widget::render(list, cloud_logs_area, buf);
    }
}

pub fn render_title(area: Rect, buf: &mut Buffer) {
    let title = if cloudhost_server::config_paths::is_dev_mode() {
        "CloudHost (dev)"
    } else {
        "CloudHost"
    };
    title.bold().render(area, buf);
}

impl App {
    pub fn render_footer(&self, area: Rect, buf: &mut Buffer) {
        let footer_text = match self.selected_tab {
            SelectedTab::Clouds => "j/k or ↑/↓ to navigate | s to start/stop server | gt/gT to switch tabs | q to quit",
            SelectedTab::Folders => "j/k or ↑/↓ to navigate | Tab to switch panels | gt/gT to switch tabs | q to quit",
            SelectedTab::Settings => "j/k or ↑/↓ to navigate | Enter to execute action | gt/gT to switch tabs | q to quit",
        };
        Line::raw(footer_text).centered().render(area, buf);
    }
}
