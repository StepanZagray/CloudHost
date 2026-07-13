use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    prelude::Stylize,
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, List, ListItem, Paragraph, Tabs, Widget},
};
use strum::IntoEnumIterator;

use crate::{
    app::{App, NoticeKind},
    tabs::SelectedTab,
};

impl Widget for &mut App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let main_constraints = if self.debug_mode {
            vec![
                Constraint::Length(1),
                Constraint::Min(0),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(12),
            ]
        } else {
            vec![
                Constraint::Length(1),
                Constraint::Min(0),
                Constraint::Length(1),
                Constraint::Length(1),
            ]
        };
        let areas = Layout::vertical(main_constraints).split(area);
        let [title_area, tabs_area] =
            Layout::horizontal([Constraint::Length(28), Constraint::Min(0)]).areas(areas[0]);

        render_title(title_area, buf);
        self.render_tabs(tabs_area, buf);
        self.selected_tab.render_tab(self, areas[1], buf);
        self.render_status_bar(areas[2], buf);
        self.render_footer(areas[3], buf);
        if self.debug_mode {
            self.render_debug_panel(areas[4], buf);
        }

        if let Some(confirmation) = self.pending_confirmation.as_ref() {
            crate::components::confirmation_modal::render_confirmation_modal(
                confirmation,
                area,
                buf,
            );
        } else if self.help_open {
            crate::components::help_overlay::render_help_overlay(self, area, buf);
        }
    }
}

impl App {
    fn render_tabs(&self, area: Rect, buf: &mut Buffer) {
        let titles = SelectedTab::iter().map(SelectedTab::title);
        let highlight_style = (Color::default(), self.selected_tab.palette().c700);
        Tabs::new(titles)
            .highlight_style(highlight_style)
            .select(self.selected_tab as usize)
            .padding("", "")
            .divider(" ")
            .render(area, buf);
    }

    fn render_debug_panel(&self, area: Rect, buf: &mut Buffer) {
        let [messages_area, header_area] =
            Layout::vertical([Constraint::Min(0), Constraint::Length(3)]).areas(area);
        Paragraph::new(format!("TUI Debug ({} messages)", self.debug_info.len()))
            .block(Block::default().borders(Borders::ALL).title("Debug Panel"))
            .style(Style::default().fg(Color::Cyan))
            .render(header_area, buf);

        let items = self.debug_info.iter().map(|message| {
            ListItem::new(format!("[TUI] {message}")).style(Style::default().fg(Color::Magenta))
        });
        List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Logs"))
            .render(messages_area, buf);
    }

    fn render_status_bar(&self, area: Rect, buf: &mut Buffer) {
        let (icon, color, message) = if let Some(notice) = &self.notice {
            let (icon, color) = match notice.kind {
                NoticeKind::Info => ("●", Color::Cyan),
                NoticeKind::Success => ("✓", Color::Green),
                NoticeKind::Warning => ("!", Color::Yellow),
                NoticeKind::Error => ("×", Color::Red),
            };
            (icon, color, notice.message.clone())
        } else {
            let clouds = self.dashboard.clouds.len();
            let running = self.dashboard.running_clouds.len();
            let folders = self.storage.folders.len();
            (
                "●",
                Color::DarkGray,
                format!("{clouds} clouds · {folders} folders · {running} running"),
            )
        };

        Paragraph::new(format!(" {icon}  {message}"))
            .style(Style::default().fg(color).add_modifier(Modifier::BOLD))
            .render(area, buf);
    }

    fn render_footer(&self, area: Rect, buf: &mut Buffer) {
        let keys = |action: &str| self.config.get_keys_for_action(action).join("/");
        let footer = match self.selected_tab {
            SelectedTab::Dashboard => format!(
                "{} navigate · {} focus · / search · {} start/stop · {} open · ? help · {} quit",
                keys("Navigate Up"),
                keys("Cycle Focus Forward"),
                keys("Start/Stop Cloud"),
                keys("Open Cloud"),
                keys("Quit"),
            ),
            SelectedTab::Storage => format!(
                "{} navigate · {} focus · / search · {} add folder · {} new cloud · {} remove · ? help",
                keys("Navigate Up"),
                keys("Cycle Focus Forward"),
                keys("Add Folder"),
                keys("Create Cloud"),
                keys("Remove Folder / Cloud"),
            ),
            SelectedTab::Settings => format!(
                "{} navigate · / search · Enter run · ? help · {} quit",
                keys("Navigate Up"),
                keys("Quit"),
            ),
        };
        Line::raw(footer).centered().render(area, buf);
    }
}

fn render_title(area: Rect, buf: &mut Buffer) {
    let title = if cloudhost_server::config_paths::is_dev_mode() {
        "CloudHost (dev)"
    } else {
        "CloudHost"
    };
    let title = if area.width >= 36 {
        format!("{title} · personal file hosting")
    } else {
        title.to_string()
    };
    format!("  {title}")
        .fg(Color::Cyan)
        .bold()
        .render(area, buf);
}
