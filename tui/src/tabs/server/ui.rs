use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{
        Block, List, ListItem, ListState, Paragraph, Scrollbar, ScrollbarOrientation,
        StatefulWidget, Widget,
    },
};

use crate::models::App;
use crate::tabs::server::models::FocusedPanel;

pub fn render_server_tab(app: &mut App, area: Rect, buf: &mut Buffer) {
    // Ensure we have enough space for borders
    if area.height < 8 || area.width < 20 {
        return;
    }

    // Create 3-column layout: cloudfolders (15%), server info (35%), server logs (50%)
    let three_column_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length((area.width * 15 / 100).max(15)), // Left: cloudfolders (15% or min 15 chars)
            Constraint::Length((area.width * 35 / 100).max(30)), // Middle: server info (35% or min 30 chars)
            Constraint::Min(30), // Right: server logs (50% - remaining space)
        ])
        .split(area);

    // Left side: Cloudfolder list
    let cloudfolder_items: Vec<ListItem> = app
        .server_state
        .cloudfolders
        .iter()
        .enumerate()
        .map(|(i, cloudfolder)| {
            let style = if i == app.server_state.selected_cloudfolder_index {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            };
            ListItem::new(cloudfolder.name.as_str()).style(style)
        })
        .collect();

    let mut list_state = ListState::default();
    list_state.select(Some(app.server_state.selected_cloudfolder_index));

    // Add focus indicator to title
    let cloudfolder_title = if app.server_state.focused_panel == FocusedPanel::CloudFolders {
        "Cloud folders (FOCUSED)"
    } else {
        "Cloud folders"
    };

    let cloudfolder_list = List::new(cloudfolder_items)
        .block(
            Block::default()
                .borders(ratatui::widgets::Borders::ALL)
                .title(cloudfolder_title)
                .title_alignment(Alignment::Left)
                .border_style(
                    if app.server_state.focused_panel == FocusedPanel::CloudFolders {
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(ratatui::style::Modifier::BOLD)
                    } else {
                        Style::default()
                    },
                ),
        )
        .highlight_style(Style::default().fg(Color::Yellow));

    Widget::render(cloudfolder_list, three_column_chunks[0], buf);

    // Right side: Server controls and info
    let server_status = if app.server_state.is_server_running() {
        if let Some(port) = app.server_state.get_server_port() {
            format!("🟢 Running (port {})", port)
        } else {
            "🟢 Running".to_string()
        }
    } else {
        "🔴 Not Running".to_string()
    };

    let server_info = if let Some(cloudfolder) = app
        .server_state
        .cloudfolders
        .get(app.server_state.selected_cloudfolder_index)
    {
        let cloudfolder_url = if app.server_state.is_server_running() {
            if let Some(port) = app.server_state.get_server_port() {
                format!("http://127.0.0.1:{}/{}", port, cloudfolder.name)
            } else {
                "Server running".to_string()
            }
        } else {
            "Server not running".to_string()
        };

        format!(
            "Selected Cloudfolder: {}\nPath: {}\nURL: {}\nTo add files to this cloudfolder,\nadd them to the cloudfolder folder manually\nStatus: {}",
            cloudfolder.name,
            cloudfolder.folder_path.display(),
            cloudfolder_url,
            server_status,
        )
    } else {
        "No cloudfolders available".to_string()
    };

    // Add focus indicator to server info title
    let server_info_title = if app.server_state.focused_panel == FocusedPanel::ServerInfo {
        "Server Info (FOCUSED)"
    } else {
        "Server Info"
    };

    let server_block = Paragraph::new(server_info)
        .block(
            Block::default()
                .borders(ratatui::widgets::Borders::ALL)
                .title(server_info_title)
                .title_alignment(Alignment::Left)
                .border_style(
                    if app.server_state.focused_panel == FocusedPanel::ServerInfo {
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(ratatui::style::Modifier::BOLD)
                    } else {
                        Style::default()
                    },
                ),
        )
        .alignment(ratatui::layout::Alignment::Left);

    server_block.render(three_column_chunks[1], buf);

    // Server logs section with scrolling
    let logs = &app.server_logs; // Use the main app's server logs instead of server_state

    // Ensure the logs area has enough space
    if three_column_chunks[2].height < 3 {
        return;
    }

    let visible_logs: Vec<ListItem> = logs
        .iter()
        .map(|log| {
            let level_color = match log.level {
                cloudhost_shared::debug_stream::LogLevel::Error => Color::Red,
                cloudhost_shared::debug_stream::LogLevel::Warning => Color::Yellow,
                cloudhost_shared::debug_stream::LogLevel::Info => Color::Green,
                cloudhost_shared::debug_stream::LogLevel::Debug => Color::Blue,
            };

            let formatted_msg = format!(
                "[{}] [{}] {}: {}",
                log.timestamp.format("%H:%M:%S%.3f"),
                log.level,
                log.source,
                log.message
            );

            ListItem::new(formatted_msg).style(Style::default().fg(level_color))
        })
        .collect();

    // Add focus indicator to title
    let logs_title = if app.server_state.focused_panel == FocusedPanel::ServerLogs {
        "Server Logs (FOCUSED)"
    } else {
        "Server Logs"
    };

    let server_logs = List::new(visible_logs.clone())
        .block(
            Block::default()
                .borders(ratatui::widgets::Borders::ALL)
                .title(logs_title)
                .title_alignment(Alignment::Left)
                .border_style(
                    if app.server_state.focused_panel == FocusedPanel::ServerLogs {
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(ratatui::style::Modifier::BOLD)
                    } else {
                        Style::default()
                    },
                ),
        )
        .style(Style::default().fg(Color::Green))
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(ratatui::style::Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    // Use the persistent ListState from ServerState
    let mut list_state = app.server_state.server_logs_list_state.clone();
    if !visible_logs.is_empty() && list_state.selected().is_none() {
        // Select the last item (newest log) by default if nothing is selected
        let selected_index = visible_logs.len().saturating_sub(1);
        list_state.select(Some(selected_index));
    }

    // Render the list with state
    StatefulWidget::render(server_logs, three_column_chunks[2], buf, &mut list_state);

    // Update the persistent state
    app.server_state.server_logs_list_state = list_state;

    // Render scrollbar
    let mut scroll_state = app.server_state.server_logs_scroll_state.clone();
    scroll_state = scroll_state.content_length(visible_logs.len());
    if let Some(selected) = app.server_state.server_logs_list_state.selected() {
        scroll_state = scroll_state.position(selected);
    }

    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(Some("↑"))
        .end_symbol(Some("↓"));
    scrollbar.render(three_column_chunks[2], buf, &mut scroll_state);

    // Update the persistent scroll state
    app.server_state.server_logs_scroll_state = scroll_state;

    // Show popup if creating cloudfolder
    if app.server_state.creating_cloudfolder {
        render_cloudfolder_creation_popup(app, area, buf);
    }
}

pub fn render_cloudfolder_creation_popup(app: &App, area: Rect, buf: &mut Buffer) {
    use ratatui::layout::{Alignment, Constraint, Layout};
    use ratatui::style::{Color, Style};
    use ratatui::widgets::{Block, Borders, Clear, Paragraph};

    // Create a centered popup
    let popup_area = Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            Constraint::Percentage(20),
            Constraint::Length(12),
            Constraint::Percentage(20),
        ])
        .split(area)[1];

    let popup_inner = Layout::default()
        .direction(ratatui::layout::Direction::Horizontal)
        .constraints([
            Constraint::Percentage(15),
            Constraint::Percentage(70),
            Constraint::Percentage(15),
        ])
        .split(popup_area)[1];

    // Clear the background
    Clear.render(popup_inner, buf);

    // Create content with both fields
    let name_field = if app.server_state.cloudfolder_input_field
        == crate::tabs::server::models::CloudFolderInputField::Name
    {
        format!("Name: {}_", app.server_state.new_cloudfolder_name)
    } else {
        format!("Name: {}", app.server_state.new_cloudfolder_name)
    };

    let path_field = if app.server_state.cloudfolder_input_field
        == crate::tabs::server::models::CloudFolderInputField::Path
    {
        format!("Path: {}_", app.server_state.new_cloudfolder_path)
    } else {
        format!("Path: {}", app.server_state.new_cloudfolder_path)
    };

    let popup_content = if let Some(error) = &app.server_state.cloudfolder_creation_error {
        format!(
            "Create New Cloudfolder\n\n{}\n{}\n\nError: {}",
            name_field, path_field, error
        )
    } else {
        format!(
            "Create New Cloudfolder\n\n{}\n{}\n\nPress Tab to switch fields, Enter to confirm, Esc to cancel",
            name_field, path_field
        )
    };

    let popup = Paragraph::new(popup_content)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("New Cloudfolder")
                .title_alignment(Alignment::Center)
                .border_style(Style::default().fg(Color::Yellow)),
        )
        .alignment(Alignment::Left)
        .style(Style::default().fg(Color::White));

    popup.render(popup_inner, buf);
}
