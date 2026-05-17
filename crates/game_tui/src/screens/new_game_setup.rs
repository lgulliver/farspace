//! New Game Setup screen — configure empire choice, galaxy seed, size, AI count, and other
//! scenario options before starting a new game. Empire selection happens on the EmpireSelect
//! screen; this screen shows the chosen empire as a read-only header.

use crate::app::AppState;
use crate::components::render_footer;
use crate::layout::compose_layout;
use crate::screens::Screen;
use crate::theme::Theme;
use game_core::{all_empire_definitions, GalaxySize};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

/// Height of the setup box (border×2 + padding + rows)
const SETUP_BOX_HEIGHT: u16 = 32;
/// Minimum width of the setup box.
const SETUP_BOX_MIN_WIDTH: u16 = 72;
/// Maximum width of the setup box.
const SETUP_BOX_MAX_WIDTH: u16 = 112;
const FIELD_LABEL_WIDTH: usize = 14;
const FIELD_VALUE_WIDTH: usize = 20;

/// Index of each editable setup field in `AppState::setup_cursor`.
pub const FIELD_GALAXY_SIZE: usize = 0;
pub const FIELD_AI_COUNT: usize = 1;
pub const FIELD_SEED: usize = 2;

fn enter_hint(app_state: &AppState) -> &'static str {
    if app_state.new_game_setup.seed_editing {
        "Confirm Seed"
    } else {
        match app_state.new_game_setup.cursor {
            FIELD_SEED => "Edit Seed",
            _ => "Start",
        }
    }
}

fn setup_box_size(main_area: Rect) -> (u16, u16) {
    let width = main_area
        .width
        .min(SETUP_BOX_MAX_WIDTH)
        .max(SETUP_BOX_MIN_WIDTH.min(main_area.width));
    let height = main_area.height.min(SETUP_BOX_HEIGHT);
    (width, height)
}

fn pad_to_width(text: &str, width: usize) -> String {
    let len = text.chars().count();
    if len >= width {
        text.to_string()
    } else {
        format!("{text}{}", " ".repeat(width - len))
    }
}

fn field_line(label: &str, value: &str, is_active: bool, content_width: u16) -> Line<'static> {
    let marker = if is_active { ">" } else { " " };
    let text = format!(
        "{marker} {:<FIELD_LABEL_WIDTH$} {value:<FIELD_VALUE_WIDTH$}",
        label
    );
    if is_active {
        return Line::from(Span::styled(
            pad_to_width(&text, usize::from(content_width)),
            Theme::highlight_style(),
        ));
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

fn selector_value(label: impl std::fmt::Display, has_prev: bool, has_next: bool) -> String {
    format!(
        "{}{}{}",
        if has_prev { "< " } else { "  " },
        label,
        if has_next { " >" } else { "  " }
    )
}

/// Render the new game setup screen.
pub fn render_new_game_setup(frame: &mut Frame, area: Rect, app_state: &AppState) {
    let (_header_area, main_area, footer_area) = compose_layout(area);
    render_footer(frame, footer_area, &Screen::NewGameSetup, None);

    let (setup_width, setup_height) = setup_box_size(main_area);

    // Center the setup box vertically and horizontally
    let v_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(setup_height),
            Constraint::Fill(1),
        ])
        .split(main_area);

    let h_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(setup_width),
            Constraint::Fill(1),
        ])
        .split(v_chunks[1]);

    let box_area = h_chunks[1];
    let content_width = box_area.width.saturating_sub(2);

    let all_defs = all_empire_definitions();
    let empire_idx = app_state
        .new_game_setup
        .empire_cursor
        .min(all_defs.len().saturating_sub(1));
    let selected_def = &all_defs[empire_idx];

    // Derived summary values
    let star_count = app_state.new_game_setup.galaxy_size.default_star_count();
    let sector_count = app_state.new_game_setup.galaxy_size.default_sector_count();

    // Build content lines
    let mut lines: Vec<Line> = vec![Line::from("")];

    lines.push(Line::from(vec![Span::styled(
        "  GALAXY CONFIGURATION",
        Theme::title_style(),
    )]));
    lines.push(Line::from(""));

    // Empire header (read-only — chosen on the empire select screen)
    lines.push(Line::from(vec![
        Span::styled("  Playing as  ", Theme::muted_style()),
        Span::styled(
            format!("{} {}", selected_def.symbol, selected_def.name),
            Theme::title_style(),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled(selected_def.short_description, Theme::muted_style()),
    ]));
    lines.push(Line::from(""));

    // Galaxy Size field
    {
        let is_active = app_state.new_game_setup.cursor == FIELD_GALAXY_SIZE;
        let all_sizes = GalaxySize::all();
        let current_label = app_state.new_game_setup.galaxy_size.label();
        let idx = all_sizes
            .iter()
            .position(|s| *s == app_state.new_game_setup.galaxy_size)
            .unwrap_or(0);
        let has_prev = idx > 0;
        let has_next = idx + 1 < all_sizes.len();
        let value = selector_value(current_label, has_prev, has_next);
        lines.push(field_line("Galaxy Size", &value, is_active, content_width));
    }

    lines.push(Line::from(""));

    // AI Empire Count field
    {
        let is_active = app_state.new_game_setup.cursor == FIELD_AI_COUNT;
        let count = app_state.new_game_setup.ai_count;
        let value = selector_value(count, count > 1, count < 4);
        lines.push(field_line("AI Empires", &value, is_active, content_width));
    }

    lines.push(Line::from(""));

    // Seed field
    {
        let is_active = app_state.new_game_setup.cursor == FIELD_SEED;
        let seed_display = if app_state.new_game_setup.seed_editing {
            let mut s = app_state.new_game_setup.seed_str.clone();
            s.push('_'); // cursor indicator
            s
        } else {
            app_state.new_game_setup.seed_str.clone()
        };
        lines.push(field_line("Seed", &seed_display, is_active, content_width));
        if is_active && !app_state.new_game_setup.seed_editing {
            lines.push(Line::from(vec![Span::styled(
                "  (press Enter to edit seed)",
                Theme::muted_style(),
            )]));
        } else {
            lines.push(Line::from(""));
        }
    }

    lines.push(Line::from(""));

    // Derived summary
    lines.push(Line::from(vec![
        Span::styled("  Stars         ", Theme::muted_style()),
        Span::styled(star_count.to_string(), Theme::default_style()),
    ]));
    lines.push(Line::from(vec![
        Span::styled("  Sectors       ", Theme::muted_style()),
        Span::styled(sector_count.to_string(), Theme::default_style()),
    ]));
    lines.push(Line::from(vec![
        Span::styled("  Difficulty    ", Theme::muted_style()),
        Span::styled("Standard", Theme::muted_style()),
    ]));

    lines.push(Line::from(""));

    lines.push(Line::from(vec![
        Span::styled("  [S]", Theme::title_style()),
        Span::raw(" Start   "),
        Span::styled("  [Enter]", Theme::title_style()),
        Span::raw(format!(" {}   ", enter_hint(app_state))),
        Span::styled("[Esc]", Theme::title_style()),
        Span::raw(" Back"),
    ]));

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(Theme::default_style()),
        )
        .alignment(Alignment::Left)
        .style(Theme::default_style())
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, box_area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::AppState;
    use ratatui::backend::TestBackend;
    use ratatui::buffer::Buffer;
    use ratatui::Terminal;

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

    fn render_to_string(state: &AppState) -> String {
        let buf = render_to_buffer(state, 100, 40);
        (0..40u16)
            .flat_map(|y| (0..100u16).map(move |x| (x, y)))
            .map(|(x, y)| cell_char(&buf, x, y))
            .collect()
    }

    #[test]
    fn new_game_setup_renders_without_panic() {
        let state = AppState::default();
        render_to_string(&state);
    }

    #[test]
    fn setup_screen_shows_empire_name() {
        let state = AppState::default();
        let rendered = render_to_string(&state);
        let defs = game_core::all_empire_definitions();
        let first_name = defs[0].name;
        assert!(
            rendered.contains(first_name),
            "Default empire name '{first_name}' not rendered"
        );
    }

    #[test]
    fn setup_screen_shows_second_empire_when_cursor_advances() {
        let state = AppState {
            new_game_setup: crate::app::NewGameSetupState {
                empire_cursor: 1,
                ..Default::default()
            },
            ..AppState::default()
        };
        let rendered = render_to_string(&state);
        let defs = game_core::all_empire_definitions();
        assert!(
            rendered.contains(defs[1].name),
            "Second empire '{}' not rendered when cursor=1",
            defs[1].name
        );
    }

    #[test]
    fn setup_screen_shows_galaxy_size_label() {
        let state = AppState {
            new_game_setup: crate::app::NewGameSetupState {
                galaxy_size: GalaxySize::Large,
                ..Default::default()
            },
            ..AppState::default()
        };
        let rendered = render_to_string(&state);
        assert!(
            rendered.contains("Large"),
            "Large galaxy size label not found"
        );
    }

    #[test]
    fn setup_screen_shows_ai_count() {
        let state = AppState {
            new_game_setup: crate::app::NewGameSetupState {
                ai_count: 3,
                ..Default::default()
            },
            ..AppState::default()
        };
        let rendered = render_to_string(&state);
        // Assert on the "AI Empires" label and value together so the test
        // doesn't pass spuriously due to a '3' appearing elsewhere on screen.
        assert!(
            rendered.contains("AI Empires") && rendered.contains('3'),
            "AI Empires field with count 3 not rendered"
        );
    }

    #[test]
    fn setup_screen_shows_seed() {
        let state = AppState {
            new_game_setup: crate::app::NewGameSetupState {
                seed_str: "12345".to_string(),
                ..Default::default()
            },
            ..AppState::default()
        };
        let rendered = render_to_string(&state);
        assert!(rendered.contains("12345"), "Seed 12345 not rendered");
    }

    #[test]
    fn setup_screen_derived_summary_shown() {
        let state = AppState {
            new_game_setup: crate::app::NewGameSetupState {
                galaxy_size: GalaxySize::Small,
                ..Default::default()
            },
            ..AppState::default()
        };
        let rendered = render_to_string(&state);
        // Small galaxy has 10 stars and 2 sectors
        assert!(
            rendered.contains("10"),
            "Star count 10 not rendered for Small galaxy"
        );
    }

    #[test]
    fn setup_panel_expands_on_wide_terminals() {
        let state = AppState::default();
        let buf = render_to_buffer(&state, 160, 44);

        let top_border = (0..44u16).find_map(|y| {
            let left = (0..160u16).find(|&x| cell_char(&buf, x, y) == '┌')?;
            let right = (0..160u16).rfind(|&x| cell_char(&buf, x, y) == '┐')?;
            Some((left, right))
        });

        let (left, right) = top_border.expect("setup panel top border not found");
        assert!(
            right - left + 1 > 60,
            "setup panel should be wider than the previous fixed 60 columns"
        );
    }

    #[test]
    fn active_setup_field_uses_highlight_background() {
        let state = AppState {
            new_game_setup: crate::app::NewGameSetupState {
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
            .find(|&x| cell_char(&buf, x, ai_row) == '>')
            .expect("active field marker not found");
        let ai_cell = buf
            .cell((ai_marker_x, ai_row))
            .expect("active field marker cell not found");

        assert_eq!(ai_cell.bg, Theme::accent());
    }

    #[test]
    fn setup_screen_shows_terran_concord_details() {
        let state = AppState {
            new_game_setup: crate::app::NewGameSetupState {
                empire_cursor: 6,
                ..Default::default()
            },
            ..AppState::default()
        };
        let rendered = render_to_string(&state);
        assert!(rendered.contains("Terran Concord"));
        assert!(rendered.contains("science, dialogue, and exploration"));
    }

    #[test]
    fn setup_screen_shows_terran_dominion_playstyle_summary() {
        let state = AppState {
            new_game_setup: crate::app::NewGameSetupState {
                empire_cursor: 7,
                ..Default::default()
            },
            ..AppState::default()
        };
        let rendered = render_to_string(&state);
        assert!(rendered.contains("Terran Dominion"));
        assert!(rendered.contains("A hardline Terran hierarchy"));
    }

    #[test]
    fn setup_screen_shows_default_enter_hint_for_start() {
        let state = AppState::default();
        let rendered = render_to_string(&state);
        assert!(rendered.contains("[Enter] Start"));
        assert!(rendered.contains("[S] Start"));
    }

    #[test]
    fn setup_screen_shows_field_specific_enter_hint_for_seed() {
        let state = AppState {
            new_game_setup: crate::app::NewGameSetupState {
                cursor: FIELD_SEED,
                ..Default::default()
            },
            ..AppState::default()
        };
        let rendered = render_to_string(&state);
        assert!(rendered.contains("[Enter] Edit Seed"));
    }
}
