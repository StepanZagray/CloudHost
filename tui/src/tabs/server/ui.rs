use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, List, ListItem, ListState, Paragraph, Widget},
};

use crate::models::App;
use crate::tabs::server::models::FocusedPanel;

pub fn render_server_tab(app: &App, area: Rect, buf: &mut Buffer) {
    // Ensure we have enough space for borders
    if area.height < 8 || area.width < 20 {
        return;
    }

    // Create horizontal split: left (profiles) and right (server info + logs)
    let horizontal_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length((area.width * 40 / 100).max(20)), // Left: profiles (40% or min 20 chars)
            Constraint::Min(30), // Right: server info + logs (remaining space)
        ])
        .split(area);

    // Split the right side vertically: server info (top) and logs (bottom)
    let right_vertical_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(10), // Server info - fixed height
            Constraint::Min(5),     // Server logs - remaining space
        ])
        .split(horizontal_chunks[1]);

    // Left side: Profile list
    let profile_items: Vec<ListItem> = app
        .server_state
        .profiles
        .iter()
        .enumerate()
        .map(|(i, profile)| {
            let style = if i == app.server_state.selected_profile_index {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            };
            ListItem::new(profile.name.as_str()).style(style)
        })
        .collect();

    let mut list_state = ListState::default();
    list_state.select(Some(app.server_state.selected_profile_index));

    // Add focus indicator to title
    let profile_title = if app.server_state.focused_panel == FocusedPanel::Profiles {
        "Profiles (FOCUSED)"
    } else {
        "Profiles"
    };

    let profile_list = List::new(profile_items)
        .block(
            Block::default()
                .borders(ratatui::widgets::Borders::ALL)
                .title(profile_title)
                .title_alignment(Alignment::Left)
                .border_style(
                    if app.server_state.focused_panel == FocusedPanel::Profiles {
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(ratatui::style::Modifier::BOLD)
                    } else {
                        Style::default()
                    },
                ),
        )
        .highlight_style(Style::default().fg(Color::Yellow));

    profile_list.render(horizontal_chunks[0], buf);

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

    let running_servers_count = app.server_state.get_running_servers_count();
    let server_info = if let Some(profile) = app
        .server_state
        .profiles
        .get(app.server_state.selected_profile_index)
    {
        format!(
            "Selected Profile: {}\nPath: {}\nTo add files to this profile, add them to the profile folder manually\nStatus: {}\n\nRunning Servers: {}",
            profile.name,
            profile.folder_path.display(),
            server_status,
            running_servers_count
        )
    } else {
        "No profiles available".to_string()
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

    server_block.render(right_vertical_chunks[0], buf);

    // Server logs section with scrolling
    let logs = &app.server_state.server_logs;
    let scroll_offset = app.server_state.log_scroll_offset;

    // Ensure the logs area has enough space
    if right_vertical_chunks[1].height < 3 {
        return;
    }

    // Calculate how many logs can fit in the available height
    let available_height = right_vertical_chunks[1].height.saturating_sub(2); // Subtract border height
    let max_scroll_offset = if logs.len() > available_height as usize {
        logs.len() - available_height as usize
    } else {
        0
    };

    // Limit scroll offset to prevent scrolling past the newest logs
    let effective_scroll_offset = scroll_offset.min(max_scroll_offset);

    let visible_logs: Vec<ListItem> = logs
        .iter()
        .skip(effective_scroll_offset) // Skip older logs at the top
        .map(|log| ListItem::new(log.as_str()))
        .collect();

    // Add focus indicator to title
    let logs_title = if app.server_state.focused_panel == FocusedPanel::ServerLogs {
        "Server Logs (FOCUSED)"
    } else {
        "Server Logs"
    };

    let server_logs = List::new(visible_logs)
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
        .style(Style::default().fg(Color::Green));

    server_logs.render(right_vertical_chunks[1], buf);

    // Show popup if creating profile
    if app.server_state.creating_profile {
        render_profile_creation_popup(app, area, buf);
    }
}

pub fn render_profile_creation_popup(app: &App, area: Rect, buf: &mut Buffer) {
    use ratatui::layout::{Alignment, Constraint, Layout};
    use ratatui::style::{Color, Style};
    use ratatui::widgets::{Block, Borders, Clear, Paragraph};

    // Create a centered popup
    let popup_area = Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Length(8),
            Constraint::Percentage(25),
        ])
        .split(area)[1];

    let popup_inner = Layout::default()
        .direction(ratatui::layout::Direction::Horizontal)
        .constraints([
            Constraint::Percentage(20),
            Constraint::Percentage(60),
            Constraint::Percentage(20),
        ])
        .split(popup_area)[1];

    // Clear the background
    Clear.render(popup_inner, buf);

    let popup_content = if let Some(error) = &app.server_state.profile_creation_error {
        format!(
            "Create New Profile\n\nName: {}\n\nError: {}",
            app.server_state.new_profile_name, error
        )
    } else {
        format!(
            "Create New Profile\n\nName: {}",
            app.server_state.new_profile_name
        )
    };

    let popup = Paragraph::new(popup_content)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("New Profile")
                .title_alignment(Alignment::Center)
                .border_style(Style::default().fg(Color::Yellow)),
        )
        .alignment(Alignment::Left)
        .style(Style::default().fg(Color::White));

    popup.render(popup_inner, buf);
}
