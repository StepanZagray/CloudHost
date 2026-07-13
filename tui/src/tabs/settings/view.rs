use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Scrollbar, ScrollbarOrientation, StatefulWidget},
};

use crate::{
    app::{App, SearchTarget},
    tabs::settings::state::SettingsAction,
};

pub fn render_settings(app: &mut App, area: Rect, buf: &mut Buffer) {
    let target = SearchTarget::Settings;
    let indices = app.filtered_indices(target);
    let search_suffix = app.search_title_suffix(target);
    let items: Vec<ListItem> = if indices.is_empty() {
        vec![ListItem::new("No settings match this search.")]
    } else {
        indices
            .iter()
            .map(|&index| setting_item(app, SettingsAction::ALL[index]))
            .collect()
    };

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" Settings{search_suffix} "))
                .title_alignment(Alignment::Center)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .highlight_style(
            Style::default()
                .fg(Color::White)
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("› ");

    if app.is_searching(target) {
        let search_selection = app.search_selected_position(target);
        app.settings.list_state.select(search_selection);
    }

    StatefulWidget::render(list, area, buf, &mut app.settings.list_state);

    let selected = app.settings.list_state.selected().unwrap_or(0);
    let mut scroll_state = app
        .settings
        .scroll_state
        .content_length(indices.len())
        .position(selected);
    Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .begin_symbol(Some("↑"))
        .end_symbol(Some("↓"))
        .render(area, buf, &mut scroll_state);
    app.settings.scroll_state = scroll_state;
}

fn setting_item(app: &App, action: SettingsAction) -> ListItem<'static> {
    let shortcut = action
        .config_action()
        .map(|name| app.config.get_keys_for_action(name).join(" / "))
        .unwrap_or_else(|| app.config.get_keys_for_action("Execute Action").join(" / "));
    let tone = if action.is_destructive() {
        Color::Red
    } else {
        Color::Cyan
    };

    ListItem::new(vec![
        Line::from(vec![
            Span::styled(
                format!(" {} ", shortcut),
                Style::default().fg(Color::Black).bg(tone),
            ),
            Span::raw(format!("  {}", action.title())),
        ]),
        Line::from(Span::styled(
            format!("      {}", action.description()),
            Style::default().fg(Color::DarkGray),
        )),
        Line::raw(""),
    ])
}
