use ratatui::{buffer::Buffer, layout::Rect, style::Style};

use crate::renderer::{
    palette::ColorToken,
    sprite::{AlphaMode, Sprite},
};

#[derive(Debug, Clone, Copy)]
struct CanvasCell {
    glyph: char,
    style: Style,
    z: u16,
}

#[derive(Debug, Clone)]
pub struct Canvas {
    width: u16,
    height: u16,
    cells: Vec<Option<CanvasCell>>,
}

impl Canvas {
    pub fn new(width: u16, height: u16) -> Self {
        let len = usize::from(width) * usize::from(height);
        Self {
            width,
            height,
            cells: vec![None; len],
        }
    }

    pub fn set_cell(&mut self, x: u16, y: u16, glyph: char, style: Style, z: u16) {
        if x >= self.width || y >= self.height {
            return;
        }
        let idx = self.index(x, y);
        if self.cells[idx].is_some_and(|cell| cell.z > z) {
            return;
        }
        self.cells[idx] = Some(CanvasCell { glyph, style, z });
    }

    pub fn draw_text(&mut self, x: u16, y: u16, text: &str, style: Style, z: u16) {
        if y >= self.height {
            return;
        }
        for (offset, ch) in text.chars().enumerate() {
            let xx = x.saturating_add(offset as u16);
            if xx >= self.width {
                break;
            }
            self.set_cell(xx, y, ch, style, z);
        }
    }

    pub fn draw_sprite(&mut self, sprite: &Sprite, x: u16, y: u16, frame_index: usize, z: u16) {
        if sprite.frames.is_empty() {
            return;
        }
        let frame = &sprite.frames[frame_index % sprite.frames.len()];
        for cell in &frame.cells {
            let xx = x.saturating_add(cell.x);
            let yy = y.saturating_add(cell.y);
            if xx >= self.width || yy >= self.height {
                continue;
            }
            match cell.alpha {
                AlphaMode::Transparent => continue,
                AlphaMode::Opaque => {
                    let style = cell.fg.to_style(cell.bg);
                    self.set_cell(xx, yy, cell.glyph, style, z);
                }
                AlphaMode::BlendGlyph => {
                    let idx = self.index(xx, yy);
                    let style = if let Some(existing) = self.cells[idx] {
                        let mut blended = cell.fg.to_style(cell.bg);
                        blended.bg = existing.style.bg;
                        blended
                    } else {
                        cell.fg.to_style(cell.bg)
                    };
                    self.set_cell(xx, yy, cell.glyph, style, z);
                }
            }
        }
    }

    pub fn render_to_buffer(&self, area: Rect, buf: &mut Buffer) {
        let width = self.width.min(area.width);
        let height = self.height.min(area.height);
        for y in 0..height {
            for x in 0..width {
                let idx = self.index(x, y);
                let Some(cell) = self.cells[idx] else {
                    continue;
                };
                if let Some(target) = buf.cell_mut((area.x + x, area.y + y)) {
                    target.set_char(cell.glyph);
                    target.set_style(cell.style);
                }
            }
        }
    }

    pub fn fill(&mut self, token: ColorToken, z: u16) {
        for y in 0..self.height {
            for x in 0..self.width {
                self.set_cell(x, y, ' ', token.to_style(Some(token)), z);
            }
        }
    }

    fn index(&self, x: u16, y: u16) -> usize {
        usize::from(y) * usize::from(self.width) + usize::from(x)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderer::palette::ColorToken;
    use crate::renderer::sprite::{AlphaMode, Sprite, SpriteCell, SpriteFrame};

    fn single_cell_sprite(alpha: AlphaMode, glyph: char) -> Sprite {
        Sprite {
            width: 1,
            height: 1,
            frames: vec![SpriteFrame {
                cells: vec![SpriteCell {
                    x: 0,
                    y: 0,
                    glyph,
                    fg: ColorToken::Accent,
                    bg: None,
                    alpha,
                }],
            }],
        }
    }

    #[test]
    fn canvas_clips_out_of_bounds_writes() {
        let mut canvas = Canvas::new(2, 2);
        canvas.set_cell(3, 0, 'X', ColorToken::Default.to_style(None), 1);

        let mut buf = Buffer::empty(Rect::new(0, 0, 2, 2));
        canvas.render_to_buffer(Rect::new(0, 0, 2, 2), &mut buf);
        assert_eq!(buf.cell((1, 1)).unwrap().symbol(), " ");
    }

    #[test]
    fn canvas_higher_z_overrides_lower_z() {
        let mut canvas = Canvas::new(2, 1);
        canvas.set_cell(0, 0, 'a', ColorToken::Default.to_style(None), 1);
        canvas.set_cell(0, 0, 'b', ColorToken::Accent.to_style(None), 2);

        let mut buf = Buffer::empty(Rect::new(0, 0, 2, 1));
        canvas.render_to_buffer(Rect::new(0, 0, 2, 1), &mut buf);
        assert_eq!(buf.cell((0, 0)).unwrap().symbol(), "b");
    }

    #[test]
    fn transparent_sprite_cells_do_not_overwrite() {
        let mut canvas = Canvas::new(1, 1);
        canvas.set_cell(0, 0, 'z', ColorToken::Default.to_style(None), 1);
        canvas.draw_sprite(&single_cell_sprite(AlphaMode::Transparent, 'x'), 0, 0, 0, 2);

        let mut buf = Buffer::empty(Rect::new(0, 0, 1, 1));
        canvas.render_to_buffer(Rect::new(0, 0, 1, 1), &mut buf);
        assert_eq!(buf.cell((0, 0)).unwrap().symbol(), "z");
    }
}
