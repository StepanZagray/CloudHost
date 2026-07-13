use ratatui::layout::Rect;

/// Returns a modal rectangle that always fits inside the current terminal.
///
/// Keeping a one-cell gutter where possible means modal borders remain visible
/// even in a small terminal or split pane.
pub fn centered_rect(area: Rect, desired_width: u16, desired_height: u16) -> Rect {
    if area.width == 0 || area.height == 0 {
        return area;
    }

    let width = desired_width.min(area.width.saturating_sub(2).max(1));
    let height = desired_height.min(area.height.saturating_sub(2).max(1));

    Rect::new(
        area.x + area.width.saturating_sub(width) / 2,
        area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn centers_and_keeps_a_gutter_when_space_allows() {
        let area = Rect::new(10, 5, 100, 40);
        let modal = centered_rect(area, 60, 20);

        assert_eq!(modal, Rect::new(30, 15, 60, 20));
    }

    #[test]
    fn never_extends_beyond_a_small_terminal() {
        let area = Rect::new(3, 4, 12, 7);
        let modal = centered_rect(area, 60, 20);

        assert!(modal.x >= area.x);
        assert!(modal.y >= area.y);
        assert!(modal.x + modal.width <= area.x + area.width);
        assert!(modal.y + modal.height <= area.y + area.height);
        assert_eq!(modal.width, 10);
        assert_eq!(modal.height, 5);
    }
}
