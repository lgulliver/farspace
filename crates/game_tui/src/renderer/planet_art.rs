use game_core::{Colony, PlanetClass, SpectralClass};

use crate::renderer::{
    palette::ColorToken,
    sprite::{AlphaMode, DetailLevel, Sprite, SpriteCell, SpriteFrame},
};

const MAX_VISUAL_POPULATION: u64 = 32;
const POPULATION_SCALE_FACTOR: u64 = 8;
const POPULATION_PER_CITY_LIGHT: u8 = 40;
const HIGH_POLLUTION_THRESHOLD: u8 = 128;
const HIGH_INDUSTRY_THRESHOLD: u8 = 96;
// Terminal glyph cells are typically taller than they are wide. These coefficients
// bias the radial falloff so spheres render visually round instead of vertically oval.
// Approximation is width:height ≈ 4:9 for common monospace terminal fonts.
const TERMINAL_ASPECT_X: i16 = 4;
const TERMINAL_ASPECT_Y: i16 = 9;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanetVisualKind {
    Terran,
    Ocean,
    Arid,
    Desert,
    Ice,
    Barren,
    Volcanic,
    GasGiant,
    Toxic,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColonyPortraitInput {
    pub planet_kind: PlanetVisualKind,
    pub population_level: u8,
    pub industry_level: u8,
    pub pollution_level: u8,
    pub has_orbital_infrastructure: bool,
}

pub fn planet_kind_from_class(class: Option<PlanetClass>) -> PlanetVisualKind {
    match class {
        Some(PlanetClass::Terran) => PlanetVisualKind::Terran,
        Some(PlanetClass::Oceanic) => PlanetVisualKind::Ocean,
        Some(PlanetClass::Desert) => PlanetVisualKind::Desert,
        Some(PlanetClass::Frozen) => PlanetVisualKind::Ice,
        Some(PlanetClass::Volcanic) => PlanetVisualKind::Volcanic,
        Some(PlanetClass::Barren) => PlanetVisualKind::Barren,
        None => PlanetVisualKind::Unknown,
    }
}

pub fn portrait_input_from_colony(
    planet_class: Option<PlanetClass>,
    colony: Option<&Colony>,
) -> ColonyPortraitInput {
    let population = colony
        .map(|c| {
            let scaled = c.population.min(MAX_VISUAL_POPULATION) * POPULATION_SCALE_FACTOR;
            scaled.min(u64::from(u8::MAX)) as u8
        })
        .unwrap_or_default();
    let industry = colony
        .map(|c| {
            let capped = c.production.min(255);
            u8::try_from(capped).unwrap_or(u8::MAX)
        })
        .unwrap_or_default();

    ColonyPortraitInput {
        planet_kind: planet_kind_from_class(planet_class),
        population_level: population,
        industry_level: industry,
        pollution_level: 0,
        has_orbital_infrastructure: colony
            .map(|c| !c.orbital_installations.is_empty())
            .unwrap_or(false),
    }
}

pub fn planet_sprite(kind: PlanetVisualKind, detail: DetailLevel) -> Sprite {
    let mut sprite = sphere_sprite(
        kind_primary_color(kind),
        kind_secondary_color(kind),
        kind_glyph(kind),
        detail,
    );
    apply_planet_kind_overlay(&mut sprite, kind, detail);
    sprite
}

pub fn star_sprite(class: SpectralClass, detail: DetailLevel) -> Sprite {
    let (primary, secondary) = spectral_class_color_tokens(class);
    // All game stars are single-body; '☉' is the standard star glyph for tiny detail.
    sphere_sprite(primary, secondary, '☉', detail)
}

fn spectral_class_color_tokens(class: SpectralClass) -> (ColorToken, ColorToken) {
    // Hot blue/white classes use StarCold/Default; yellow/orange/red classes use StarWarm,
    // Warning (orange), and Error (red) — these semantic tokens happen to match the
    // physical star colours and are the closest available tokens in the current palette.
    // If dedicated StarOrange/StarRed tokens are added later, update this mapping.
    match class {
        SpectralClass::O | SpectralClass::B => (ColorToken::StarCold, ColorToken::Default),
        SpectralClass::A => (ColorToken::Default, ColorToken::StarCold),
        SpectralClass::F | SpectralClass::G => (ColorToken::StarWarm, ColorToken::Default),
        SpectralClass::K => (ColorToken::Warning, ColorToken::StarWarm),
        SpectralClass::M => (ColorToken::Error, ColorToken::Warning),
    }
}

fn sphere_sprite(
    primary: ColorToken,
    secondary: ColorToken,
    tiny_glyph: char,
    detail: DetailLevel,
) -> Sprite {
    let (width, height, radius) = match detail {
        DetailLevel::Tiny => (1, 1, 0i16),
        DetailLevel::Compact => (5, 3, 2),
        DetailLevel::Standard => (9, 7, 4),
        DetailLevel::Cinematic => (17, 11, 5),
    };

    if matches!(detail, DetailLevel::Tiny) {
        return Sprite {
            width,
            height,
            frames: vec![SpriteFrame {
                cells: vec![SpriteCell {
                    x: 0,
                    y: 0,
                    glyph: tiny_glyph,
                    fg: primary,
                    bg: None,
                    alpha: AlphaMode::Opaque,
                }],
            }],
        };
    }

    let cx = i16::try_from(width / 2).unwrap_or(0);
    let cy = i16::try_from(height / 2).unwrap_or(0);
    let max_dist = radius * radius * TERMINAL_ASPECT_X;
    let mut cells = Vec::new();

    for y in 0..height {
        for x in 0..width {
            let dx = i16::try_from(x).unwrap_or(0) - cx;
            let dy = i16::try_from(y).unwrap_or(0) - cy;
            let dist = dx * dx * TERMINAL_ASPECT_X + dy * dy * TERMINAL_ASPECT_Y;
            if dist > max_dist {
                continue;
            }
            let shade = ((max_dist - dist) * 255 / max_dist.max(1)) as u8;
            let glyph = if shade > 220 {
                '●'
            } else if shade > 180 {
                '◉'
            } else if shade > 140 {
                '◍'
            } else if shade > 90 {
                '○'
            } else if shade > 40 {
                '·'
            } else {
                ' '
            };
            let fg = if dx < 0 { primary } else { secondary };
            cells.push(SpriteCell {
                x,
                y,
                glyph,
                fg,
                bg: None,
                alpha: AlphaMode::Opaque,
            });
        }
    }

    Sprite {
        width,
        height,
        frames: vec![SpriteFrame { cells }],
    }
}

pub fn colony_portrait(input: ColonyPortraitInput, detail: DetailLevel) -> Sprite {
    let mut base = planet_sprite(input.planet_kind, detail);
    if base.frames.is_empty() {
        return base;
    }

    let mut frame = base.frames.remove(0);
    if detail != DetailLevel::Tiny {
        let horizon_y = base.height.saturating_sub(2);
        for x in 1..base.width.saturating_sub(1) {
            frame.cells.push(SpriteCell {
                x,
                y: horizon_y,
                glyph: '_',
                fg: ColorToken::DimOverlay,
                bg: None,
                alpha: AlphaMode::BlendGlyph,
            });
        }
        let city_lights = usize::from(input.population_level / POPULATION_PER_CITY_LIGHT).max(1);
        for i in 0..city_lights {
            let x = ((i * 3 + 2) % usize::from(base.width)) as u16;
            let y = ((i * 2 + 1) % usize::from(base.height)) as u16;
            frame.cells.push(SpriteCell {
                x,
                y,
                glyph: if input.pollution_level > HIGH_POLLUTION_THRESHOLD {
                    '▒'
                } else {
                    '▪'
                },
                fg: ColorToken::ColonyLight,
                bg: None,
                alpha: AlphaMode::BlendGlyph,
            });
        }
        if input.population_level >= POPULATION_PER_CITY_LIGHT {
            for i in 0..usize::from(base.width.saturating_sub(4) / 2) {
                let x = 2 + (i as u16 * 2);
                let height = (i as u16 % 3) + 1;
                for step in 0..height {
                    frame.cells.push(SpriteCell {
                        x,
                        y: horizon_y.saturating_sub(step),
                        glyph: '▮',
                        fg: ColorToken::Muted,
                        bg: None,
                        alpha: AlphaMode::BlendGlyph,
                    });
                }
            }
        }
        if input.has_orbital_infrastructure {
            frame.cells.push(SpriteCell {
                x: base.width.saturating_sub(2),
                y: 1,
                glyph: '◌',
                fg: ColorToken::Accent2,
                bg: None,
                alpha: AlphaMode::Opaque,
            });
        }
        if input.industry_level > HIGH_INDUSTRY_THRESHOLD {
            frame.cells.push(SpriteCell {
                x: base.width / 2,
                y: base.height.saturating_sub(2),
                glyph: '▦',
                fg: ColorToken::Warning,
                bg: None,
                alpha: AlphaMode::BlendGlyph,
            });
        }
    }

    base.frames = vec![frame];
    base
}

fn apply_planet_kind_overlay(sprite: &mut Sprite, kind: PlanetVisualKind, detail: DetailLevel) {
    if matches!(detail, DetailLevel::Tiny) || sprite.frames.is_empty() {
        return;
    }

    let mut frame = sprite.frames.remove(0);
    let width = sprite.width;
    let height = sprite.height;

    match kind {
        PlanetVisualKind::Terran => {
            add_cells(
                &mut frame,
                &[
                    (2, 2, '~', ColorToken::PlanetWater),
                    (width / 2, 1, '~', ColorToken::Default),
                    (
                        width.saturating_sub(3),
                        height / 2,
                        '~',
                        ColorToken::PlanetWater,
                    ),
                ],
            );
        }
        PlanetVisualKind::Ocean => {
            for x in 2..width.saturating_sub(2) {
                if x % 2 == 0 {
                    frame.cells.push(SpriteCell {
                        x,
                        y: height / 2,
                        glyph: '=',
                        fg: ColorToken::Default,
                        bg: None,
                        alpha: AlphaMode::BlendGlyph,
                    });
                }
            }
        }
        PlanetVisualKind::Arid | PlanetVisualKind::Desert => {
            add_cells(
                &mut frame,
                &[
                    (2, height / 2, '=', ColorToken::Accent2),
                    (width / 2, height / 2 + 1, '~', ColorToken::PlanetDesert),
                    (
                        width.saturating_sub(3),
                        height / 2,
                        '=',
                        ColorToken::Accent2,
                    ),
                ],
            );
        }
        PlanetVisualKind::Ice => {
            add_cells(
                &mut frame,
                &[
                    (width / 2, 1, '^', ColorToken::Default),
                    (2, 2, '*', ColorToken::PlanetIce),
                    (width.saturating_sub(3), 2, '*', ColorToken::PlanetIce),
                ],
            );
        }
        PlanetVisualKind::Volcanic => {
            add_cells(
                &mut frame,
                &[
                    (2, 2, '#', ColorToken::Warning),
                    (width / 2, height / 2, '#', ColorToken::PlanetLava),
                    (
                        width.saturating_sub(3),
                        height.saturating_sub(3),
                        '#',
                        ColorToken::Warning,
                    ),
                ],
            );
        }
        PlanetVisualKind::Barren => {
            add_cells(
                &mut frame,
                &[
                    (2, 2, 'o', ColorToken::DimOverlay),
                    (width / 2, height / 2, 'o', ColorToken::DimOverlay),
                    (width.saturating_sub(3), 2, 'o', ColorToken::DimOverlay),
                ],
            );
        }
        PlanetVisualKind::GasGiant => {
            for y in 1..height.saturating_sub(1) {
                if y % 2 == 1 {
                    for x in 2..width.saturating_sub(2) {
                        frame.cells.push(SpriteCell {
                            x,
                            y,
                            glyph: '=',
                            fg: ColorToken::StarWarm,
                            bg: None,
                            alpha: AlphaMode::BlendGlyph,
                        });
                    }
                }
            }
        }
        PlanetVisualKind::Toxic => {
            add_cells(
                &mut frame,
                &[
                    (2, 2, 'x', ColorToken::Warning),
                    (width / 2, height / 2, 'x', ColorToken::Error),
                    (width.saturating_sub(3), 2, 'x', ColorToken::Warning),
                ],
            );
        }
        PlanetVisualKind::Unknown => {
            add_cells(
                &mut frame,
                &[
                    (2, 2, '?', ColorToken::Muted),
                    (width / 2, height / 2, '?', ColorToken::Muted),
                    (
                        width.saturating_sub(3),
                        height.saturating_sub(3),
                        '?',
                        ColorToken::Muted,
                    ),
                ],
            );
        }
    }

    sprite.frames = vec![frame];
}

fn add_cells(frame: &mut SpriteFrame, cells: &[(u16, u16, char, ColorToken)]) {
    for (x, y, glyph, fg) in cells {
        frame.cells.push(SpriteCell {
            x: *x,
            y: *y,
            glyph: *glyph,
            fg: *fg,
            bg: None,
            alpha: AlphaMode::BlendGlyph,
        });
    }
}

fn kind_primary_color(kind: PlanetVisualKind) -> ColorToken {
    match kind {
        PlanetVisualKind::Terran => ColorToken::PlanetLand,
        PlanetVisualKind::Ocean => ColorToken::PlanetWater,
        PlanetVisualKind::Arid | PlanetVisualKind::Desert => ColorToken::PlanetDesert,
        PlanetVisualKind::Ice => ColorToken::PlanetIce,
        PlanetVisualKind::Volcanic => ColorToken::PlanetLava,
        PlanetVisualKind::Barren => ColorToken::Muted,
        PlanetVisualKind::GasGiant => ColorToken::Accent2,
        PlanetVisualKind::Toxic => ColorToken::Warning,
        PlanetVisualKind::Unknown => ColorToken::Default,
    }
}

fn kind_secondary_color(kind: PlanetVisualKind) -> ColorToken {
    match kind {
        PlanetVisualKind::Terran => ColorToken::PlanetWater,
        PlanetVisualKind::Ocean => ColorToken::StarCold,
        PlanetVisualKind::Arid | PlanetVisualKind::Desert => ColorToken::Accent2,
        PlanetVisualKind::Ice => ColorToken::Default,
        PlanetVisualKind::Volcanic => ColorToken::Warning,
        PlanetVisualKind::Barren => ColorToken::DimOverlay,
        PlanetVisualKind::GasGiant => ColorToken::StarWarm,
        PlanetVisualKind::Toxic => ColorToken::Error,
        PlanetVisualKind::Unknown => ColorToken::Muted,
    }
}

fn kind_glyph(kind: PlanetVisualKind) -> char {
    match kind {
        PlanetVisualKind::Terran => '●',
        PlanetVisualKind::Ocean => '◉',
        PlanetVisualKind::Arid | PlanetVisualKind::Desert => '◍',
        PlanetVisualKind::Ice => '◌',
        PlanetVisualKind::Barren => '○',
        PlanetVisualKind::Volcanic => '◐',
        PlanetVisualKind::GasGiant => '◎',
        PlanetVisualKind::Toxic => '◒',
        PlanetVisualKind::Unknown => '○',
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn planet_kind_mapping_has_safe_fallback() {
        assert_eq!(planet_kind_from_class(None), PlanetVisualKind::Unknown);
        assert_eq!(
            planet_kind_from_class(Some(PlanetClass::Oceanic)),
            PlanetVisualKind::Ocean
        );
        assert_eq!(
            planet_kind_from_class(Some(PlanetClass::Frozen)),
            PlanetVisualKind::Ice
        );
    }

    #[test]
    fn planet_sprite_adds_class_signature_overlays() {
        let sprite = planet_sprite(PlanetVisualKind::Volcanic, DetailLevel::Standard);
        let glyphs: BTreeSet<char> = sprite.frames[0]
            .cells
            .iter()
            .map(|cell| cell.glyph)
            .collect();

        assert!(glyphs.contains(&'#'));
        assert!(glyphs.contains(&'◉') || glyphs.contains(&'●'));
    }

    #[test]
    fn colony_portrait_adds_horizon_and_skyline_for_settled_worlds() {
        let portrait = colony_portrait(
            ColonyPortraitInput {
                planet_kind: PlanetVisualKind::Terran,
                population_level: 120,
                industry_level: 120,
                pollution_level: 0,
                has_orbital_infrastructure: true,
            },
            DetailLevel::Standard,
        );
        let glyphs: BTreeSet<char> = portrait.frames[0]
            .cells
            .iter()
            .map(|cell| cell.glyph)
            .collect();

        assert!(glyphs.contains(&'_'));
        assert!(glyphs.contains(&'▮'));
        assert!(glyphs.contains(&'◌'));
    }
}
