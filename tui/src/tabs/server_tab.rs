use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Widget},
};

use crate::models::{App, Profile};

pub fn render_server_tab(app: &App, area: Rect, buf: &mut Buffer) {
    use ratatui::layout::{Constraint, Direction, Layout};
    use ratatui::widgets::{List, ListItem, ListState};

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    // Left side: Profile list
    let profile_items: Vec<ListItem> = app
        .profiles
        .iter()
        .enumerate()
        .map(|(i, profile)| {
            let style = if i == app.selected_profile_index {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            };
            ListItem::new(profile.name.as_str()).style(style)
        })
        .collect();

    let mut list_state = ListState::default();
    list_state.select(Some(app.selected_profile_index));

    let profile_list = List::new(profile_items)
        .block(
            Block::default()
                .borders(ratatui::widgets::Borders::ALL)
                .title("Profiles")
                .title_alignment(Alignment::Left),
        )
        .highlight_style(Style::default().fg(Color::Yellow));

    profile_list.render(chunks[0], buf);

    // Right side: Server controls and info
    let server_status = if app.server_running {
        "🟢 Running"
    } else {
        "🔴 Stopped"
    };

    let server_info = if let Some(profile) = app.profiles.get(app.selected_profile_index) {
        format!(
            "Selected Profile: {}\nPath: {}\nStatus: {}",
            profile.name,
            profile.folder_path.display(),
            server_status
        )
    } else {
        "No profiles available".to_string()
    };

    let server_block = Paragraph::new(server_info)
        .block(
            Block::default()
                .borders(ratatui::widgets::Borders::ALL)
                .title("Server Info")
                .title_alignment(Alignment::Left),
        )
        .alignment(ratatui::layout::Alignment::Left);

    server_block.render(chunks[1], buf);

    // Show popup if creating profile
    if app.creating_profile {
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
        .split(area);

    let popup_inner = Layout::default()
        .direction(ratatui::layout::Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(50),
            Constraint::Percentage(25),
        ])
        .split(popup_area[1])[1];

    // Clear the background
    Clear.render(popup_inner, buf);

    let popup_content = if let Some(error) = &app.profile_creation_error {
        format!(
            "Create New Profile\n\nName: {}\n\nError: {}",
            app.new_profile_name, error
        )
    } else {
        format!("Create New Profile\n\nName: {}", app.new_profile_name)
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
