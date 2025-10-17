use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Clear, Paragraph, Widget},
};

use crate::utils::password::{PasswordCreationState, PasswordMode};

/// Renders a password creation modal
pub fn render_password_modal(
    title: &str,
    cloud_name: &str,
    password_state: &PasswordCreationState,
    area: Rect,
    buf: &mut Buffer,
) {
    use ratatui::style::Modifier;

    // Modal dimensions
    let modal_width = 60.min(area.width.saturating_sub(4));
    let modal_height = 15.min(area.height.saturating_sub(4));

    let modal_area = Rect {
        x: (area.width.saturating_sub(modal_width)) / 2,
        y: (area.height.saturating_sub(modal_height)) / 2,
        width: modal_width,
        height: modal_height,
    };

    // Clear the area behind the modal
    Clear.render(modal_area, buf);

    // Create vertical layout for modal content
    let modal_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Length(2), // Cloud name display
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
        .title(title)
        .title_alignment(Alignment::Center)
        .border_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );

    Paragraph::new("")
        .block(title_block)
        .render(modal_area, buf);

    // Cloud name display
    Paragraph::new(format!("Cloud: {}", cloud_name))
        .style(Style::default().fg(Color::Cyan))
        .alignment(Alignment::Center)
        .render(modal_chunks[1], buf);

    // Instructions
    let instructions = match password_state.get_password_mode() {
        PasswordMode::Creating => "Enter new password (min 8 characters):",
        PasswordMode::Confirming => "Confirm your password:",
    };

    Paragraph::new(instructions)
        .style(Style::default().fg(Color::Cyan))
        .alignment(Alignment::Center)
        .render(modal_chunks[2], buf);

    // Password field
    let password_field = format!("Password: {}", password_state.get_masked_password());
    let password_style = if password_state.get_password_mode() == &PasswordMode::Creating {
        Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };

    Paragraph::new(password_field)
        .style(password_style)
        .alignment(Alignment::Center)
        .render(modal_chunks[3], buf);

    // Confirm field (only show when confirming)
    if password_state.get_password_mode() == &PasswordMode::Confirming {
        let confirm_field = format!("Confirm: {}", password_state.get_masked_confirm());
        Paragraph::new(confirm_field)
            .style(
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center)
            .render(modal_chunks[4], buf);
    }

    // Error message or help text
    if let Some(error) = password_state.get_error() {
        Paragraph::new(format!("❌ {}", error))
            .style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
            .alignment(Alignment::Center)
            .render(modal_chunks[5], buf);
    } else if password_state.get_password_mode() == &PasswordMode::Creating {
        Paragraph::new("Enter password (min 8 characters) and press Enter")
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center)
            .render(modal_chunks[5], buf);
    } else {
        Paragraph::new("Confirm password and press Enter")
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center)
            .render(modal_chunks[5], buf);
    }
}
