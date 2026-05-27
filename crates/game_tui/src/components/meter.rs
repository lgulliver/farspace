//! Shared meter/bar helper for status displays.

use crate::theme::{ColorMode, Theme};
use ratatui::text::{Line, Span};

/// Offset used to round to nearest integer in `(x + 50) / 100` percentage math.
const PERCENT_ROUNDING_OFFSET: usize = 50;

fn truncate_to_width(text: &str, width: usize) -> String {
    text.chars().take(width).collect()
}

pub fn meter_line(label: impl Into<String>, percent: u8, total_width: u16) -> Line<'static> {
    let width = usize::from(total_width);
    if width == 0 {
        return Line::from("");
    }

    let percent = percent.min(100);
    let percent_text = format!("{percent:>3}%");
    let suffix_width = percent_text.chars().count() + 1;
    if width <= suffix_width {
        return Line::from(Span::styled(
            truncate_to_width(&percent_text, width),
            Theme::text_secondary_style(),
        ));
    }

    let mut label_text = label.into();
    let max_label = width.saturating_sub(suffix_width + 2);
    label_text = truncate_to_width(&label_text, max_label);

    let bar_width = width.saturating_sub(label_text.chars().count() + suffix_width + 1);
    if bar_width == 0 {
        return Line::from(vec![
            Span::styled(label_text, Theme::text_primary_style()),
            Span::raw(" "),
            Span::styled(percent_text, Theme::text_secondary_style()),
        ]);
    }

    let filled = ((bar_width * usize::from(percent)) + PERCENT_ROUNDING_OFFSET) / 100;
    let (filled_glyph, empty_glyph) = if Theme::color_mode() == ColorMode::Mono {
        ('#', '-')
    } else {
        ('█', '░')
    };

    Line::from(vec![
        Span::styled(label_text, Theme::text_primary_style()),
        Span::raw(" "),
        Span::styled(
            filled_glyph.to_string().repeat(filled),
            Theme::accent_style().bg(Theme::panel_bg()),
        ),
        Span::styled(
            empty_glyph
                .to_string()
                .repeat(bar_width.saturating_sub(filled)),
            Theme::muted_style().bg(Theme::panel_bg()),
        ),
        Span::raw(" "),
        Span::styled(percent_text, Theme::text_secondary_style()),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn meter_renders_without_panic_at_small_width() {
        let _ = meter_line("Research", 70, 8);
        let _ = meter_line("Research", 70, 2);
        let _ = meter_line("Research", 70, 0);
    }
}
