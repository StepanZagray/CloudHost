use color_eyre::Result;
use ratatui::{
    crossterm::event::{self, Event, KeyEventKind},
    DefaultTerminal,
};

mod config;
mod error;
mod models;
mod tabs;
use cloudhost_shared::debug_stream::init_debug_stream;
use error::TuiResult;
use models::App;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    // Don't initialize global tracing to avoid breaking TUI
    // tracing_subscriber::fmt::init();

    // Initialize debug stream for server-to-TUI communication
    init_debug_stream(1000); // Keep last 1000 messages

    let terminal = ratatui::init();
    let mut app = App::new();

    // Subscribe to debug stream immediately after app creation
    app.start_debug_stream_subscription().await;

    // Log TUI config loading
    let config = crate::config::Config::load();
    if let Err(e) = config {
        if let Some(debug_stream) = cloudhost_shared::debug_stream::get_debug_stream() {
            debug_stream
                .warn(
                    "TUI",
                    &format!(
                        "Could not load config.toml ({}), using default keybindings",
                        e
                    ),
                )
                .await;
        }
    }

    // Test debug stream
    if let Some(debug_stream) = cloudhost_shared::debug_stream::get_debug_stream() {
        debug_stream
            .info("TUI", "Debug stream initialized successfully")
            .await;
        debug_stream.info("TUI", "This is a test message").await;
        debug_stream.warn("TUI", "This is a warning message").await;
        debug_stream.error("TUI", "This is an error message").await;
    }

    let app_result = app.run(terminal).await;
    ratatui::restore();
    app_result
}

impl App {
    async fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        // Start debug stream subscription
        while self.state == models::AppState::Running {
            terminal.draw(|frame| frame.render_widget(&mut self, frame.area()))?;

            // Check for timeouts before handling new events
            self.check_timeouts().await;

            // Update server logs periodically
            self.update_server_logs().await;

            self.handle_events().await?;
        }
        Ok(())
    }

    async fn handle_events(&mut self) -> TuiResult<()> {
        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                self.handle_dynamic_key(key.code, key.modifiers).await;
            }
        }
        Ok(())
    }
}
