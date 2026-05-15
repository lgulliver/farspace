use std::collections::{BTreeMap, BTreeSet};

use ratatui::{buffer::Buffer, layout::Rect, style::Style, widgets::Widget};

// SplitMix64-style mixing constants used only for deterministic cosmetic hashing.
const FRAME_MIX: u64 = 0x9E37_79B9_7F4A_7C15;
const SALT_MIX: u64 = 0xBF58_476D_1CE4_E5B9;
const FINAL_MIX: u64 = 0x94D0_49BB_1331_11EB;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MapLayer {
    Background,
    Territory,
    Fog,
    Route,
    Entity,
    Overlay,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LabelPlacement {
    Right,
    Left,
    Below,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CellCommand {
    pub layer: MapLayer,
    pub order: u16,
    pub x: u16,
    pub y: u16,
    pub symbol: Option<char>,
    pub style: Style,
    pub protect: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LabelCommand {
    pub text: String,
    pub anchor: (u16, u16),
    pub style: Style,
    pub priority: u8,
    pub placements: Vec<LabelPlacement>,
}

#[derive(Debug, Clone)]
pub struct HaloSpec {
    pub radius_x: i16,
    pub radius_y: i16,
    pub style: Style,
    pub layer: MapLayer,
    pub order: u16,
}

#[derive(Debug, Clone)]
pub struct LayeredMap {
    pub base_style: Style,
    pub cells: Vec<CellCommand>,
    pub labels: Vec<LabelCommand>,
}

impl Widget for LayeredMap {
    fn render(mut self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        paint_base(area, buf, self.base_style);

        self.cells
            .sort_by_key(|cmd| (cmd.layer, cmd.order, cmd.y, cmd.x));

        let mut protected = BTreeMap::<(u16, u16), u8>::new();

        for command in &self.cells {
            if command.x >= area.width || command.y >= area.height {
                continue;
            }
            let gx = area.x + command.x;
            let gy = area.y + command.y;
            if let Some(cell) = buf.cell_mut((gx, gy)) {
                if let Some(symbol) = command.symbol {
                    cell.set_char(symbol);
                }
                cell.set_style(command.style);
                if command.protect > 0 {
                    protected
                        .entry((command.x, command.y))
                        .and_modify(|current| *current = (*current).max(command.protect))
                        .or_insert(command.protect);
                }
            }
        }

        self.labels.sort_by(|a, b| {
            b.priority
                .cmp(&a.priority)
                .then_with(|| a.anchor.1.cmp(&b.anchor.1))
                .then_with(|| a.anchor.0.cmp(&b.anchor.0))
                .then_with(|| a.text.cmp(&b.text))
        });

        let mut used = BTreeSet::<(u16, u16)>::new();
        for label in &self.labels {
            if let Some((x, y)) = place_label(label, area, &protected, &used) {
                let mut current_x = x;
                for ch in label.text.chars() {
                    if let Some(cell) = buf.cell_mut((area.x + current_x, area.y + y)) {
                        cell.set_char(ch);
                        cell.set_style(label.style);
                    }
                    used.insert((current_x, y));
                    current_x = current_x.saturating_add(1);
                }
            }
        }
    }
}

/// Return a deterministic cosmetic hash for map background effects.
///
/// `seed` is the game seed, `(x, y)` is the local cell position, `frame` is a
/// cosmetic animation frame group, and `salt` separates different visual layers.
pub fn visual_hash(seed: u64, x: u16, y: u16, frame: u64, salt: u64) -> u64 {
    let mut value = seed
        ^ (u64::from(x) << 17)
        ^ (u64::from(y) << 31)
        ^ frame.wrapping_mul(FRAME_MIX)
        ^ salt.wrapping_mul(SALT_MIX);
    value = value.wrapping_add(FRAME_MIX);
    value = (value ^ (value >> 30)).wrapping_mul(SALT_MIX);
    value = (value ^ (value >> 27)).wrapping_mul(FINAL_MIX);
    value ^ (value >> 31)
}

/// Push an elliptical halo of styled cells into a layered map buffer.
///
/// `cells` receives the generated commands, `center` is the halo origin,
/// `bounds` is the local map size used for clipping, and `spec` defines the
/// halo radii, style, layer, and draw order.
pub fn push_halo(
    cells: &mut Vec<CellCommand>,
    center: (u16, u16),
    bounds: (u16, u16),
    spec: &HaloSpec,
) {
    let (center_x, center_y) = center;
    let (width, height) = bounds;
    for dx in -spec.radius_x..=spec.radius_x {
        for dy in -spec.radius_y..=spec.radius_y {
            if (dx * dx * 4) + (dy * dy * 9) > (spec.radius_x * spec.radius_x * 4) {
                continue;
            }
            let x = i32::from(center_x) + i32::from(dx);
            let y = i32::from(center_y) + i32::from(dy);
            if x < 0 || y < 0 || x >= i32::from(width) || y >= i32::from(height) {
                continue;
            }
            cells.push(CellCommand {
                layer: spec.layer,
                order: spec.order,
                x: x as u16,
                y: y as u16,
                symbol: None,
                style: spec.style,
                protect: 0,
            });
        }
    }
}

fn paint_base(area: Rect, buf: &mut Buffer, style: Style) {
    for y in 0..area.height {
        for x in 0..area.width {
            if let Some(cell) = buf.cell_mut((area.x + x, area.y + y)) {
                cell.set_style(style);
            }
        }
    }
}

fn place_label(
    label: &LabelCommand,
    area: Rect,
    protected: &BTreeMap<(u16, u16), u8>,
    used: &BTreeSet<(u16, u16)>,
) -> Option<(u16, u16)> {
    let width = label.text.chars().count() as u16;
    if width == 0 || width > area.width {
        return None;
    }

    for placement in &label.placements {
        let (x, y) = match placement {
            LabelPlacement::Right => {
                let x = label.anchor.0.saturating_add(2);
                (x, label.anchor.1)
            }
            LabelPlacement::Left => {
                if label.anchor.0 <= width {
                    continue;
                }
                (label.anchor.0 - width - 1, label.anchor.1)
            }
            LabelPlacement::Below => {
                let y = label.anchor.1.saturating_add(1);
                (
                    label
                        .anchor
                        .0
                        .saturating_add(1)
                        .min(area.width.saturating_sub(width)),
                    y,
                )
            }
        };

        if y >= area.height || x + width > area.width {
            continue;
        }

        if (0..width).any(|offset| {
            let point = (x + offset, y);
            protected.get(&point).copied().unwrap_or_default() > 0 || used.contains(&point)
        }) {
            continue;
        }

        return Some((x, y));
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_VISUAL_SALT: u64 = 11;

    #[test]
    fn visual_hash_is_deterministic() {
        assert_eq!(
            visual_hash(42, 3, 7, 1, TEST_VISUAL_SALT),
            visual_hash(42, 3, 7, 1, TEST_VISUAL_SALT)
        );
        assert_ne!(
            visual_hash(42, 3, 7, 1, TEST_VISUAL_SALT),
            visual_hash(42, 3, 7, 2, TEST_VISUAL_SALT)
        );
    }
}
