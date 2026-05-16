use ratatui::{layout::Rect, style::Color};

use crate::renderer::{
    glyphs::{BRAILLE_RAMP, DENSITY_RAMP_ASCII, DENSITY_RAMP_UNICODE},
    sprite::DetailLevel,
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
        DetailLevel::Compact => DENSITY_RAMP_ASCII[(hash % DENSITY_RAMP_ASCII.len() as u64) as usize],
        DetailLevel::Standard => {
            DENSITY_RAMP_UNICODE[(hash % DENSITY_RAMP_UNICODE.len() as u64) as usize]
        }
        DetailLevel::Cinematic => BRAILLE_RAMP[(hash % BRAILLE_RAMP.len() as u64) as usize],
    }
}

pub fn star_magnitude_color(hash: u64, twinkle_hash: u64) -> Color {
    match hash % 5 {
        0 => Color::Rgb(109, 127, 170),
        1 => Color::Rgb(130, 148, 194),
        2 => Color::Rgb(152, 171, 224),
        3 => Color::Rgb(181, 201, 246),
        _ => {
            if twinkle_hash.is_multiple_of(2) {
                Color::Rgb(210, 225, 255)
            } else {
                Color::Rgb(170, 198, 245)
            }
        }
    }
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

pub fn detail_for_map_area(area: Rect) -> DetailLevel {
    crate::renderer::sprite::detail_for_area(area)
}
