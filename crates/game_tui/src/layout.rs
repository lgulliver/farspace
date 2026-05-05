//! Layout helpers

use ratatui::layout::{Constraint, Direction, Layout, Rect};

/// Standard layout with header, main content, and footer
pub fn compose_layout(area: Rect) -> (Rect, Rect, Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Header
            Constraint::Min(10),   // Main content
            Constraint::Length(3), // Footer
        ])
        .split(area);

    (chunks[0], chunks[1], chunks[2])
}

/// Split area horizontally with given percentages
pub fn split_horizontal(area: Rect, left_percent: u16) -> (Rect, Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(left_percent),
            Constraint::Percentage(100 - left_percent),
        ])
        .split(area);

    (chunks[0], chunks[1])
}

/// Create a centered popup area
pub fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

/// Create a fixed-size centered rect
pub fn centered_fixed(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect::new(x, y, width.min(area.width), height.min(area.height))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compose_layout_creates_three_areas() {
        let area = Rect::new(0, 0, 80, 24);
        let (header, main, footer) = compose_layout(area);

        assert_eq!(header.height, 1);
        assert_eq!(footer.height, 3);
        assert!(main.height >= 10);
    }

    #[test]
    fn split_horizontal_respects_percentage() {
        let area = Rect::new(0, 0, 100, 24);
        let (left, right) = split_horizontal(area, 60);

        assert_eq!(left.width, 60);
        assert_eq!(right.width, 40);
    }

    #[test]
    fn centered_rect_is_centered() {
        let area = Rect::new(0, 0, 100, 100);
        let popup = centered_rect(50, 50, area);

        // Should be roughly centered (accounting for rounding)
        assert!(popup.x > 0);
        assert!(popup.y > 0);
        assert!(popup.x + popup.width < area.width);
        assert!(popup.y + popup.height < area.height);
    }

    #[test]
    fn centered_fixed_respects_bounds() {
        let area = Rect::new(10, 10, 80, 24);
        let popup = centered_fixed(40, 10, area);

        assert!(popup.x >= area.x);
        assert!(popup.y >= area.y);
        assert!(popup.x + popup.width <= area.x + area.width);
        assert!(popup.y + popup.height <= area.y + area.height);
    }
}
