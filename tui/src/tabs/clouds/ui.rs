use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{
        Block, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation, StatefulWidget, Widget,
    },
};

use crate::components::password_modal::render_password_modal;
use crate::models::App;
use crate::tabs::clouds::models::CloudFocusedPanel;

pub fn render_servers_tab(app: &mut App, area: Rect, buf: &mut Buffer) {
    // Ensure we have enough space for borders
    if area.height < 8 || area.width < 20 {
        return;
    }

    // Create 3-column layout: clouds (15%), server info (35%), server logs (50%)
    let three_column_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length((area.width * 15 / 100).max(25)), // Left: clouds (15% or min 15 chars)
            Constraint::Length((area.width * 35 / 100).max(45)), // Middle: server info (35% or min 30 chars)
            Constraint::Min(30), // Right: server logs (50% - remaining space)
        ])
        .split(area);

    // Create clouds list items
    let cloud_items: Vec<ListItem> = app
        .clouds_state
        .clouds
        .iter()
        .enumerate()
        .map(|(i, cloud)| {
            let style = if i == app.clouds_state.selected_cloud_index {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            };
            ListItem::new(cloud.name.as_str()).style(style)
        })
        .collect();

    // Add focus indicator to title
    let cloud_title = if app.clouds_state.focused_panel == CloudFocusedPanel::Clouds {
        "Clouds (FOCUSED)"
    } else {
        "Clouds"
    };

    // Ensure we have a selection if none exists
    if app.clouds_state.clouds_list_state.selected().is_none() && !cloud_items.is_empty() {
        app.clouds_state.clouds_list_state.select(Some(0));
    }

    let cloud_list = List::new(cloud_items.clone())
        .block(
            Block::default()
                .borders(ratatui::widgets::Borders::ALL)
                .title(cloud_title)
                .title_alignment(Alignment::Left)
                .border_style(
                    if app.clouds_state.focused_panel == CloudFocusedPanel::Clouds {
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(ratatui::style::Modifier::BOLD)
                    } else {
                        Style::default()
                    },
                ),
        )
        .highlight_style(Style::default().fg(Color::Yellow))
        .highlight_symbol(">> ");

    // Render the list with persistent state
    StatefulWidget::render(
        cloud_list,
        three_column_chunks[0],
        buf,
        &mut app.clouds_state.clouds_list_state,
    );

    // Render scrollbar for clouds
    let mut scroll_state = app.clouds_state.clouds_scroll_state;
    scroll_state = scroll_state.content_length(cloud_items.len());
    if let Some(selected) = app.clouds_state.clouds_list_state.selected() {
        scroll_state = scroll_state.position(selected);
    }

    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(Some("↑"))
        .end_symbol(Some("↓"));
    scrollbar.render(three_column_chunks[0], buf, &mut scroll_state);

    // Update the persistent scroll state
    app.clouds_state.clouds_scroll_state = scroll_state;

    // Right side: Server controls and info
    let cloud_info = if let Some(cloud) = app
        .clouds_state
        .clouds
        .get(app.clouds_state.selected_cloud_index)
    {
        let is_running = if !app.clouds_state.clouds.is_empty()
            && app.clouds_state.selected_cloud_index < app.clouds_state.clouds.len()
        {
            app.clouds_state.is_cloud_running(
                &app.clouds_state.clouds[app.clouds_state.selected_cloud_index].name,
            )
        } else {
            false
        };
        let cloud_status = if is_running {
            if let Some(port) = app.clouds_state.get_cloud_port(&cloud.name) {
                format!("🟢 Running (port {})", port)
            } else {
                "🟢 Running".to_string()
            }
        } else {
            "🔴 Not Running".to_string()
        };

        let cloud_url = if is_running {
            app.orchestrator
                .get_cloud_server_url(&cloud.name)
                .unwrap_or_else(|| "Cloud running".to_string())
        } else {
            "Cloud not running".to_string()
        };

        let mut info = format!(
            "Selected Cloud: {}\nCloud Folders: {}\nURL: {}\nTo add files to this cloud,\nadd them to the cloud folders manually\nStatus: {}",
            cloud.name,
            cloud.cloud_folders.len(),
            cloud_url,
            cloud_status,
        );

        // Add server start error if present
        if let Some(ref error) = app.clouds_state.cloud_start_error {
            info.push_str(&format!("\n\n{}", error));
        }

        info
    } else {
        let mut info = "No clouds available".to_string();

        // Add server start error if present
        if let Some(ref error) = app.clouds_state.cloud_start_error {
            info.push_str(&format!("\n\n{}", error));
        }

        info
    };

    // Add focus indicator to cloud info title
    let cloud_info_title = if app.clouds_state.focused_panel == CloudFocusedPanel::CloudInfo {
        "Cloud Info (FOCUSED)"
    } else {
        "Cloud Info"
    };

    let cloud_block = Paragraph::new(cloud_info)
        .block(
            Block::default()
                .borders(ratatui::widgets::Borders::ALL)
                .title(cloud_info_title)
                .title_alignment(Alignment::Left)
                .border_style(
                    if app.clouds_state.focused_panel == CloudFocusedPanel::CloudInfo {
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(ratatui::style::Modifier::BOLD)
                    } else {
                        Style::default()
                    },
                ),
        )
        .alignment(ratatui::layout::Alignment::Left);

    cloud_block.render(three_column_chunks[1], buf);

    // Cloud logs section with scrolling
    let logs = &app.cloud_logs; // Use the main app's cloud logs

    // Ensure the logs area has enough space
    if three_column_chunks[2].height < 3 {
        return;
    }

    let visible_logs: Vec<ListItem> = logs
        .iter()
        .map(|log| {
            let level_color = match log.level {
                cloudhost_server::debug_stream::LogLevel::Error => Color::Red,
                cloudhost_server::debug_stream::LogLevel::Warning => Color::Yellow,
                cloudhost_server::debug_stream::LogLevel::Info => Color::Green,
                cloudhost_server::debug_stream::LogLevel::Debug => Color::Blue,
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
    let logs_title = if app.clouds_state.focused_panel == CloudFocusedPanel::CloudLogs {
        "Cloud Logs (FOCUSED)"
    } else {
        "Cloud Logs"
    };

    let cloud_logs_list = List::new(visible_logs.clone())
        .block(
            Block::default()
                .borders(ratatui::widgets::Borders::ALL)
                .title(logs_title)
                .title_alignment(Alignment::Left)
                .border_style(
                    if app.clouds_state.focused_panel == CloudFocusedPanel::CloudLogs {
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
    if !visible_logs.is_empty() && app.clouds_state.cloud_logs_list_state.selected().is_none() {
        // Select the last item (newest log) by default if nothing is selected
        let selected_index = visible_logs.len().saturating_sub(1);
        app.clouds_state
            .cloud_logs_list_state
            .select(Some(selected_index));
    }

    // Render the list with state
    StatefulWidget::render(
        cloud_logs_list,
        three_column_chunks[2],
        buf,
        &mut app.clouds_state.cloud_logs_list_state,
    );

    // Render scrollbar
    let mut scroll_state = app.clouds_state.cloud_logs_scroll_state;
    scroll_state = scroll_state.content_length(visible_logs.len());
    if let Some(selected) = app.clouds_state.cloud_logs_list_state.selected() {
        scroll_state = scroll_state.position(selected);
    }

    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(Some("↑"))
        .end_symbol(Some("↓"));
    scrollbar.render(three_column_chunks[2], buf, &mut scroll_state);

    // Update the persistent scroll state
    app.clouds_state.cloud_logs_scroll_state = scroll_state;

    // Render password creation modal if active
    if app.clouds_state.password_creation.creating_password {
        let cloud_name = if !app.clouds_state.clouds.is_empty()
            && app.clouds_state.selected_cloud_index < app.clouds_state.clouds.len()
        {
            &app.clouds_state.clouds[app.clouds_state.selected_cloud_index].name
        } else {
            "Unknown"
        };
        render_password_modal(
            "🔐 Set Cloud Password",
            cloud_name,
            &app.clouds_state.password_creation,
            area,
            buf,
        );
    }
}
