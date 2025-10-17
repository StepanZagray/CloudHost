use crate::tabs::focus::TabFocus;
use crate::utils::password::PasswordCreationState;
use ratatui::crossterm::event::KeyCode;
use ratatui::widgets::{ListState, ScrollbarState};

// Re-export server types
pub use cloudhost_server::{Cloud, CloudFolder};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusedPanel {
    Folders,
    Clouds,
    Info,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FolderInputField {
    Name,
    Path,
}

pub struct FoldersState {
    pub cloud_folders: Vec<CloudFolder>,
    pub selected_folder_index: usize,
    pub selected_folders: std::collections::HashSet<usize>, // Track selected folder indices
    pub clouds: Vec<Cloud>,
    pub selected_cloud_index: usize,
    pub creating_folder: bool,
    pub new_folder_name: String,
    pub new_folder_path: String,
    pub folder_input_field: FolderInputField,
    pub folder_creation_error: Option<String>,
    pub creating_cloud: bool,
    pub new_cloud_name: String,
    pub cloud_creation_error: Option<String>,
    // Shared password creation state
    pub password_creation: PasswordCreationState,
    // Edit state
    pub editing_folder: bool,
    pub edit_folder_original_name: String,
    pub edit_folder_name: String,
    pub edit_folder_path: String,
    pub edit_folder_input_field: FolderInputField,
    pub folder_edit_error: Option<String>,
    pub editing_cloud: bool,
    pub edit_cloud_original_name: String,
    pub edit_cloud_name: String,
    pub edit_cloud_selected_folders: std::collections::HashSet<usize>, // Folders selected for this group
    pub cloud_edit_error: Option<String>,
    pub focused_panel: FocusedPanel,
    pub folders_list_state: ListState,
    pub folders_scroll_state: ScrollbarState,
    pub clouds_list_state: ListState,
    pub clouds_scroll_state: ScrollbarState,
}

impl Default for FoldersState {
    fn default() -> Self {
        Self {
            cloud_folders: Vec::new(),
            selected_folder_index: 0,
            selected_folders: std::collections::HashSet::new(),
            clouds: Vec::new(),
            selected_cloud_index: 0,
            creating_folder: false,
            new_folder_name: String::new(),
            new_folder_path: String::new(),
            folder_input_field: FolderInputField::Name,
            folder_creation_error: None,
            creating_cloud: false,
            new_cloud_name: String::new(),
            cloud_creation_error: None,
            password_creation: PasswordCreationState::new(),
            editing_folder: false,
            edit_folder_original_name: String::new(),
            edit_folder_name: String::new(),
            edit_folder_path: String::new(),
            edit_folder_input_field: FolderInputField::Name,
            folder_edit_error: None,
            editing_cloud: false,
            edit_cloud_original_name: String::new(),
            edit_cloud_name: String::new(),
            edit_cloud_selected_folders: std::collections::HashSet::new(),
            cloud_edit_error: None,
            focused_panel: FocusedPanel::Folders,
            folders_list_state: ListState::default(),
            folders_scroll_state: ScrollbarState::default(),
            clouds_list_state: ListState::default(),
            clouds_scroll_state: ScrollbarState::default(),
        }
    }
}

impl TabFocus for FoldersState {
    fn get_focused_element(&self) -> String {
        match self.focused_panel {
            FocusedPanel::Folders => "FoldersList".to_string(),
            FocusedPanel::Clouds => "CloudsList".to_string(),
            FocusedPanel::Info => "InfoPanel".to_string(),
        }
    }

    fn cycle_focus_forward(&mut self) {
        self.focused_panel = match self.focused_panel {
            FocusedPanel::Folders => FocusedPanel::Clouds,
            FocusedPanel::Clouds => FocusedPanel::Folders,
            FocusedPanel::Info => FocusedPanel::Folders, // Info panel is not focusable
        };
    }

    fn cycle_focus_backward(&mut self) {
        self.focused_panel = match self.focused_panel {
            FocusedPanel::Folders => FocusedPanel::Clouds,
            FocusedPanel::Clouds => FocusedPanel::Folders,
            FocusedPanel::Info => FocusedPanel::Folders, // Info panel is not focusable
        };
    }

    fn handle_navigation(&mut self, key: KeyCode) -> bool {
        match self.focused_panel {
            FocusedPanel::Folders => self.handle_folders_navigation(key),
            FocusedPanel::Clouds => self.handle_clouds_navigation(key),
            FocusedPanel::Info => false, // Info panel is not navigable
        }
    }
}

impl FoldersState {
    pub fn get_selected_folders_count(&self) -> usize {
        self.selected_folders.len()
    }

    pub fn is_folder_selected(&self, index: usize) -> bool {
        self.selected_folders.contains(&index)
    }

    pub fn toggle_folder_selection(&mut self, index: usize) {
        if self.selected_folders.contains(&index) {
            self.selected_folders.remove(&index);
        } else {
            self.selected_folders.insert(index);
        }
    }

    pub fn clear_folder_selections(&mut self) {
        self.selected_folders.clear();
    }

    pub fn select_all_folders(&mut self) {
        self.selected_folders.clear();
        for i in 0..self.cloud_folders.len() {
            self.selected_folders.insert(i);
        }
    }

    pub fn start_creating_cloud(&mut self) {
        self.creating_cloud = true;
        self.new_cloud_name.clear();
        self.cloud_creation_error = None;
    }

    pub fn clear_cloud_creation(&mut self) {
        self.creating_cloud = false;
        self.new_cloud_name.clear();
        self.cloud_creation_error = None;
        self.password_creation.clear_password_creation();
    }

    pub fn get_selected_folder_names(&self) -> Vec<String> {
        self.selected_folders
            .iter()
            .filter_map(|&index| {
                self.cloud_folders
                    .get(index)
                    .map(|folder| folder.name.clone())
            })
            .collect()
    }

    pub fn start_editing_folder(&mut self) {
        if let Some(folder) = self.cloud_folders.get(self.selected_folder_index) {
            self.editing_folder = true;
            self.edit_folder_original_name = folder.name.clone();
            self.edit_folder_name = folder.name.clone();
            self.edit_folder_path = folder.folder_path.to_string_lossy().to_string();
            self.edit_folder_input_field = FolderInputField::Name;
            self.folder_edit_error = None;
        }
    }

    pub fn clear_folder_edit(&mut self) {
        self.editing_folder = false;
        self.edit_folder_original_name.clear();
        self.edit_folder_name.clear();
        self.edit_folder_path.clear();
        self.folder_edit_error = None;
    }

    pub fn start_editing_cloud(&mut self) {
        if let Some(cloud) = self.clouds.get(self.selected_cloud_index) {
            self.editing_cloud = true;
            self.edit_cloud_original_name = cloud.name.clone();
            self.edit_cloud_name = cloud.name.clone();
            self.cloud_edit_error = None;

            // Mark folders that are in this cloud as selected
            self.edit_cloud_selected_folders.clear();
            for (index, folder) in self.cloud_folders.iter().enumerate() {
                if cloud.cloud_folders.iter().any(|cf| cf.name == folder.name) {
                    self.edit_cloud_selected_folders.insert(index);
                }
            }
        }
    }

    pub fn clear_cloud_edit(&mut self) {
        self.editing_cloud = false;
        self.edit_cloud_original_name.clear();
        self.edit_cloud_name.clear();
        self.edit_cloud_selected_folders.clear();
        self.cloud_edit_error = None;
    }

    pub fn toggle_cloud_folder_selection(&mut self, index: usize) {
        if self.edit_cloud_selected_folders.contains(&index) {
            self.edit_cloud_selected_folders.remove(&index);
        } else {
            self.edit_cloud_selected_folders.insert(index);
        }
    }

    pub fn is_cloud_folder_selected(&self, index: usize) -> bool {
        self.edit_cloud_selected_folders.contains(&index)
    }

    pub fn get_edit_cloud_selected_folder_names(&self) -> Vec<String> {
        self.edit_cloud_selected_folders
            .iter()
            .filter_map(|&index| {
                self.cloud_folders
                    .get(index)
                    .map(|folder| folder.name.clone())
            })
            .collect()
    }

    pub fn handle_folders_navigation(&mut self, key: KeyCode) -> bool {
        match key {
            KeyCode::Up | KeyCode::Char('k') => {
                if !self.cloud_folders.is_empty() {
                    self.selected_folder_index = self.selected_folder_index.saturating_sub(1);
                    if self.selected_folder_index >= self.cloud_folders.len() {
                        self.selected_folder_index = self.cloud_folders.len().saturating_sub(1);
                    }
                    self.folders_list_state
                        .select(Some(self.selected_folder_index));
                }
                true
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if !self.cloud_folders.is_empty() {
                    self.selected_folder_index = (self.selected_folder_index + 1)
                        .min(self.cloud_folders.len().saturating_sub(1));
                    self.folders_list_state
                        .select(Some(self.selected_folder_index));
                }
                true
            }
            KeyCode::Char('g') => {
                if !self.cloud_folders.is_empty() {
                    self.selected_folder_index = 0;
                    self.folders_list_state.select(Some(0));
                }
                true
            }
            KeyCode::Char('G') => {
                if !self.cloud_folders.is_empty() {
                    self.selected_folder_index = self.cloud_folders.len().saturating_sub(1);
                    self.folders_list_state
                        .select(Some(self.selected_folder_index));
                }
                true
            }
            _ => false,
        }
    }

    fn handle_clouds_navigation(&mut self, key: KeyCode) -> bool {
        match key {
            KeyCode::Up | KeyCode::Char('k') => {
                if !self.clouds.is_empty() {
                    self.selected_cloud_index = self.selected_cloud_index.saturating_sub(1);
                    if self.selected_cloud_index >= self.clouds.len() {
                        self.selected_cloud_index = self.clouds.len().saturating_sub(1);
                    }
                    self.clouds_list_state
                        .select(Some(self.selected_cloud_index));
                }
                true
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if !self.clouds.is_empty() {
                    self.selected_cloud_index =
                        (self.selected_cloud_index + 1).min(self.clouds.len().saturating_sub(1));
                    self.clouds_list_state
                        .select(Some(self.selected_cloud_index));
                }
                true
            }
            KeyCode::Char('g') => {
                if !self.clouds.is_empty() {
                    self.selected_cloud_index = 0;
                    self.clouds_list_state.select(Some(0));
                }
                true
            }
            KeyCode::Char('G') => {
                if !self.clouds.is_empty() {
                    self.selected_cloud_index = self.clouds.len().saturating_sub(1);
                    self.clouds_list_state
                        .select(Some(self.selected_cloud_index));
                }
                true
            }
            _ => false,
        }
    }
}
