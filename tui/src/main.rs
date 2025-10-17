use clap::Parser;
use color_eyre::Result;
use ratatui::{
    crossterm::event::{self, Event, KeyEventKind},
    DefaultTerminal,
};

mod components;
mod config;
mod error;
mod models;
mod tabs;
mod utils;
use error::TuiResult;
use models::App;

/// CloudHost TUI - Personal Cloud Storage Server
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Force development mode (configs in project root)
    #[arg(long)]
    dev: bool,

    /// Force production mode (configs in appdata)
    #[arg(long)]
    prod: bool,

    /// Enable debug logging
    #[arg(short = 'v', long)]
    debug: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    // Parse command line arguments
    let args = Args::parse();

    // Set environment variables based on command line arguments
    if args.dev {
        std::env::set_var("CLOUDHOST_DEV", "1");
    } else if args.prod {
        std::env::remove_var("CLOUDHOST_DEV");
        std::env::remove_var("CARGO");
        std::env::remove_var("DEBUG");
        std::env::remove_var("RUST_LOG");
    }

    if args.debug {
        std::env::set_var("RUST_LOG", "debug");
        std::env::set_var("DEBUG", "1");
    }

    // Don't initialize global tracing to avoid breaking TUI
    // tracing_subscriber::fmt::init();

    // Initialize debug stream for server-to-TUI communication

    let terminal = ratatui::init();
    let app = App::new();

    // Load TUI config
    let _config = crate::config::Config::load();

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
            self.update_cloud_logs().await;

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
