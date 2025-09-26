use color_eyre::Result;
use ratatui::{
    crossterm::event::{self, Event, KeyEventKind},
    DefaultTerminal,
};

mod config;
mod models;
mod tabs;
use models::App;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    // Don't initialize global tracing to avoid breaking TUI
    // tracing_subscriber::fmt::init();

    let terminal = ratatui::init();
    let app_result = App::new().run(terminal).await;
    ratatui::restore();
    app_result
}

impl App {
    async fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        while self.state == models::AppState::Running {
            terminal.draw(|frame| frame.render_widget(&self, frame.area()))?;

            // Check for timeouts before handling new events
            self.check_timeouts();

            self.handle_events()?;
        }
        Ok(())
    }

    fn handle_events(&mut self) -> std::io::Result<()> {
        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                self.handle_dynamic_key(key.code, key.modifiers);
            }
        }
        Ok(())
    }
}
