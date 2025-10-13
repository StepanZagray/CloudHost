use ratatui::{
    buffer::Buffer,
    layout::Rect,
    prelude::Stylize,
    text::Line,
    widgets::{Tabs, Widget},
};
use strum::IntoEnumIterator;

use crate::tabs::{client, focus::TabFocus, server, settings, SelectedTab};
use cloudhost_shared::debug_stream::{get_debug_stream, DebugMessage};

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
    pub server_logs: Vec<DebugMessage>,
    pub debug_receiver:
        Option<std::sync::Arc<std::sync::Mutex<Vec<cloudhost_shared::debug_stream::DebugMessage>>>>,

    // Tab states
    pub server_state: server::models::ServerState,
    pub client_state: client::models::ClientState,
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
        let server_config = Self::load_server_config(&config.server_config_path);

        // Create default server config file if it doesn't exist
        Self::ensure_server_config_file(&config.server_config_path);
        Self {
            config,
            server_state: server::models::ServerState::new_with_config(&server_config),
            client_state: client::models::ClientState::default(),
            settings_state: settings::models::SettingsState::default(),
            debug_receiver: None,
            ..Default::default()
        }
    }

    fn load_server_config(server_config_path: &str) -> cloudhost_server::ServerConfig {
        let expanded_path = if let Some(stripped) = server_config_path.strip_prefix("~/") {
            if let Some(home) = dirs::home_dir() {
                home.join(stripped)
            } else {
                std::path::PathBuf::from(server_config_path)
            }
        } else {
            std::path::PathBuf::from(server_config_path)
        };

        match std::fs::read_to_string(&expanded_path) {
            Ok(content) => toml::from_str::<cloudhost_server::ServerConfig>(&content).unwrap_or_default(),
            Err(_e) => {
                // Server will handle its own logging - just use default config
                cloudhost_server::ServerConfig::default()
            }
        }
    }

    fn ensure_server_config_file(server_config_path: &str) {
        let expanded_path = if let Some(stripped) = server_config_path.strip_prefix("~/") {
            if let Some(home) = dirs::home_dir() {
                home.join(stripped)
            } else {
                std::path::PathBuf::from(server_config_path)
            }
        } else {
            std::path::PathBuf::from(server_config_path)
        };

        // Create the directory if it doesn't exist
        if let Some(parent) = expanded_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        // Create default config file if it doesn't exist
        if !expanded_path.exists() {
            let default_config = cloudhost_server::ServerConfig::default();
            if let Ok(config_str) = toml::to_string_pretty(&default_config) {
                let _ = std::fs::write(&expanded_path, config_str);
            }
        }
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

    pub async fn start_debug_stream_subscription(&mut self) {
        if let Some(debug_stream) = get_debug_stream() {
            let mut receiver = debug_stream.subscribe();

            // Get initial messages from history
            let initial_messages = debug_stream.get_recent(50).await;
            self.server_logs = initial_messages;

            // Spawn a task to listen for new debug messages
            let server_logs = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
            let server_logs_clone = server_logs.clone();

            tokio::spawn(async move {
                while let Ok(message) = receiver.recv().await {
                    if let Ok(mut logs) = server_logs_clone.lock() {
                        logs.push(message);
                        // Keep only last 100 messages
                        if logs.len() > 100 {
                            let excess = logs.len() - 100;
                            logs.drain(0..excess);
                        }
                    }
                }
            });

            // Store the receiver for later use
            self.debug_receiver = Some(server_logs);
        }
    }

    pub async fn update_server_logs(&mut self) {
        if let Some(server_logs) = &self.debug_receiver {
            if let Ok(logs) = server_logs.lock() {
                self.server_logs = logs.clone();

                // Also update the server state's logs for display
                self.server_state.server_logs = logs
                    .iter()
                    .map(|msg| {
                        format!(
                            "[{}] [{}] {}: {}",
                            msg.timestamp.format("%H:%M:%S"),
                            msg.level,
                            msg.source,
                            msg.message
                        )
                    })
                    .collect();
            }
        }
    }

    // Tab-specific focus management
    pub fn cycle_focus_forward(&mut self) {
        match self.selected_tab {
            SelectedTab::Server => self.server_state.cycle_focus_forward(),
            SelectedTab::Client => self.client_state.cycle_focus_forward(),
            SelectedTab::Settings => self.settings_state.cycle_focus_forward(),
        }
    }

    pub fn cycle_focus_backward(&mut self) {
        match self.selected_tab {
            SelectedTab::Server => self.server_state.cycle_focus_backward(),
            SelectedTab::Client => self.client_state.cycle_focus_backward(),
            SelectedTab::Settings => self.settings_state.cycle_focus_backward(),
        }
    }

    pub fn get_current_focused_element(&self) -> String {
        match self.selected_tab {
            SelectedTab::Server => self.server_state.get_focused_element(),
            SelectedTab::Client => self.client_state.get_focused_element(),
            SelectedTab::Settings => self.settings_state.get_focused_element(),
        }
    }

    async fn complete_password_creation(&mut self) {
        // Set the password
        if let Some(ref mut server) = self.server_state.server {
            if let Err(e) = server.set_password(&self.settings_state.password_input) {
                self.settings_state.password_error = Some(format!("Failed to set password: {}", e));
            } else {
                self.settings_state.clear_password_creation_input();
                self.settings_state.password_success = true;
                self.add_debug("Password set successfully");

                // If server is running, restart it to pick up the new AuthState
                let was_running = self.server_state.is_server_running();
                let port = self.server_state.server_port;

                if was_running {
                    self.add_debug("Stopping server to apply password changes");
                    self.server_state.stop_server().await;
                }

                // Recreate the server instance to pick up the new config
                self.add_debug("Recreating server instance with new config");
                self.server_state.server = Some(cloudhost_server::CloudServer::new());

                // If server was running, restart it automatically
                if was_running {
                    if let Some(server_port) = port {
                        self.add_debug(&format!("Restarting server on port {}", server_port));
                        self.server_state.start_server();
                        self.add_debug("Server restart initiated with new password");
                    }
                }
            }
        } else {
            self.settings_state.password_error = Some("Server not available".to_string());
        }
    }

    // Tab-specific navigation methods
    pub fn handle_tab_navigation(&mut self, key: ratatui::crossterm::event::KeyCode) -> bool {
        match self.selected_tab {
            SelectedTab::Server => self.server_state.handle_navigation(key),
            SelectedTab::Client => self.client_state.handle_navigation(key),
            SelectedTab::Settings => self.settings_state.handle_navigation(key),
        }
    }

    pub async fn handle_dynamic_key(
        &mut self,
        key: ratatui::crossterm::event::KeyCode,
        modifiers: ratatui::crossterm::event::KeyModifiers,
    ) {
        use ratatui::crossterm::event::KeyCode;

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

        // Handle special cases first (cloudfolder creation)
        if self.server_state.creating_cloudfolder {
            self.server_state.handle_cloudfolder_input(key);
            return;
        }

        // Handle password creation modal
        if self.settings_state.creating_password && self.settings_state.handle_password_input(key) {
            // If password creation is complete, handle it
            if self.settings_state.password_mode
                == crate::tabs::settings::models::PasswordMode::Confirming
                && self.settings_state.password_input == self.settings_state.password_confirm
            {
                self.complete_password_creation().await;
            }
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
                if let Some(keybinding) = self.config.get_keybinding(seq) {
                    if self.config.is_key_valid_for_tab(seq, current_tab) {
                        let action = keybinding.action.clone();
                        self.execute_action(&action).await;
                    }
                }
                self.input_state = InputState::Normal;
                return;
            }

            if seq == "<leader>" {
                let leader_key = format!("<leader>{}", key_str);
                if let Some(keybinding) = self.config.get_keybinding(&leader_key) {
                    if self.config.is_key_valid_for_tab(&leader_key, current_tab) {
                        let action = keybinding.action.clone();
                        self.execute_action(&action).await;
                    }
                }
                self.input_state = InputState::Normal;
                return;
            } else {
                // Try to complete the sequence with the current key
                let complete_key = format!("{}{}", seq, key_str);
                if let Some(keybinding) = self.config.get_keybinding(&complete_key) {
                    if self.config.is_key_valid_for_tab(&complete_key, current_tab) {
                        let action = keybinding.action.clone();
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
            .keybindings
            .keys()
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
        if let Some(keybinding) = self.config.get_keybinding(&key_str) {
            if self.config.is_key_valid_for_tab(&key_str, current_tab) {
                let action = keybinding.action.clone();
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
            "Start/Stop Server" => {
                if self.server_state.is_server_running() {
                    self.server_state.stop_server().await;
                } else {
                    self.server_state.start_server();
                }
            }
            "Create Cloud Folder" => self.server_state.start_creating_cloudfolder(),
            "Delete cloud Folder" => self.server_state.delete_selected_cloudfolder(),
            "Create Password" => self.settings_state.start_creating_password(),
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
                if let Some(keybinding) = self.config.get_keybinding(seq) {
                    let current_tab = match self.selected_tab {
                        SelectedTab::Server => "server",
                        SelectedTab::Client => "client",
                        SelectedTab::Settings => "settings",
                    };
                    if self.config.is_key_valid_for_tab(seq, current_tab) {
                        let action = keybinding.action.clone();
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
                Constraint::Length(8), // Debug panel
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
        let server_logs_area = Rect {
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
        let header_text = format!(
            "TUI Debug ({} messages) | Server logs are in Server tab",
            self.debug_info.len()
        );
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

        ratatui::widgets::Widget::render(list, server_logs_area, buf);
    }
}

pub fn render_title(area: Rect, buf: &mut Buffer) {
    "CloudTUI".bold().render(area, buf);
}

impl App {
    pub fn render_footer(&self, area: Rect, buf: &mut Buffer) {
        let leader = &self.config.leader;
        let footer_text = match self.selected_tab {
            SelectedTab::Server => {
                if self.server_state.creating_cloudfolder {
                    "Type cloudfolder name and folder path, press Tab to switch fields, Enter to confirm, Esc to cancel"
                } else {
                    "j/k to navigate cloudfolders | s to start/stop server | n to create cloudfolder | d to delete cloudfolder | q to quit"
                }
            }
            SelectedTab::Settings => {
                if self.settings_state.creating_password {
                    "Type password, press Enter to confirm, Esc to cancel"
                } else {
                    "p to create password | q to quit"
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
