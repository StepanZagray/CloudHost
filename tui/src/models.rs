use std::path::PathBuf;

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{palette::tailwind, Stylize},
    symbols,
    text::Line,
    widgets::{Block, Padding, Paragraph, Tabs, Widget},
};
use strum::{Display, EnumIter, FromRepr, IntoEnumIterator};

use crate::tabs::{SelectedTab, server_tab};

#[derive(Default)]
pub struct App {
    pub state: AppState,
    pub selected_tab: SelectedTab,
    pub config: crate::config::Config,
    pub input_state: InputState,
    pub pending_number: Option<usize>,
    pub debug_mode: bool,
    pub debug_info: Vec<String>,

    // Server and profile management
    pub profiles: Vec<Profile>,
    pub selected_profile_index: usize,
    pub server_running: bool,
    pub creating_profile: bool,
    pub new_profile_name: String,
    pub profile_creation_error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Profile {
    pub name: String,
    pub folder_path: PathBuf,
}

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum AppState {
    #[default]
    Running,
    Quitting,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputState {
    Normal,
    NumberPrefix,
    KeySequence(String), // Stores the current key sequence being typed
}

impl Default for InputState {
    fn default() -> Self {
        InputState::Normal
    }
}

impl App {
    pub fn new() -> Self {
        let mut app = Self {
            config: crate::config::Config::load_or_default(),
            ..Default::default()
        };
        app.load_profiles();
        app
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

    // Server and profile management methods
    pub fn start_creating_profile(&mut self) {
        self.creating_profile = true;
        self.new_profile_name = String::new();
        self.profile_creation_error = None;
        self.add_debug("Started creating new profile");
    }

    pub fn cancel_creating_profile(&mut self) {
        self.creating_profile = false;
        self.new_profile_name = String::new();
        self.profile_creation_error = None;
        self.add_debug("Cancelled profile creation");
    }

    pub fn create_profile(&mut self) {
        if !self.new_profile_name.is_empty() {
            let name = self.new_profile_name.trim();

            // Check if profile already exists
            if self.profile_exists(name) {
                self.profile_creation_error = Some(format!("Profile '{}' already exists", name));
                self.add_debug(&format!("Profile '{}' already exists", name));
                return;
            }

            // Create profile folder
            let profiles_path = self.expand_path(&self.config.profiles_path);
            let profile_folder = format!("{}/{}", profiles_path, name);

            if let Err(e) = std::fs::create_dir_all(&profile_folder) {
                self.profile_creation_error =
                    Some(format!("Failed to create profile folder: {}", e));
                self.add_debug(&format!("Failed to create profile folder: {}", e));
                return;
            }

            let profile = Profile {
                name: name.to_string(),
                folder_path: profile_folder.into(),
            };

            self.profiles.push(profile);
            self.add_debug(&format!("Created profile: {}", name));

            self.creating_profile = false;
            self.new_profile_name = String::new();
            self.profile_creation_error = None;
        }
    }

    pub fn start_server(&mut self) {
        if let Some(profile) = self.profiles.get(self.selected_profile_index) {
            self.server_running = true;
            self.add_debug(&format!("Started server for profile: {}", profile.name));
        }
    }

    pub fn stop_server(&mut self) {
        self.server_running = false;
        self.add_debug("Stopped server");
    }

    pub fn load_profiles(&mut self) {
        let profiles_path = self.expand_path(&self.config.profiles_path);
        self.add_debug(&format!("Loading profiles from: {}", profiles_path));

        if let Ok(entries) = std::fs::read_dir(&profiles_path) {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    if entry.path().is_dir() {
                        let profile = Profile {
                            name: name.to_string(),
                            folder_path: entry.path(),
                        };
                        self.profiles.push(profile);
                        self.add_debug(&format!("Loaded profile: {}", name));
                    }
                }
            }
        } else {
            self.add_debug(&format!(
                "Could not read profiles directory: {}",
                profiles_path
            ));
        }
    }

    pub fn expand_path(&self, path: &str) -> String {
        if path.starts_with("~/") {
            if let Some(home) = dirs::home_dir() {
                return format!("{}/{}", home.to_string_lossy(), &path[2..]);
            }
        }
        path.to_string()
    }

    pub fn profile_exists(&self, name: &str) -> bool {
        self.profiles.iter().any(|p| p.name == name)
    }

    pub fn delete_selected_profile(&mut self) {
        if !self.profiles.is_empty() && self.selected_profile_index < self.profiles.len() {
            let profile_name = self.profiles[self.selected_profile_index].name.clone();
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
                    self.add_debug(&format!("Deleted profile: {}", profile_name));
                }
                Err(e) => {
                    self.add_debug(&format!(
                        "Failed to delete profile '{}': {}",
                        profile_name, e
                    ));
                }
            }
        }
    }

    pub fn handle_dynamic_key(&mut self, key: ratatui::crossterm::event::KeyCode) {
        use ratatui::crossterm::event::KeyCode;

        // Convert key to string for config lookup
        let key_str = match key {
            KeyCode::Char(c) => c.to_string(),
            KeyCode::Up => "<Up>".to_string(),
            KeyCode::Down => "<Down>".to_string(),
            KeyCode::Left => "<Left>".to_string(),
            KeyCode::Right => "<Right>".to_string(),
            KeyCode::Enter => "<Enter>".to_string(),
            KeyCode::Esc => "<Esc>".to_string(),
            KeyCode::Backspace => "<Backspace>".to_string(),
            _ => return,
        };

        // Get current tab name
        let current_tab = match self.selected_tab {
            SelectedTab::Server => "server",
            SelectedTab::Client => "client",
            SelectedTab::Settings => "settings",
        };

        self.add_debug(&format!("Key: {} -> tab: {}", key_str, current_tab));
        self.add_debug(&format!("Input state: {:?}", self.input_state));

        // Handle special cases first (profile creation)
        if self.creating_profile {
            match key {
                KeyCode::Enter => {
                    self.create_profile();
                    return;
                }
                KeyCode::Esc => {
                    self.cancel_creating_profile();
                    return;
                }
                KeyCode::Char(c) => {
                    self.new_profile_name.push(c);
                    self.profile_creation_error = None; // Clear error when typing
                    return;
                }
                KeyCode::Backspace => {
                    self.new_profile_name.pop();
                    self.profile_creation_error = None; // Clear error when editing
                    return;
                }
                _ => return,
            }
        }

        // Handle leader key sequences first
        if key_str == self.config.leader {
            self.input_state = InputState::KeySequence("<leader>".to_string());
            self.add_debug(&format!("Leader key ('{}') pressed", key_str));
            return;
        }

        // Handle multi-key sequences (like gt, gT, <leader>d)
        if let InputState::KeySequence(ref seq) = self.input_state {
            if seq == "<leader>" {
                let leader_key = format!("<leader>{}", key_str);
                if let Some(keybinding) = self.config.get_keybinding(&leader_key) {
                    if self.config.is_key_valid_for_tab(&leader_key, current_tab) {
                        match keybinding.action.as_str() {
                            "Toggle Debug" => self.toggle_debug(),
                            _ => {}
                        }
                    }
                }
                self.input_state = InputState::Normal;
                return;
            } else if seq == "g" {
                let g_key = format!("g{}", key_str);
                if let Some(keybinding) = self.config.get_keybinding(&g_key) {
                    if self.config.is_key_valid_for_tab(&g_key, current_tab) {
                        match keybinding.action.as_str() {
                            "Next Tab" => self.next_tab(),
                            "Previous Tab" => self.previous_tab(),
                            _ => {}
                        }
                    }
                }
                self.input_state = InputState::Normal;
                return;
            }
        }

        // Handle single character keys that start sequences
        if key_str == "g" {
            self.input_state = InputState::KeySequence("g".to_string());
            self.add_debug(&format!("Started 'g' sequence"));
            return;
        }

        // Check if key is valid for current tab and process it
        if let Some(keybinding) = self.config.get_keybinding(&key_str) {
            if self.config.is_key_valid_for_tab(&key_str, current_tab) {
                match keybinding.action.as_str() {
                    "Quit" => self.quit(),
                    "Next Tab" => self.next_tab(),
                    "Previous Tab" => self.previous_tab(),
                    "Toggle Debug" => self.toggle_debug(),
                    "Start/Stop Server" => {
                        if self.server_running {
                            self.stop_server();
                        } else {
                            self.start_server();
                        }
                    }
                    "Create Profile" => self.start_creating_profile(),
                    "Delete Profile" => self.delete_selected_profile(),
                    "Previous Profile" => {
                        if self.selected_profile_index > 0 {
                            self.selected_profile_index -= 1;
                        }
                    }
                    "Next Profile" => {
                        if self.selected_profile_index < self.profiles.len().saturating_sub(1) {
                            self.selected_profile_index += 1;
                        }
                    }
                    _ => {
                        self.add_debug(&format!("Unknown action: {}", keybinding.action));
                    }
                }
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
}

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        use ratatui::layout::{Constraint, Layout};

        if self.debug_mode {
            // Debug mode: show debug panel
            let vertical = Layout::vertical([
                Constraint::Length(1),
                Constraint::Min(0),
                Constraint::Length(1),
                Constraint::Length(8), // Debug panel
            ]);
            let [header_area, inner_area, footer_area, debug_area] = vertical.areas(area);

            let horizontal = Layout::horizontal([Constraint::Min(0), Constraint::Length(20)]);
            let [tabs_area, title_area] = horizontal.areas(header_area);

            render_title(title_area, buf);
            self.render_tabs(tabs_area, buf);
            if self.selected_tab == SelectedTab::Server {
                server_tab::render_server_tab(self, inner_area, buf);
            } else {
                self.selected_tab.render(inner_area, buf);
            }
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
            if self.selected_tab == SelectedTab::Server {
                server_tab::render_server_tab(self, inner_area, buf);
            } else {
                self.selected_tab.render(inner_area, buf);
            }
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
        use ratatui::widgets::{Block, Borders, List, ListItem};

        let debug_items: Vec<ListItem> = self
            .debug_info
            .iter()
            .map(|info| ListItem::new(info.as_str()))
            .collect();

        let debug_list = List::new(debug_items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Debug Log (<leader>d to toggle)")
                    .border_style(Style::default().fg(Color::Yellow)),
            )
            .style(Style::default().fg(Color::White));

        debug_list.render(area, buf);
    }
}

pub fn render_title(area: Rect, buf: &mut Buffer) {
    "Ratatui Tabs Example".bold().render(area, buf);
}

impl App {
    pub fn render_footer(&self, area: Rect, buf: &mut Buffer) {
        let leader = &self.config.leader;
        let footer_text = match self.selected_tab {
            SelectedTab::Server => {
                if self.creating_profile {
                    "Type profile name and press Enter to create, Esc to cancel"
                } else {
                    "↑↓ to navigate profiles | s to start/stop server | n to create profile | d to delete profile | q to quit"
                }
            }
            _ => &format!(
                "◄ ► to change tab | gt/gT for sequences | Press q to quit | {}+d for debug",
                if leader == " " { "Space" } else { leader }
            ),
        };
        Line::raw(footer_text).centered().render(area, buf);
    }
}
