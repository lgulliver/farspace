//! Color interpolation helpers.

use ratatui::style::Color;

fn rgb_components(color: Color) -> Option<(u8, u8, u8)> {
    match color {
        Color::Rgb(r, g, b) => Some((r, g, b)),
        _ => None,
    }
}

/// Linear interpolation between two RGB colors.
pub fn lerp_rgb(a: Color, b: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    match (rgb_components(a), rgb_components(b)) {
        (Some((ar, ag, ab)), Some((br, bg, bb))) => Color::Rgb(
            (ar as f32 + (br as f32 - ar as f32) * t).round() as u8,
            (ag as f32 + (bg as f32 - ag as f32) * t).round() as u8,
            (ab as f32 + (bb as f32 - ab as f32) * t).round() as u8,
        ),
        _ if t < 0.5 => a,
        _ => b,
    }
}

/// Build an RGB gradient ramp. Non-RGB colors degrade to endpoint selection.
pub fn gradient(a: Color, b: Color, steps: usize) -> Vec<Color> {
    match steps {
        0 => Vec::new(),
        1 => vec![a],
        _ => (0..steps)
            .map(|index| {
                let t = index as f32 / (steps.saturating_sub(1)) as f32;
                lerp_rgb(a, b, t)
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lerp_rgb_returns_midpoint() {
        assert_eq!(
            lerp_rgb(Color::Rgb(0, 0, 0), Color::Rgb(100, 150, 200), 0.5),
            Color::Rgb(50, 75, 100)
        );
    }

    #[test]
    fn gradient_returns_expected_endpoints() {
        let colors = gradient(Color::Rgb(10, 20, 30), Color::Rgb(110, 120, 130), 3);
        assert_eq!(colors[0], Color::Rgb(10, 20, 30));
        assert_eq!(colors[1], Color::Rgb(60, 70, 80));
        assert_eq!(colors[2], Color::Rgb(110, 120, 130));
    }

    #[test]
    fn gradient_falls_back_for_non_rgb() {
        let colors = gradient(Color::Blue, Color::White, 4);
        assert_eq!(
            colors,
            vec![Color::Blue, Color::Blue, Color::White, Color::White]
        );
    }
}
