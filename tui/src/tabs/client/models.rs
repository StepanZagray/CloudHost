use crate::tabs::focus::TabFocus;
use ratatui::crossterm::event::KeyCode;

#[derive(Default)]
pub struct ClientState {
    // Placeholder for client-specific state
}

impl TabFocus for ClientState {
    fn get_focused_element(&self) -> String {
        "ClientMain".to_string()
    }

    fn cycle_focus_forward(&mut self) {
        // Client tab has only one focusable element
    }

    fn cycle_focus_backward(&mut self) {
        // Client tab has only one focusable element
    }

    fn handle_navigation(&mut self, _key: KeyCode) -> bool {
        // Client navigation not implemented yet
        false
    }

    fn has_focusable_elements(&self) -> bool {
        true
    }

    fn focusable_elements_count(&self) -> usize {
        1 // Just the main client area
    }
}
