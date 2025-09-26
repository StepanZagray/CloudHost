use ratatui::crossterm::event::KeyCode;

/// Trait for tab focus management
pub trait TabFocus {
    /// Get the currently focused element within this tab
    fn get_focused_element(&self) -> String;

    /// Cycle focus forward within this tab
    fn cycle_focus_forward(&mut self);

    /// Cycle focus backward within this tab
    fn cycle_focus_backward(&mut self);

    /// Handle navigation within the focused element
    fn handle_navigation(&mut self, key: KeyCode) -> bool;

    /// Check if this tab has focusable elements
    fn has_focusable_elements(&self) -> bool;

    /// Get the number of focusable elements in this tab
    fn focusable_elements_count(&self) -> usize;
}

/// Default implementation for tabs that don't need focus management
impl TabFocus for () {
    fn get_focused_element(&self) -> String {
        "None".to_string()
    }

    fn cycle_focus_forward(&mut self) {}

    fn cycle_focus_backward(&mut self) {}

    fn handle_navigation(&mut self, _key: KeyCode) -> bool {
        false
    }

    fn has_focusable_elements(&self) -> bool {
        false
    }

    fn focusable_elements_count(&self) -> usize {
        0
    }
}
