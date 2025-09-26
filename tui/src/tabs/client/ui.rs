use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget, style::{Color, Style}, widgets::{Block, Borders}};

use crate::models::App;

pub fn render_client_tab(_app: &App, area: Rect, buf: &mut Buffer) {
    use ratatui::widgets::Paragraph;
    
    // Add focus indicator
    let title = "Client (FOCUSED)"; // Client tab always has focus when active
    
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(Style::default().fg(Color::Cyan).add_modifier(ratatui::style::Modifier::BOLD));
    
    Paragraph::new("Client functionality coming soon!")
        .block(block)
        .render(area, buf);
}
