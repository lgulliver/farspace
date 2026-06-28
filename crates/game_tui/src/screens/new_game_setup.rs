//! New Game Setup screen — polished campaign wizard for choosing empire and scenario options.

use crate::app::AppState;
use crate::components::{
    EmblemPattern, EmpireEmblem, EmpireEmblemPalette, panel_block, render_empire_emblem,
    render_footer,
};
use crate::screens::Screen;
use crate::theme::{SplashPalette, Theme, gradient, lerp_rgb};
use game_core::{GalaxySize, VictoryPath, VictorySettings, all_empire_definitions};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph, Wrap},
};

/// Index of each editable setup field in `AppState::setup_cursor`.
pub const FIELD_EMPIRE: usize = 0;
pub const FIELD_GALAXY_SIZE: usize = 1;
pub const FIELD_AI_COUNT: usize = 2;
pub const FIELD_SEED: usize = 3;
pub const FIELD_DIFFICULTY: usize = 4;

const COMPACT_MIN_WIDTH: u16 = 80;
const COMPACT_MIN_HEIGHT: u16 = 24;
const PREMIUM_MIN_WIDTH: u16 = 96;
const PREMIUM_MIN_HEIGHT: u16 = 30;
const FIELD_LABEL_WIDTH: usize = 14;
const FIELD_VALUE_WIDTH: usize = 18;
const WIDE_LAYOUT_WIDTH: u16 = 118;
const CAMPAIGN_STAGE: usize = 2;

fn enter_hint(app_state: &AppState) -> &'static str {
    if app_state.new_game_setup.seed_editing {
        "Confirm Seed"
    } else if app_state.new_game_setup.cursor == FIELD_SEED {
        "Edit Seed"
    } else {
        "Start"
    }
}

fn pad_to_width(text: &str, width: usize) -> String {
    let len = text.chars().count();
    if len >= width {
        text.to_string()
    } else {
        format!("{text}{}", " ".repeat(width - len))
    }
}

fn selector_value(label: impl std::fmt::Display, has_prev: bool, has_next: bool) -> String {
    format!(
        "{}{}{}",
        if has_prev { "< " } else { "  " },
        label,
        if has_next { " >" } else { "  " }
    )
}

fn field_line(label: &str, value: &str, is_active: bool, content_width: u16) -> Line<'static> {
    let marker = if is_active { "▶" } else { " " };
    let text = format!(
        "{marker} {:<FIELD_LABEL_WIDTH$} {value:<FIELD_VALUE_WIDTH$}",
        label
    );
    let padded = pad_to_width(&text, usize::from(content_width));
    if is_active {
        return Line::from(Span::styled(padded, Theme::highlight_style()));
    }

    let prefix = format!("{marker} {label:<FIELD_LABEL_WIDTH$} ");
    let value_text = format!("{value:<FIELD_VALUE_WIDTH$}");
    let padding_width = usize::from(content_width)
        .saturating_sub(prefix.chars().count() + value_text.chars().count());

    Line::from(vec![
        Span::styled(prefix, Theme::default_style()),
        Span::styled(value_text, Theme::title_style()),
        Span::raw(" ".repeat(padding_width)),
    ])
}

fn gradient_word_spans(text: &str, start: Color, end: Color) -> Vec<Span<'static>> {
    let letters = text.chars().collect::<Vec<_>>();
    let colors = gradient(start, end, letters.len().max(1));
    letters
        .iter()
        .enumerate()
        .map(|(idx, ch)| {
            Span::styled(
                ch.to_string(),
                Style::default()
                    .fg(colors[idx])
                    .add_modifier(Modifier::BOLD),
            )
        })
        .collect()
}

fn title_band_line(app_state: &AppState, palette: SplashPalette) -> Line<'static> {
    let mut line = gradient_word_spans("FARSPACE", palette.title_primary, palette.title_secondary);
    line.push(Span::raw("  "));
    line.push(Span::styled(
        "CHART • EXPAND • ENDURE",
        Style::default()
            .fg(palette.accent)
            .add_modifier(Modifier::BOLD),
    ));
    line.push(Span::raw("  "));
    line.push(Span::styled(
        "New Campaign 2 / 5",
        Style::default().fg(palette.text_muted),
    ));
    if app_state.new_game_setup.seed_editing {
        line.push(Span::raw("  "));
        line.push(Span::styled("Seed editing", Theme::warning_style()));
    }
    Line::from(line)
}

fn stage_tracker_line(palette: SplashPalette) -> Line<'static> {
    let current = CAMPAIGN_STAGE;
    let stages = [
        (1usize, "Empire"),
        (2, "Galaxy"),
        (3, "Rules"),
        (4, "Summary"),
        (5, "Launch"),
    ];
    let mut spans = Vec::new();
    for (idx, label) in stages {
        if !spans.is_empty() {
            spans.push(Span::raw("   "));
        }
        let styled = if idx == current {
            Style::default()
                .fg(palette.accent)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(palette.text_muted)
        };
        spans.push(Span::styled(format!("{idx} {label}"), styled));
    }
    Line::from(spans)
}

fn render_wizard_surface(frame: &mut Frame, area: Rect, palette: SplashPalette) {
    frame.render_widget(
        Block::default().style(Style::default().bg(lerp_rgb(
            palette.void_bg,
            palette.nebula_a,
            0.10,
        ))),
        area,
    );
}

fn render_title_band(frame: &mut Frame, area: Rect, app_state: &AppState, palette: SplashPalette) {
    let lines = vec![
        title_band_line(app_state, palette),
        Line::from(""),
        stage_tracker_line(palette),
    ];
    let paragraph = Paragraph::new(lines)
        .alignment(Alignment::Center)
        .style(Theme::default_style());
    frame.render_widget(paragraph, area);
}

fn render_section_block(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    active: bool,
    palette: SplashPalette,
) {
    let border_style = if active {
        Style::default()
            .fg(palette.border_hot)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(palette.border_cold)
    };
    let block = panel_block(title, active)
        .border_style(border_style)
        .style(Style::default().bg(lerp_rgb(palette.void_bg, palette.nebula_b, 0.12)));
    frame.render_widget(block, area);
}

fn selected_empire(app_state: &AppState) -> (&'static game_core::EmpireDefinition, usize) {
    let all_defs = all_empire_definitions();
    let empire_idx = app_state
        .new_game_setup
        .empire_cursor
        .min(all_defs.len().saturating_sub(1));
    (&all_defs[empire_idx], empire_idx)
}

fn render_empire_section(
    frame: &mut Frame,
    area: Rect,
    app_state: &AppState,
    compact: bool,
    palette: SplashPalette,
) {
    let (def, index) = selected_empire(app_state);
    render_section_block(frame, area, "Empire", false, palette);
    let inner = Rect::new(
        area.x + 1,
        area.y + 1,
        area.width.saturating_sub(2),
        area.height.saturating_sub(2),
    );
    let palette = EmpireEmblemPalette::from_empire_index(index);
    let emblem = EmpireEmblem {
        pattern: EmblemPattern::from_empire_index(index),
        palette,
        symbol: def.symbol,
    };
    let defs_count = all_empire_definitions().len();
    let is_empire_active = app_state.new_game_setup.cursor == FIELD_EMPIRE;
    let empire_value = selector_value(
        format!("{} {}", def.symbol, def.name),
        index > 0,
        index + 1 < defs_count,
    );

    if compact || inner.width < 30 || inner.height < 9 {
        let lines = vec![
            field_line("Faction", &empire_value, is_empire_active, inner.width),
            Line::from(""),
            Line::from(vec![
                Span::styled("Name        ", Theme::muted_style()),
                Span::styled(def.name, Theme::title_style()),
            ]),
            Line::from(vec![
                Span::styled("Doctrine    ", Theme::muted_style()),
                Span::styled(
                    def.playstyle
                        .iter()
                        .map(|tag| tag.label())
                        .collect::<Vec<_>>()
                        .join(" / "),
                    Theme::default_style(),
                ),
            ]),
            Line::from(vec![
                Span::styled("Profile     ", Theme::muted_style()),
                Span::styled(def.short_description, Theme::muted_style()),
            ]),
            Line::from(vec![
                Span::styled("Seal        ", Theme::muted_style()),
                Span::styled(format!("{} {}", def.symbol, def.tone), Theme::title_style()),
            ]),
        ];
        frame.render_widget(
            Paragraph::new(lines)
                .wrap(Wrap { trim: false })
                .style(Theme::default_style()),
            inner,
        );
        return;
    }

    let [info_area, emblem_area] = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(62), Constraint::Percentage(38)])
        .areas(inner);
    let [info_top, info_bottom] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(8), Constraint::Fill(1)])
        .areas(info_area);

    let info_lines = vec![
        field_line("Faction", &empire_value, is_empire_active, info_top.width),
        Line::from(""),
        Line::from(vec![
            Span::styled("Name        ", Theme::muted_style()),
            Span::styled(def.name, Theme::title_style()),
        ]),
        Line::from(vec![
            Span::styled("Doctrine    ", Theme::muted_style()),
            Span::styled(
                def.playstyle
                    .iter()
                    .map(|tag| tag.label())
                    .collect::<Vec<_>>()
                    .join(" / "),
                Theme::default_style(),
            ),
        ]),
        Line::from(vec![
            Span::styled("Profile     ", Theme::muted_style()),
            Span::styled(def.short_description, Theme::muted_style()),
        ]),
        Line::from(vec![
            Span::styled("Seal        ", Theme::muted_style()),
            Span::styled(format!("{} {}", def.symbol, def.tone), Theme::title_style()),
        ]),
    ];
    frame.render_widget(
        Paragraph::new(info_lines)
            .wrap(Wrap { trim: false })
            .style(Theme::default_style()),
        info_top,
    );

    let doctrine_line = def
        .playstyle
        .iter()
        .map(|tag| tag.label())
        .collect::<Vec<_>>()
        .join("  ·  ");
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(vec![
                Span::styled("Doctrine", Theme::muted_style()),
                Span::raw("  "),
                Span::styled(doctrine_line, Theme::title_style()),
            ]),
            Line::from(vec![
                Span::styled("Tone    ", Theme::muted_style()),
                Span::styled(def.tone, Theme::default_style()),
            ]),
        ])
        .wrap(Wrap { trim: false })
        .style(Theme::default_style()),
        info_bottom,
    );

    render_empire_emblem(frame, emblem_area, &emblem);
}

fn render_galaxy_section(
    frame: &mut Frame,
    area: Rect,
    app_state: &AppState,
    palette: SplashPalette,
) {
    render_section_block(frame, area, "Galaxy", false, palette);
    let inner = Rect::new(
        area.x + 1,
        area.y + 1,
        area.width.saturating_sub(2),
        area.height.saturating_sub(2),
    );

    let is_size = app_state.new_game_setup.cursor == FIELD_GALAXY_SIZE;
    let is_ai = app_state.new_game_setup.cursor == FIELD_AI_COUNT;
    let is_seed = app_state.new_game_setup.cursor == FIELD_SEED;
    let is_difficulty = app_state.new_game_setup.cursor == FIELD_DIFFICULTY;
    let diff_label = app_state.new_game_setup.difficulty.label();

    let all_sizes = GalaxySize::all();
    let current_label = app_state.new_game_setup.galaxy_size.label();
    let idx = all_sizes
        .iter()
        .position(|s| *s == app_state.new_game_setup.galaxy_size)
        .unwrap_or(0);
    let has_prev = idx > 0;
    let has_next = idx + 1 < all_sizes.len();
    let galaxy_value = selector_value(current_label, has_prev, has_next);

    let ai_count = app_state.new_game_setup.ai_count;
    let ai_value = selector_value(ai_count, ai_count > 1, ai_count < 4);

    let seed_display = if app_state.new_game_setup.seed_editing {
        let mut s = app_state.new_game_setup.seed_str.clone();
        s.push('_');
        s
    } else {
        app_state.new_game_setup.seed_str.clone()
    };

    let lines = vec![
        field_line("Galaxy Size", &galaxy_value, is_size, inner.width),
        Line::from(""),
        field_line("AI Empires", &ai_value, is_ai, inner.width),
        Line::from(""),
        field_line("Seed", &seed_display, is_seed, inner.width),
        Line::from(""),
        field_line("Difficulty", diff_label, is_difficulty, inner.width),
        Line::from(""),
        Line::from(vec![
            Span::styled("[Enter] ", Theme::title_style()),
            Span::styled(enter_hint(app_state), Theme::muted_style()),
        ]),
    ];
    frame.render_widget(
        Paragraph::new(lines)
            .style(Theme::default_style())
            .wrap(Wrap { trim: false }),
        inner,
    );
}

fn render_preview_section(
    frame: &mut Frame,
    area: Rect,
    app_state: &AppState,
    palette: SplashPalette,
) {
    render_section_block(frame, area, "Campaign Preview", false, palette);
    let inner = Rect::new(
        area.x + 1,
        area.y + 1,
        area.width.saturating_sub(2),
        area.height.saturating_sub(2),
    );
    let star_count = app_state.new_game_setup.galaxy_size.default_star_count();
    let sector_count = app_state.new_game_setup.galaxy_size.default_sector_count();
    let victory = VictorySettings::default_v1();
    let victory_paths = VictoryPath::tie_break_order()
        .iter()
        .filter(|path| victory.is_enabled(**path))
        .map(|path| path.label())
        .collect::<Vec<_>>()
        .join(", ");
    let rivals = format!("{} Empires", app_state.new_game_setup.ai_count);
    let lines = vec![
        Line::from(vec![
            Span::styled("Stars      ", Theme::muted_style()),
            Span::styled(star_count.to_string(), Theme::default_style()),
        ]),
        Line::from(vec![
            Span::styled("Sectors    ", Theme::muted_style()),
            Span::styled(sector_count.to_string(), Theme::default_style()),
        ]),
        Line::from(vec![
            Span::styled("Rivals     ", Theme::muted_style()),
            Span::styled(rivals, Theme::default_style()),
        ]),
        Line::from(vec![
            Span::styled("Victory    ", Theme::muted_style()),
            Span::styled(victory_paths, Theme::title_style()),
        ]),
        Line::from(vec![
            Span::styled("Ruleset    ", Theme::muted_style()),
            Span::styled("Standard", Theme::muted_style()),
        ]),
    ];
    frame.render_widget(
        Paragraph::new(lines)
            .style(Theme::default_style())
            .wrap(Wrap { trim: false }),
        inner,
    );
}

fn render_compact_new_game_setup(frame: &mut Frame, area: Rect, app_state: &AppState) {
    let (def, index) = selected_empire(app_state);
    let _ = index;
    let all_sizes = GalaxySize::all();
    let idx = all_sizes
        .iter()
        .position(|s| *s == app_state.new_game_setup.galaxy_size)
        .unwrap_or(0);
    let current_label = app_state.new_game_setup.galaxy_size.label();
    let galaxy_value = selector_value(current_label, idx > 0, idx + 1 < all_sizes.len());
    let ai_value = selector_value(
        app_state.new_game_setup.ai_count,
        app_state.new_game_setup.ai_count > 1,
        app_state.new_game_setup.ai_count < 4,
    );
    let seed_display = if app_state.new_game_setup.seed_editing {
        format!("{}_", app_state.new_game_setup.seed_str)
    } else {
        app_state.new_game_setup.seed_str.clone()
    };
    let victory = VictorySettings::default_v1();
    let victory_paths = VictoryPath::tie_break_order()
        .iter()
        .filter(|path| victory.is_enabled(**path))
        .map(|path| path.label())
        .collect::<Vec<_>>()
        .join(", ");

    let lines = vec![
        Line::from(vec![
            Span::styled("FARSPACE", Theme::title_style()),
            Span::raw(" — "),
            Span::styled("New Campaign", Theme::muted_style()),
        ]),
        Line::from(vec![Span::styled(
            "CHART • EXPAND • ENDURE",
            Theme::muted_style(),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Empire: ", Theme::muted_style()),
            Span::styled(format!("{} {}", def.symbol, def.name), Theme::title_style()),
        ]),
        Line::from(vec![
            Span::styled("Profile: ", Theme::muted_style()),
            Span::styled(def.short_description, Theme::muted_style()),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Galaxy Size ", Theme::muted_style()),
            Span::styled(galaxy_value, Theme::title_style()),
        ]),
        Line::from(vec![
            Span::styled("AI Empires  ", Theme::muted_style()),
            Span::styled(ai_value, Theme::title_style()),
        ]),
        Line::from(vec![
            Span::styled("Seed        ", Theme::muted_style()),
            Span::styled(seed_display, Theme::title_style()),
        ]),
        Line::from(vec![
            Span::styled("[Enter] ", Theme::title_style()),
            Span::styled(enter_hint(app_state), Theme::muted_style()),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Stars ", Theme::muted_style()),
            Span::styled(
                app_state
                    .new_game_setup
                    .galaxy_size
                    .default_star_count()
                    .to_string(),
                Theme::default_style(),
            ),
            Span::raw("   "),
            Span::styled("Sectors ", Theme::muted_style()),
            Span::styled(
                app_state
                    .new_game_setup
                    .galaxy_size
                    .default_sector_count()
                    .to_string(),
                Theme::default_style(),
            ),
        ]),
        Line::from(vec![
            Span::styled("Victory ", Theme::muted_style()),
            Span::styled(victory_paths, Theme::title_style()),
        ]),
    ];

    let paragraph = Paragraph::new(lines)
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: false })
        .style(Theme::default_style());
    frame.render_widget(paragraph, area);
}

/// Render the new game setup screen.
pub fn render_new_game_setup(frame: &mut Frame, area: Rect, app_state: &AppState) {
    let palette = Theme::splash_palette();
    let [main_area, footer_area] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(3)])
        .areas(area);
    let footer_context = if app_state.new_game_setup.seed_editing {
        Some("Type Seed   Enter Confirm   Esc Cancel")
    } else if app_state.new_game_setup.cursor == FIELD_SEED {
        Some("Enter Edit Seed")
    } else {
        None
    };
    render_footer(frame, footer_area, &Screen::NewGameSetup, footer_context);
    render_wizard_surface(frame, main_area, palette);

    if area.width < COMPACT_MIN_WIDTH
        || area.height < COMPACT_MIN_HEIGHT
        || main_area.width < PREMIUM_MIN_WIDTH
        || main_area.height < PREMIUM_MIN_HEIGHT
    {
        render_compact_new_game_setup(frame, main_area, app_state);
        return;
    }

    // Full-viewport wizard: use the entire main content area instead of a centered fixed card.
    let box_area = main_area;

    let [title_area, content_area] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Fill(1)])
        .areas(box_area);
    render_title_band(frame, title_area, app_state, palette);

    let wide = content_area.width >= WIDE_LAYOUT_WIDTH && content_area.height >= 18;
    if wide {
        let [left, right] = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
            .areas(content_area);
        let [galaxy_area, preview_area] = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(11), Constraint::Fill(1)])
            .areas(right);
        render_empire_section(frame, left, app_state, false, palette);
        render_galaxy_section(frame, galaxy_area, app_state, palette);
        render_preview_section(frame, preview_area, app_state, palette);
    } else {
        let [empire_area, galaxy_area, preview_area] = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(12),
                Constraint::Length(9),
                Constraint::Fill(1),
            ])
            .areas(content_area);
        render_empire_section(frame, empire_area, app_state, false, palette);
        render_galaxy_section(frame, galaxy_area, app_state, palette);
        render_preview_section(frame, preview_area, app_state, palette);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::NewGameSetupState;
    use ratatui::{Terminal, backend::TestBackend, buffer::Buffer};

    fn cell_char(buf: &Buffer, x: u16, y: u16) -> char {
        buf.cell((x, y))
            .and_then(|c| c.symbol().chars().next())
            .unwrap_or(' ')
    }

    fn render_to_buffer(state: &AppState, width: u16, height: u16) -> Buffer {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| render_new_game_setup(frame, frame.area(), state))
            .unwrap();
        terminal.backend().buffer().clone()
    }

    fn render_to_string(state: &AppState, width: u16, height: u16) -> String {
        let buf = render_to_buffer(state, width, height);
        (0..height)
            .flat_map(|y| (0..width).map(move |x| (x, y)))
            .map(|(x, y)| cell_char(&buf, x, y))
            .collect()
    }

    #[test]
    fn new_game_setup_renders_without_panic() {
        let state = AppState::default();
        render_to_string(&state, 100, 40);
    }

    #[test]
    fn setup_screen_shows_empire_name() {
        let state = AppState::default();
        let rendered = render_to_string(&state, 100, 40);
        let defs = game_core::all_empire_definitions();
        assert!(rendered.contains(defs[0].name));
    }

    #[test]
    fn setup_screen_shows_second_empire_when_cursor_advances() {
        let state = AppState {
            new_game_setup: NewGameSetupState {
                empire_cursor: 1,
                ..Default::default()
            },
            ..AppState::default()
        };
        let rendered = render_to_string(&state, 100, 40);
        let defs = game_core::all_empire_definitions();
        assert!(rendered.contains(defs[1].name));
    }

    #[test]
    fn setup_screen_shows_galaxy_size_label() {
        let state = AppState {
            new_game_setup: NewGameSetupState {
                galaxy_size: GalaxySize::Large,
                ..Default::default()
            },
            ..AppState::default()
        };
        let rendered = render_to_string(&state, 100, 40);
        assert!(rendered.contains("Large"));
    }

    #[test]
    fn setup_screen_shows_ai_count() {
        let state = AppState {
            new_game_setup: NewGameSetupState {
                ai_count: 3,
                ..Default::default()
            },
            ..AppState::default()
        };
        let rendered = render_to_string(&state, 100, 40);
        assert!(rendered.contains("AI Empires"));
        assert!(rendered.contains('3'));
    }

    #[test]
    fn setup_screen_shows_seed() {
        let state = AppState {
            new_game_setup: NewGameSetupState {
                seed_str: "12345".to_string(),
                ..Default::default()
            },
            ..AppState::default()
        };
        let rendered = render_to_string(&state, 100, 40);
        assert!(rendered.contains("12345"));
    }

    #[test]
    fn setup_screen_derived_summary_shown() {
        let state = AppState {
            new_game_setup: NewGameSetupState {
                galaxy_size: GalaxySize::Small,
                ..Default::default()
            },
            ..AppState::default()
        };
        let rendered = render_to_string(&state, 100, 40);
        assert!(rendered.contains("10"));
        assert!(rendered.contains("Sectors"));
    }

    #[test]
    fn setup_panel_expands_on_wide_terminals() {
        let state = AppState::default();
        let buf = render_to_buffer(&state, 160, 44);

        let top_border = (0..44u16).find_map(|y| {
            let left =
                (0..160u16).find(|&x| matches!(cell_char(&buf, x, y), '┌' | '╭' | '╒' | '┏'))?;
            let right =
                (0..160u16).rfind(|&x| matches!(cell_char(&buf, x, y), '┐' | '╮' | '╕' | '┓'))?;
            Some((left, right))
        });

        let (left, right) = top_border.expect("setup panel top border not found");
        assert!(right - left + 1 > 60);
    }

    #[test]
    fn active_setup_field_uses_highlight_background() {
        let state = AppState {
            new_game_setup: NewGameSetupState {
                cursor: FIELD_AI_COUNT,
                ..Default::default()
            },
            ..AppState::default()
        };
        let buf = render_to_buffer(&state, 100, 40);

        let ai_row = (0..40u16)
            .find(|&y| {
                let row: String = (0..100u16).map(|x| cell_char(&buf, x, y)).collect();
                row.contains("AI Empires")
            })
            .expect("AI Empires label not found");
        let ai_marker_x = (0..100u16)
            .find(|&x| cell_char(&buf, x, ai_row) == '▶')
            .expect("active field marker not found");
        let ai_cell = buf
            .cell((ai_marker_x, ai_row))
            .expect("active field marker cell not found");

        assert_eq!(ai_cell.bg, Theme::accent());
    }

    #[test]
    fn setup_screen_shows_terran_concord_details() {
        let state = AppState {
            new_game_setup: NewGameSetupState {
                empire_cursor: 6,
                ..Default::default()
            },
            ..AppState::default()
        };
        let rendered = render_to_string(&state, 100, 40);
        assert!(rendered.contains("Terran Concord"));
        assert!(rendered.contains("science"));
        assert!(rendered.contains("exploration"));
    }

    #[test]
    fn setup_screen_shows_terran_dominion_playstyle_summary() {
        let state = AppState {
            new_game_setup: NewGameSetupState {
                empire_cursor: 7,
                ..Default::default()
            },
            ..AppState::default()
        };
        let rendered = render_to_string(&state, 100, 40);
        assert!(rendered.contains("Terran Dominion"));
        assert!(rendered.contains("A hardline Terran hierarchy"));
    }

    #[test]
    fn setup_screen_shows_default_enter_hint_for_start() {
        let state = AppState::default();
        let rendered = render_to_string(&state, 100, 40);
        assert!(rendered.contains("Enter Edit / Start"));
        assert!(rendered.contains("Esc Back"));
    }

    #[test]
    fn setup_screen_shows_field_specific_enter_hint_for_seed() {
        let state = AppState {
            new_game_setup: NewGameSetupState {
                cursor: FIELD_SEED,
                ..Default::default()
            },
            ..AppState::default()
        };
        let rendered = render_to_string(&state, 100, 40);
        assert!(rendered.contains("Enter Edit Seed"));
    }

    #[test]
    fn setup_screen_renders_default_empire_emblem() {
        let state = AppState::default();
        let rendered = render_to_string(&state, 100, 40);
        assert!(rendered.contains('◇'));
    }

    #[test]
    fn setup_screen_changes_emblem_with_cursor() {
        let default_state = AppState::default();
        let other_state = AppState {
            new_game_setup: NewGameSetupState {
                empire_cursor: 1,
                ..Default::default()
            },
            ..AppState::default()
        };
        let first = render_to_string(&default_state, 100, 40);
        let second = render_to_string(&other_state, 100, 40);
        assert!(first.contains('◇'));
        assert!(second.contains('▣'));
    }

    #[test]
    fn setup_screen_renders_at_compact_size() {
        let state = AppState::default();
        render_to_string(&state, 80, 24);
    }

    #[test]
    fn setup_screen_renders_seed_editing_state() {
        let state = AppState {
            new_game_setup: NewGameSetupState {
                seed_editing: true,
                seed_str: "12345".to_string(),
                cursor: FIELD_SEED,
                ..Default::default()
            },
            ..AppState::default()
        };
        render_to_string(&state, 100, 40);
    }
}
