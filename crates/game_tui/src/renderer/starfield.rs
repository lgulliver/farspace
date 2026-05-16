use ratatui::{layout::Rect, style::Color};

use crate::renderer::{
    glyphs::{BRAILLE_RAMP, DENSITY_RAMP_ASCII, DENSITY_RAMP_UNICODE},
    sprite::{detail_for_area, DetailLevel},
};

pub fn detail_star_glyph(hash: u64, detail: DetailLevel) -> char {
    match detail {
        DetailLevel::Tiny => {
            if hash.is_multiple_of(2) {
                '.'
            } else {
                '·'
            }
        }
        DetailLevel::Compact => {
            DENSITY_RAMP_ASCII[(hash % DENSITY_RAMP_ASCII.len() as u64) as usize]
        }
        DetailLevel::Standard => {
            DENSITY_RAMP_UNICODE[(hash % DENSITY_RAMP_UNICODE.len() as u64) as usize]
        }
        DetailLevel::Cinematic => BRAILLE_RAMP[(hash % BRAILLE_RAMP.len() as u64) as usize],
    }
}

pub fn star_magnitude_color(hash: u64, twinkle_hash: u64) -> Color {
    let (r, g, b) = match hash % 5 {
        0 => (109u8, 127u8, 170u8),
        1 => (130, 148, 194),
        2 => (152, 171, 224),
        3 => (181, 201, 246),
        _ => (195, 214, 250),
    };
    let bump = match twinkle_hash % 3 {
        0 => 0,
        1 => 10,
        _ => 20,
    };
    Color::Rgb(
        r.saturating_add(bump),
        g.saturating_add(bump),
        b.saturating_add(bump),
    )
}

pub fn should_render_star(hash: u64, detail: DetailLevel) -> bool {
    let divisor = match detail {
        DetailLevel::Tiny => 43,
        DetailLevel::Compact => 55,
        DetailLevel::Standard => 67,
        DetailLevel::Cinematic => 79,
    };
    hash.is_multiple_of(divisor)
}

pub fn nebula_density_glyph(hash: u64) -> char {
    DENSITY_RAMP_UNICODE[(hash % DENSITY_RAMP_UNICODE.len() as u64) as usize]
}

pub fn starfield_detail(area: Rect) -> DetailLevel {
    detail_for_area(area)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nebula_density_glyph_is_deterministic() {
        assert_eq!(nebula_density_glyph(42), nebula_density_glyph(42));
        assert_ne!(nebula_density_glyph(1), nebula_density_glyph(4));
    }
}
