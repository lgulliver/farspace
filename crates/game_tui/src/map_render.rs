use std::collections::{BTreeMap, BTreeSet};

use ratatui::{buffer::Buffer, layout::Rect, style::Style, widgets::Widget};

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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WorldProjection {
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
    width: u16,
    height: u16,
}

impl WorldProjection {
    pub fn new(bounds: (f64, f64, f64, f64), width: u16, height: u16) -> Self {
        let (mut min_x, mut min_y, mut max_x, mut max_y) = bounds;
        if (max_x - min_x).abs() < f64::EPSILON {
            min_x -= 1.0;
            max_x += 1.0;
        }
        if (max_y - min_y).abs() < f64::EPSILON {
            min_y -= 1.0;
            max_y += 1.0;
        }

        Self {
            min_x,
            min_y,
            max_x,
            max_y,
            width,
            height,
        }
    }

    pub fn from_points(
        points: &[(f64, f64)],
        width: u16,
        height: u16,
        padding: f64,
    ) -> Option<Self> {
        if width == 0 || height == 0 || points.is_empty() {
            return None;
        }

        let mut min_x = points[0].0;
        let mut max_x = points[0].0;
        let mut min_y = points[0].1;
        let mut max_y = points[0].1;

        for &(x, y) in points.iter().skip(1) {
            min_x = min_x.min(x);
            max_x = max_x.max(x);
            min_y = min_y.min(y);
            max_y = max_y.max(y);
        }

        Some(Self::new(
            (
                min_x - padding,
                min_y - padding,
                max_x + padding,
                max_y + padding,
            ),
            width,
            height,
        ))
    }

    pub fn project(&self, wx: f64, wy: f64) -> Option<(u16, u16)> {
        if self.width == 0 || self.height == 0 {
            return None;
        }
        let rel_x = (wx - self.min_x) / (self.max_x - self.min_x);
        let rel_y = (wy - self.min_y) / (self.max_y - self.min_y);
        let x = (rel_x * f64::from(self.width.saturating_sub(1))).round() as i32;
        let y = (rel_y * f64::from(self.height.saturating_sub(1))).round() as i32;
        Some((
            x.clamp(0, i32::from(self.width.saturating_sub(1))) as u16,
            y.clamp(0, i32::from(self.height.saturating_sub(1))) as u16,
        ))
    }
}

pub fn sample_line(start: (f64, f64), end: (f64, f64), spacing: f64) -> Vec<(f64, f64)> {
    let (x0, y0) = start;
    let (x1, y1) = end;
    let steps = ((x1 - x0).abs().max((y1 - y0).abs()) / spacing)
        .ceil()
        .max(1.0) as usize;
    (0..=steps)
        .map(|step| {
            let t = step as f64 / steps as f64;
            (x0 + (x1 - x0) * t, y0 + (y1 - y0) * t)
        })
        .collect()
}

pub fn visual_hash(seed: u64, x: u16, y: u16, frame: u64, salt: u64) -> u64 {
    let mut value = seed
        ^ (u64::from(x) << 17)
        ^ (u64::from(y) << 31)
        ^ frame.wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ salt.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value = value.wrapping_add(0x9E37_79B9_7F4A_7C15);
    value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^ (value >> 31)
}

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

    #[test]
    fn projection_projects_into_area() {
        let projection = WorldProjection::new((-10.0, -10.0, 10.0, 10.0), 20, 10);
        assert_eq!(projection.project(-10.0, -10.0), Some((0, 0)));
        assert_eq!(projection.project(10.0, 10.0), Some((19, 9)));
    }

    #[test]
    fn visual_hash_is_deterministic() {
        assert_eq!(visual_hash(42, 3, 7, 1, 11), visual_hash(42, 3, 7, 1, 11));
        assert_ne!(visual_hash(42, 3, 7, 1, 11), visual_hash(42, 3, 7, 2, 11));
    }
}
