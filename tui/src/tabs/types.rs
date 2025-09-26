use ratatui::{
    layout::Rect,
    prelude::Stylize,
    style::palette::tailwind,
    symbols,
    widgets::{Block, Padding, Widget},
};
use strum::{Display, EnumIter, FromRepr, IntoEnumIterator};

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Display, EnumIter, FromRepr)]
pub enum SelectedTab {
    #[default]
    #[strum(to_string = "Server")]
    Server,
    #[strum(to_string = "Client")]
    Client,
    #[strum(to_string = "Settings")]
    Settings,
}

impl SelectedTab {
    /// Get the previous tab with wrapping. If at first tab, goes to last tab.
    pub fn previous(self) -> Self {
        let current_index: usize = self as usize;
        let total_tabs = Self::iter().count();
        let previous_index = if current_index == 0 {
            total_tabs - 1
        } else {
            current_index - 1
        };
        Self::from_repr(previous_index).unwrap_or(self)
    }

    /// Get the next tab with wrapping. If at last tab, goes to first tab.
    pub fn next(self) -> Self {
        let current_index: usize = self as usize;
        let total_tabs = Self::iter().count();
        let next_index = (current_index + 1) % total_tabs;
        Self::from_repr(next_index).unwrap_or(self)
    }

    /// A block surrounding the tab's content
    pub fn block(self) -> Block<'static> {
        Block::bordered()
            .border_set(symbols::border::PROPORTIONAL_TALL)
            .padding(Padding::horizontal(1))
            .border_style(self.palette().c700)
    }

    pub const fn palette(self) -> tailwind::Palette {
        match self {
            Self::Server => tailwind::BLUE,
            Self::Client => tailwind::GREEN,
            Self::Settings => tailwind::INDIGO,
        }
    }

    /// Return tab's name as a styled `Line`
    pub fn title(self) -> ratatui::text::Line<'static> {
        use ratatui::style::palette::tailwind;
        format!("  {self}  ")
            .fg(tailwind::SLATE.c200)
            .bg(self.palette().c900)
            .into()
    }
}

impl SelectedTab {
    pub fn render_tab(self, app: &crate::models::App, area: Rect, buf: &mut ratatui::buffer::Buffer) {
        match self {
            Self::Server => crate::tabs::server::ui::render_server_tab(app, area, buf),
            Self::Client => crate::tabs::client::ui::render_client_tab(app, area, buf),
            Self::Settings => crate::tabs::settings::ui::render_settings_tab(app, area, buf),
        }
    }
}

impl SelectedTab {
    pub fn render_server_placeholder(self, area: Rect, buf: &mut ratatui::buffer::Buffer) {
        use ratatui::widgets::Paragraph;
        Paragraph::new("Server functionality - managed by App")
            .block(self.block())
            .render(area, buf);
    }
}
