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
    let left_percent = left_percent.min(100);
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
    let x = area
        .x
        .saturating_add((area.width.saturating_sub(width)) / 2);
    let y = area
        .y
        .saturating_add((area.height.saturating_sub(height)) / 2);
    Rect::new(x, y, width.min(area.width), height.min(area.height))
}

/// Center a bounded card-like area inside a region.
pub fn main_card_area(area: Rect, max_width: u16, max_height: u16) -> Rect {
    centered_fixed(max_width.min(area.width), max_height.min(area.height), area)
}

/// Split area into sidebar (left) and main content (right).
pub fn split_sidebar_main(area: Rect, sidebar_width: u16) -> (Rect, Rect) {
    let sidebar_width = sidebar_width.min(area.width);
    let sidebar = Rect::new(area.x, area.y, sidebar_width, area.height);
    let main = Rect::new(
        area.x.saturating_add(sidebar_width),
        area.y,
        area.width.saturating_sub(sidebar_width),
        area.height,
    );
    (sidebar, main)
}

/// Split area into main content and detail panel.
pub fn split_main_detail(area: Rect) -> (Rect, Rect) {
    if area.width < 40 {
        return (
            area,
            Rect::new(area.x.saturating_add(area.width), area.y, 0, area.height),
        );
    }
    let detail_width = ((area.width as u32 * 35) / 100) as u16;
    let detail_width = detail_width.max(24).min(area.width.saturating_sub(20));
    let main_width = area.width.saturating_sub(detail_width);
    let main = Rect::new(area.x, area.y, main_width, area.height);
    let detail = Rect::new(
        area.x.saturating_add(main_width),
        area.y,
        detail_width,
        area.height,
    );
    (main, detail)
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
    fn split_horizontal_clamps_over_100() {
        let area = Rect::new(0, 0, 100, 24);
        // Should not panic or wrap; 100% left means right=0
        let (left, right) = split_horizontal(area, 150);
        assert_eq!(left.width + right.width, area.width);
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

    #[test]
    fn main_card_area_stays_within_bounds() {
        let area = Rect::new(5, 4, 70, 20);
        let card = main_card_area(area, 80, 30);
        assert!(card.x >= area.x);
        assert!(card.y >= area.y);
        assert!(card.x + card.width <= area.x + area.width);
        assert!(card.y + card.height <= area.y + area.height);
    }

    #[test]
    fn split_sidebar_main_stays_within_bounds() {
        let area = Rect::new(0, 0, 80, 24);
        let (sidebar, main) = split_sidebar_main(area, 28);
        assert_eq!(sidebar.width + main.width, area.width);
        assert_eq!(sidebar.height, area.height);
        assert_eq!(main.height, area.height);
    }

    #[test]
    fn split_main_detail_stays_within_bounds() {
        let area = Rect::new(0, 0, 96, 30);
        let (main, detail) = split_main_detail(area);
        assert_eq!(main.width + detail.width, area.width);
        assert_eq!(main.height, area.height);
        assert_eq!(detail.height, area.height);
    }

    #[test]
    fn split_helpers_handle_large_coordinates_without_overflow() {
        let area = Rect::new(u16::MAX - 3, 0, 3, 1);
        let (_sidebar, main) = split_sidebar_main(area, 2);
        let (_main, detail) = split_main_detail(area);
        assert!(main.x >= area.x);
        assert!(detail.x >= area.x);
    }
}
