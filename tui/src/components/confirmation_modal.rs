use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Clear, Paragraph, Widget, Wrap},
};

use crate::{app::Confirmation, components::modal::centered_rect};

pub fn render_confirmation_modal(confirmation: &Confirmation, area: Rect, buf: &mut Buffer) {
    let modal_area = centered_rect(area, 66, 13);
    if modal_area.width == 0 || modal_area.height == 0 {
        return;
    }

    Clear.render(modal_area, buf);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(3),
            Constraint::Length(3),
        ])
        .split(modal_area);

    Paragraph::new(confirmation.title.as_str())
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Confirm action ")
                .border_style(Style::default().fg(Color::Red)),
        )
        .render(modal_area, buf);

    Paragraph::new(confirmation.description.as_str())
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true })
        .style(Style::default().fg(Color::White))
        .render(chunks[1], buf);

    Paragraph::new(format!(
        "Enter / y: {}    Esc / n: cancel",
        confirmation.confirm_label
    ))
    .alignment(Alignment::Center)
    .style(
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    )
    .render(chunks[2], buf);
}
