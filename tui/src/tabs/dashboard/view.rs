use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{
        Block, Borders, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation, StatefulWidget,
        Widget, Wrap,
    },
};

use crate::{
    app::{App, SearchTarget},
    components::password_modal::render_password_modal,
    tabs::dashboard::state::DashboardPanel,
};

pub fn render_dashboard(app: &mut App, area: Rect, buf: &mut Buffer) {
    if area.width < 20 || area.height < 7 {
        Paragraph::new("Make the terminal a little larger to view the cloud dashboard.")
            .alignment(Alignment::Center)
            .render(area, buf);
        return;
    }

    let (services_area, overview_area, activity_area) = dashboard_layout(area);
    render_services(app, services_area, buf);
    render_overview(app, overview_area, buf);
    render_activity(app, activity_area, buf);

    if app.dashboard.password_creation.creating_password {
        let cloud_name = app
            .dashboard
            .clouds
            .get(app.dashboard.selected_cloud_index)
            .map(|cloud| cloud.name.as_str())
            .unwrap_or("Selected cloud");
        render_password_modal(
            " Set cloud password ",
            cloud_name,
            &app.dashboard.password_creation,
            area,
            buf,
        );
    }
}

fn dashboard_layout(area: Rect) -> (Rect, Rect, Rect) {
    if area.width >= 110 {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(28),
                Constraint::Length(44),
                Constraint::Min(30),
            ])
            .split(area);
        (chunks[0], chunks[1], chunks[2])
    } else if area.width >= 72 {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(8), Constraint::Min(7)])
            .split(area);
        let bottom = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(46), Constraint::Min(28)])
            .split(rows[1]);
        (rows[0], bottom[0], bottom[1])
    } else {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(32),
                Constraint::Percentage(34),
                Constraint::Min(5),
            ])
            .split(area);
        (chunks[0], chunks[1], chunks[2])
    }
}

fn border_style(focused: bool, color: Color) -> Style {
    if focused {
        Style::default().fg(color).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    }
}

fn render_services(app: &mut App, area: Rect, buf: &mut Buffer) {
    let target = SearchTarget::DashboardServices;
    let focused = app.dashboard.focused_panel == DashboardPanel::Services;
    let indices = app.filtered_indices(target);
    let search_suffix = app.search_title_suffix(target);
    let items: Vec<ListItem> = if app.dashboard.clouds.is_empty() {
        vec![ListItem::new(
            "No services yet\n\nAdd folders and create a cloud in Storage.",
        )]
    } else if indices.is_empty() {
        vec![ListItem::new("No services match this search.")]
    } else {
        indices
            .iter()
            .filter_map(|&index| app.dashboard.clouds.get(index))
            .map(|cloud| {
                let running = app.dashboard.is_cloud_running(&cloud.name);
                let icon = if running { "●" } else { "○" };
                let password = if cloud.password.is_some() {
                    "🔒"
                } else {
                    "⚠"
                };
                let color = if running {
                    Color::Green
                } else {
                    Color::DarkGray
                };
                ListItem::new(format!("{icon} {password} {}", cloud.name))
                    .style(Style::default().fg(color))
            })
            .collect()
    };
    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(
                    " Services · {}{search_suffix} ",
                    app.dashboard.clouds.len()
                ))
                .border_style(border_style(focused, Color::Cyan)),
        )
        .highlight_symbol("› ")
        .highlight_style(
            Style::default()
                .fg(Color::White)
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        );
    if app.is_searching(target) {
        let search_selection = app.search_selected_position(target);
        app.dashboard.clouds_list_state.select(search_selection);
    }
    StatefulWidget::render(list, area, buf, &mut app.dashboard.clouds_list_state);

    let selected = app.dashboard.clouds_list_state.selected().unwrap_or(0);
    let mut scroll_state = app
        .dashboard
        .clouds_scroll_state
        .content_length(indices.len())
        .position(selected);
    Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(Some("↑"))
        .end_symbol(Some("↓"))
        .render(area, buf, &mut scroll_state);
    app.dashboard.clouds_scroll_state = scroll_state;
}

fn render_overview(app: &App, area: Rect, buf: &mut Buffer) {
    let focused = app.dashboard.focused_panel == DashboardPanel::Overview;
    let info = match app
        .dashboard
        .clouds
        .get(app.dashboard.selected_cloud_index)
    {
        Some(cloud) => {
            let running = app.dashboard.is_cloud_running(&cloud.name);
            let status = if running {
                let port = app
                    .dashboard
                    .get_cloud_port(&cloud.name)
                    .map(|port| format!("Running · port {port}"))
                    .unwrap_or_else(|| "Running".to_string());
                format!("● {port}")
            } else if cloud.password.is_some() {
                "○ Ready to start".to_string()
            } else {
                "⚠ Password required".to_string()
            };
            let url = if running {
                app.orchestrator
                    .get_cloud_server_url(&cloud.name)
                    .unwrap_or_else(|| "URL is being prepared".to_string())
            } else {
                "Start the cloud to get a local URL".to_string()
            };
            let folders = cloud
                .cloud_folders
                .iter()
                .map(|folder| format!("  • {}", folder.name))
                .collect::<Vec<_>>()
                .join("\n");
            let start_stop = app.config.get_keys_for_action("Start/Stop Cloud").join(" / ");
            let open = app.config.get_keys_for_action("Open Cloud").join(" / ");
            let password = app
                .config
                .get_keys_for_action("Set Cloud Password")
                .join(" / ");
            format!(
                "{}\n\n{status}\n\n{url}\n\nFolders ({})\n{}\n\n{start_stop}  start / stop\n{open}  open in browser\n{password}  set password",
                cloud.name,
                cloud.cloud_folders.len(),
                folders,
            )
        }
        None => "Create a cloud in Storage to host files locally.\n\nClouds with a password can be started here.".to_string(),
    };
    let error = app.dashboard.cloud_start_error.as_deref();
    let text = match error {
        Some(error) => format!("{info}\n\n{error}"),
        None => info,
    };
    Paragraph::new(text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Overview ")
                .border_style(border_style(focused, Color::Cyan)),
        )
        .wrap(Wrap { trim: true })
        .render(area, buf);
}

fn render_activity(app: &mut App, area: Rect, buf: &mut Buffer) {
    let target = SearchTarget::DashboardActivity;
    let focused = app.dashboard.focused_panel == DashboardPanel::Activity;
    let indices = app.filtered_indices(target);
    let search_suffix = app.search_title_suffix(target);
    let follows_latest = app
        .dashboard
        .activity_list_state
        .selected()
        .is_some_and(|selected| selected + 1 == app.activity.len());
    let title = if follows_latest {
        format!(" Activity · live{search_suffix} ")
    } else {
        format!(" Activity{search_suffix} ")
    };
    let items: Vec<ListItem> = if app.activity.is_empty() {
        vec![
            ListItem::new("Waiting for activity from the selected cloud.")
                .style(Style::default().fg(Color::DarkGray)),
        ]
    } else if indices.is_empty() {
        vec![ListItem::new("No activity matches this search.")]
    } else {
        indices
            .iter()
            .filter_map(|&index| app.activity.get(index))
            .map(|log| {
                let color = match log.level {
                    cloudhost_server::debug_stream::LogLevel::Error => Color::Red,
                    cloudhost_server::debug_stream::LogLevel::Warning => Color::Yellow,
                    cloudhost_server::debug_stream::LogLevel::Info => Color::Green,
                    cloudhost_server::debug_stream::LogLevel::Debug => Color::Blue,
                };
                ListItem::new(format!(
                    "{}  {}",
                    log.timestamp.format("%H:%M:%S"),
                    log.message
                ))
                .style(Style::default().fg(color))
            })
            .collect()
    };
    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(border_style(focused, Color::Cyan)),
        )
        .highlight_symbol("› ")
        .highlight_style(
            Style::default()
                .fg(Color::White)
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        );
    if app.is_searching(target) {
        let search_selection = app.search_selected_position(target);
        app.dashboard.activity_list_state.select(search_selection);
    }
    StatefulWidget::render(list, area, buf, &mut app.dashboard.activity_list_state);

    let selected = app.dashboard.activity_list_state.selected().unwrap_or(0);
    let mut scroll_state = app
        .dashboard
        .activity_scroll_state
        .content_length(indices.len())
        .position(selected);
    Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(Some("↑"))
        .end_symbol(Some("↓"))
        .render(area, buf, &mut scroll_state);
    app.dashboard.activity_scroll_state = scroll_state;
}
