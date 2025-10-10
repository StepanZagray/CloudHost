use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Widget},
};

use crate::models::App;

pub fn render_settings_tab(app: &App, area: Rect, buf: &mut Buffer) {
    use ratatui::widgets::Paragraph;

    let title = "Settings (FOCUSED)";

    let _block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
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

    // Main content
    let main_block = Block::default()
        .borders(Borders::ALL)
        .title("Password Management");

    let password_status = if app
        .server_state
        .server
        .as_ref()
        .map_or(false, |s| s.has_password())
    {
        "✅ Password is set"
    } else {
        "❌ No password set - Server cannot start"
    };

    let password_instructions = if app.settings_state.creating_password {
        if app.settings_state.password_mode == crate::tabs::settings::models::PasswordMode::Creating
        {
            "Enter new password (min 8 chars):"
        } else if app.settings_state.password_mode
            == crate::tabs::settings::models::PasswordMode::Confirming
        {
            "Confirm password:"
        } else {
            "Creating password..."
        }
    } else {
        "Press 'p' to create new password"
    };

    let password_display = if app.settings_state.password_mode
        != crate::tabs::settings::models::PasswordMode::Normal
    {
        format!(
            "Password: {}",
            "*".repeat(app.settings_state.password_input.len())
        )
    } else {
        "".to_string()
    };

    let error_text = if let Some(ref error) = app.settings_state.password_error {
        format!("Error: {}", error)
    } else if app.settings_state.password_success {
        "✅ Password set successfully!".to_string()
    } else {
        "".to_string()
    };

    let content = format!(
        "{}\n\n{}\n\n{}\n\n{}",
        password_status, password_instructions, password_display, error_text
    );

    Paragraph::new(content)
        .block(main_block)
        .style(Style::default().fg(Color::White))
        .render(chunks[1], buf);

    // Footer is handled by the main app
}
