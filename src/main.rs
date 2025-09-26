use color_eyre::Result;
use ratatui::{
    DefaultTerminal,
    crossterm::event::{self, Event, KeyCode, KeyEventKind},
};

mod config;
mod models;
use models::{App, InputState};

fn main() -> Result<()> {
    color_eyre::install()?;
    let terminal = ratatui::init();
    let app_result = App::new().run(terminal);
    ratatui::restore();
    app_result
}

impl App {
    fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        while self.state == models::AppState::Running {
            terminal.draw(|frame| frame.render_widget(&self, frame.area()))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn handle_events(&mut self) -> std::io::Result<()> {
        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                self.handle_key(key.code);
            }
        }
        Ok(())
    }

    fn handle_key(&mut self, key: KeyCode) {
        match (&mut self.input_state, key) {
            // Quit
            (InputState::Normal, KeyCode::Char('q')) | (InputState::Normal, KeyCode::Esc) => {
                self.quit();
            }

            // Number prefix handling
            (InputState::Normal, KeyCode::Char(d @ '0'..='9')) => {
                self.pending_number = Some(d.to_digit(10).unwrap() as usize);
                self.input_state = InputState::NumberPrefix;
            }
            (InputState::NumberPrefix, KeyCode::Char(d @ '0'..='9')) => {
                if let Some(current) = self.pending_number {
                    self.pending_number = Some(current * 10 + d.to_digit(10).unwrap() as usize);
                }
            }

            // First 'g' key
            (InputState::Normal, KeyCode::Char('g'))
            | (InputState::NumberPrefix, KeyCode::Char('g')) => {
                self.input_state = InputState::AfterG;
            }

            // "gt" sequence - next tab
            (InputState::AfterG, KeyCode::Char('t')) => {
                if let Some(n) = self.pending_number.take() {
                    self.goto_tab(n.saturating_sub(1));
                } else {
                    self.next_tab();
                }
                self.input_state = InputState::Normal;
            }

            // "gT" sequence - previous tab
            (InputState::AfterG, KeyCode::Char('T')) => {
                self.previous_tab();
                self.input_state = InputState::Normal;
            }

            // Arrow keys and other navigation
            (InputState::Normal, KeyCode::Right) => self.next_tab(),
            (InputState::Normal, KeyCode::Left) => self.previous_tab(),

            // Check configurable keybindings
            (InputState::Normal, KeyCode::Char(c)) => {
                let key_str = c.to_string();
                if let Some(action) = self.config.get_action(&key_str) {
                    match action.as_str() {
                        "Next Tab" => self.next_tab(),
                        "Previous Tab" => self.previous_tab(),
                        "Quit" => self.quit(),
                        _ => {}
                    }
                }
            }

            // Reset state for unrecognized keys
            _ => {
                self.input_state = InputState::Normal;
                self.pending_number = None;
            }
        }
    }
}
