use ratatui::layout::Rect;

use crate::renderer::palette::ColorToken;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetailLevel {
    Tiny,
    Compact,
    Standard,
    Cinematic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlphaMode {
    Opaque,
    Transparent,
    BlendGlyph,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sprite {
    pub width: u16,
    pub height: u16,
    pub frames: Vec<SpriteFrame>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpriteFrame {
    pub cells: Vec<SpriteCell>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpriteCell {
    pub x: u16,
    pub y: u16,
    pub glyph: char,
    pub fg: ColorToken,
    pub bg: Option<ColorToken>,
    pub alpha: AlphaMode,
}

pub fn detail_for_area(area: Rect) -> DetailLevel {
    let area_size = u32::from(area.width) * u32::from(area.height);
    if area.width < 24 || area.height < 10 || area_size < 320 {
        DetailLevel::Tiny
    } else if area.width < 50 || area.height < 16 || area_size < 900 {
        DetailLevel::Compact
    } else if area.width < 90 || area.height < 28 || area_size < 2200 {
        DetailLevel::Standard
    } else {
        DetailLevel::Cinematic
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detail_selection_respects_area_size() {
        assert_eq!(detail_for_area(Rect::new(0, 0, 20, 8)), DetailLevel::Tiny);
        assert_eq!(
            detail_for_area(Rect::new(0, 0, 40, 14)),
            DetailLevel::Compact
        );
        assert_eq!(
            detail_for_area(Rect::new(0, 0, 70, 22)),
            DetailLevel::Standard
        );
        assert_eq!(
            detail_for_area(Rect::new(0, 0, 120, 40)),
            DetailLevel::Cinematic
        );
    }
}
