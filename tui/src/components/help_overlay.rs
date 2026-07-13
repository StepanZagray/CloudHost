use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Clear, Paragraph, Widget, Wrap},
};

use crate::{app::App, components::modal::centered_rect};

pub fn render_help_overlay(app: &App, area: Rect, buf: &mut Buffer) {
    let modal_area = centered_rect(area, 78, area.height.saturating_sub(2).min(24));
    if modal_area.width == 0 || modal_area.height == 0 {
        return;
    }

    let current_tab = match app.selected_tab {
        crate::tabs::SelectedTab::Dashboard => "dashboard",
        crate::tabs::SelectedTab::Storage => "storage",
        crate::tabs::SelectedTab::Settings => "settings",
    };

    let mut actions: Vec<_> = app
        .config
        .actions
        .iter()
        .filter(|(_, action)| action.applies_to(current_tab))
        .collect();
    actions.sort_by_key(|(name, _)| *name);

    let max_action_lines = usize::from(modal_area.height.saturating_sub(6));
    let hidden_actions = actions.len().saturating_sub(max_action_lines);
    let visible_action_lines = if hidden_actions == 0 {
        actions
            .into_iter()
            .map(|(name, action)| {
                let keys = action.keys.join(" / ");
                format!("{keys:<22}  {name}")
            })
            .collect()
    } else {
        let visible_count = max_action_lines.saturating_sub(1);
        let mut visible: Vec<String> = actions
            .into_iter()
            .take(visible_count)
            .map(|(name, action)| {
                let keys = action.keys.join(" / ");
                format!("{keys:<22}  {name}")
            })
            .collect();
        visible.push(format!("… {hidden_actions} more commands available"));
        visible
    };

    let mut lines = vec!["Keys                    Action".to_string()];
    lines.push("────────────────────  ─────────────────────────────".to_string());
    lines.extend(visible_action_lines);
    lines.push(String::new());
    lines.push("? or Esc closes this help screen.".to_string());

    Clear.render(modal_area, buf);
    Paragraph::new(lines.join("\n"))
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: false })
        .style(Style::default().fg(Color::White))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" Keyboard shortcuts · {} ", app.selected_tab))
                .title_alignment(Alignment::Center)
                .border_style(
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
        )
        .render(modal_area, buf);
}
