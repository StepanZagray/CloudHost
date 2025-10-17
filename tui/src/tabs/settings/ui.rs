use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, List, ListItem, Scrollbar, ScrollbarOrientation, StatefulWidget},
};

use crate::models::App;

pub fn render_settings_tab(app: &mut App, area: Rect, buf: &mut Buffer) {
    // Create a single column layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0)])
        .split(area);

    // Create config file information items
    let tui_config_path = cloudhost_server::config_paths::get_tui_config_path();
    let clouds_config_path = cloudhost_server::config_paths::get_clouds_config_path();

    let items = vec![
        ListItem::new("📄 TUI Config File"),
        ListItem::new(format!("   {}", tui_config_path.display())),
        ListItem::new(""),
        ListItem::new("☁️  Clouds Config File"),
        ListItem::new(format!("   {}", clouds_config_path.display())),
        ListItem::new(""),
        ListItem::new("🔄 Reset TUI Config to Default"),
        ListItem::new("   ⚠️  This will delete your current keybinds and restore defaults"),
        ListItem::new("   ℹ️  Restart the app to see the changes"),
    ];

    // Create the list
    let list = List::new(items.clone())
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("⚙️  Settings - Config Files")
                .title_alignment(Alignment::Center)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .highlight_style(Style::default().fg(Color::Yellow))
        .highlight_symbol(">> ");

    // Render the list with state
    StatefulWidget::render(list, chunks[0], buf, &mut app.settings_state.list_state);

    // Render scrollbar
    let mut scroll_state = app.settings_state.scroll_state;
    scroll_state = scroll_state.content_length(items.len());
    if let Some(selected) = app.settings_state.list_state.selected() {
        scroll_state = scroll_state.position(selected);
    }

    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(Some("↑"))
        .end_symbol(Some("↓"));
    scrollbar.render(chunks[0], buf, &mut scroll_state);

    // Update the persistent scroll state
    app.settings_state.scroll_state = scroll_state;
}
