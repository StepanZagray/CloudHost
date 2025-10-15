use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{
        Block, Borders, Clear, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation,
        StatefulWidget, Widget,
    },
};

use crate::models::App;
use crate::tabs::settings::models::ConfigFolder;

pub fn render_settings_tab(app: &mut App, area: Rect, buf: &mut Buffer) {
    // Ensure we have enough space for borders
    if area.height < 8 || area.width < 20 {
        return;
    }

    // Create single column layout with sections
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(0),    // Main content
        ])
        .split(area);

    // Header
    Paragraph::new("CloudTUI Settings")
        .style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .render(chunks[0], buf);

    // Main content area
    render_settings_content(app, chunks[1], buf);

    // Render password creation modal if active
    if app.settings_state.creating_password {
        render_password_creation_modal(app, area, buf);
    }
}

fn render_settings_content(app: &mut App, area: Rect, buf: &mut Buffer) {
    // Create sections layout
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8), // Password Management
            Constraint::Length(6), // Config Folders
            Constraint::Min(0),    // Info section
        ])
        .split(area);

    // Password Management Section
    render_password_section(app, sections[0], buf);

    // Config Folders Section
    render_config_folders_section(app, sections[1], buf);

    // Info Section
    render_info_section(app, sections[2], buf);
}

fn render_password_section(app: &App, area: Rect, buf: &mut Buffer) {
    let password_status = if app
        .server_state
        .server
        .as_ref()
        .is_some_and(|s| s.has_password())
    {
        "✅ Password is set"
    } else {
        "❌ No password set - Server cannot start"
    };

    let password_instructions = if app.settings_state.creating_password {
        match app.settings_state.password_mode {
            crate::tabs::settings::models::PasswordMode::Creating => {
                "🔐 Creating password... (Enter password)"
            }
            crate::tabs::settings::models::PasswordMode::Confirming => {
                "🔐 Confirming password... (Re-enter password)"
            }
            _ => "🔐 Creating password...",
        }
    } else {
        "Press 'p' to create new password"
    };

    let error_text = if let Some(ref error) = app.settings_state.password_error {
        format!("Error: {}", error)
    } else if app.settings_state.password_success {
        "✅ Password set successfully!".to_string()
    } else {
        "".to_string()
    };

    let content = format!(
        "{}\n\n{}\n\n{}",
        password_status, password_instructions, error_text
    );

    let title = "Password Management";
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .title_alignment(Alignment::Left);

    Paragraph::new(content)
        .block(block)
        .style(Style::default().fg(Color::White))
        .render(area, buf);
}

fn render_config_folders_section(app: &mut App, area: Rect, buf: &mut Buffer) {
    let config_folders = [
        ("📁 Server Config", ConfigFolder::ServerConfig),
        ("📁 TUI Config", ConfigFolder::TuiConfig),
    ];

    let items: Vec<ListItem> = config_folders
        .iter()
        .map(|(name, _)| ListItem::new(*name))
        .collect();

    // Ensure we have a selection if none exists
    if app
        .settings_state
        .config_folders_list_state
        .selected()
        .is_none()
    {
        app.settings_state.config_folders_list_state.select(Some(0));
        app.settings_state.selected_config_folder = Some(ConfigFolder::ServerConfig);
    }

    let title = "Config Folders (Press Enter to open)";
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .title_alignment(Alignment::Left);

    let list = List::new(items)
        .block(block)
        .highlight_style(Style::default().fg(Color::Yellow))
        .highlight_symbol(">> ");

    // Render the list with persistent state
    StatefulWidget::render(
        list,
        area,
        buf,
        &mut app.settings_state.config_folders_list_state,
    );

    // Render scrollbar
    let mut scroll_state = app.settings_state.config_folders_scroll_state;
    scroll_state = scroll_state.content_length(config_folders.len());
    if let Some(selected) = app.settings_state.config_folders_list_state.selected() {
        scroll_state = scroll_state.position(selected);
    }

    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(Some("↑"))
        .end_symbol(Some("↓"));
    scrollbar.render(area, buf, &mut scroll_state);

    // Update the persistent scroll state
    app.settings_state.config_folders_scroll_state = scroll_state;
}

fn render_info_section(_app: &App, area: Rect, buf: &mut Buffer) {
    let is_dev_mode = cloudhost_shared::config_paths::is_dev_mode();
    let server_config_path = cloudhost_shared::config_paths::get_server_config_path();
    let tui_config_path = cloudhost_shared::config_paths::get_tui_config_path();

    let mode_info = if is_dev_mode {
        "Development Mode"
    } else {
        "Production Mode"
    };

    let content = format!(
        "Mode: {}\n\nConfig Files:\n• Server: {}\n• TUI: {}\n\nNavigation:\n• Tab/Shift+Tab: Navigate\n• Enter: Open folder\n• p: Create password",
        mode_info,
        server_config_path.display(),
        tui_config_path.display()
    );

    let title = "Settings Info";
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .title_alignment(Alignment::Left);

    Paragraph::new(content)
        .block(block)
        .style(Style::default().fg(Color::White))
        .render(area, buf);
}

fn render_password_creation_modal(app: &App, area: Rect, buf: &mut Buffer) {
    // Create a centered modal
    let modal_width = 60;
    let modal_height = 14;
    let x = (area.width.saturating_sub(modal_width)) / 2;
    let y = (area.height.saturating_sub(modal_height)) / 2;

    let modal_area = Rect::new(area.x + x, area.y + y, modal_width, modal_height);

    // Clear the modal area
    Clear.render(modal_area, buf);

    // Create modal layout
    let modal_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Length(2), // Instructions
            Constraint::Length(2), // Password field
            Constraint::Length(2), // Confirm field
            Constraint::Length(2), // Error/Help
            Constraint::Min(0),    // Spacer
        ])
        .split(modal_area);

    // Modal title with better styling
    let title_block = Block::default()
        .borders(Borders::ALL)
        .title("🔐 Create Password")
        .title_alignment(Alignment::Center)
        .border_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );

    Paragraph::new("")
        .block(title_block)
        .render(modal_area, buf);

    // Instructions
    let instructions = match app.settings_state.password_mode {
        crate::tabs::settings::models::PasswordMode::Creating => {
            "Enter new password (min 8 characters):"
        }
        crate::tabs::settings::models::PasswordMode::Confirming => "Confirm your password:",
        _ => "Creating password...",
    };

    Paragraph::new(instructions)
        .style(Style::default().fg(Color::Cyan))
        .alignment(Alignment::Center)
        .render(modal_chunks[1], buf);

    // Password field
    let password_field = format!(
        "Password: {}",
        "*".repeat(app.settings_state.password_input.len())
    );
    let password_style = if app.settings_state.password_mode
        == crate::tabs::settings::models::PasswordMode::Creating
    {
        Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };

    Paragraph::new(password_field)
        .style(password_style)
        .alignment(Alignment::Center)
        .render(modal_chunks[2], buf);

    // Confirm field (only show when confirming)
    if app.settings_state.password_mode == crate::tabs::settings::models::PasswordMode::Confirming {
        let confirm_field = format!(
            "Confirm: {}",
            "*".repeat(app.settings_state.password_confirm.len())
        );

        Paragraph::new(confirm_field)
            .style(
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center)
            .render(modal_chunks[3], buf);
    }

    // Error message or help text
    if let Some(ref error) = app.settings_state.password_error {
        Paragraph::new(format!("❌ {}", error))
            .style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
            .alignment(Alignment::Center)
            .render(modal_chunks[4], buf);
    } else {
        let help_text = match app.settings_state.password_mode {
            crate::tabs::settings::models::PasswordMode::Creating => {
                "Press Enter when done, Esc to cancel"
            }
            crate::tabs::settings::models::PasswordMode::Confirming => {
                "Press Enter to confirm, Esc to cancel"
            }
            _ => "Press Enter to continue, Esc to cancel",
        };

        Paragraph::new(help_text)
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center)
            .render(modal_chunks[4], buf);
    }
}
