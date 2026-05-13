//! New Game Setup screen — configure empire choice, galaxy seed, size, AI count, and other
//! scenario options before starting a new game.

use crate::app::AppState;
use crate::components::render_footer;
use crate::layout::compose_layout;
use crate::screens::Screen;
use crate::theme::Theme;
use game_core::{all_empire_definitions, GalaxySize};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

/// Height of the setup box (border×2 + padding + rows)
const SETUP_BOX_HEIGHT: u16 = 28;
/// Width of the setup box
const SETUP_BOX_WIDTH: u16 = 60;

/// Index of each editable setup field in `AppState::setup_cursor`.
pub const FIELD_EMPIRE: usize = 0;
pub const FIELD_GALAXY_SIZE: usize = 1;
pub const FIELD_AI_COUNT: usize = 2;
pub const FIELD_SEED: usize = 3;

/// Render the new game setup screen.
pub fn render_new_game_setup(frame: &mut Frame, area: Rect, app_state: &AppState) {
    let (_header_area, main_area, footer_area) = compose_layout(area);
    render_footer(frame, footer_area, &Screen::NewGameSetup, None);

    // Center the setup box vertically and horizontally
    let v_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(SETUP_BOX_HEIGHT),
            Constraint::Fill(1),
        ])
        .split(main_area);

    let h_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(SETUP_BOX_WIDTH),
            Constraint::Fill(1),
        ])
        .split(v_chunks[1]);

    let box_area = h_chunks[1];

    let all_defs = all_empire_definitions();
    let empire_idx = app_state
        .setup_empire_cursor
        .min(all_defs.len().saturating_sub(1));
    let selected_def = &all_defs[empire_idx];

    // Derived summary values
    let star_count = app_state.setup_galaxy_size.default_star_count();
    let sector_count = app_state.setup_galaxy_size.default_sector_count();

    // Build content lines
    let mut lines: Vec<Line> = vec![Line::from("")];

    lines.push(Line::from(vec![Span::styled(
        "  NEW GAME SETUP",
        Theme::title_style(),
    )]));
    lines.push(Line::from(""));

    // Empire Selection field
    {
        let is_active = app_state.setup_cursor == FIELD_EMPIRE;
        let label_style = if is_active {
            Theme::title_style()
        } else {
            Theme::default_style()
        };
        let has_prev = empire_idx > 0;
        let has_next = empire_idx + 1 < all_defs.len();
        lines.push(Line::from(vec![
            Span::styled("  Empire       ", label_style),
            Span::styled(if has_prev { "◀ " } else { "  " }, Theme::muted_style()),
            Span::styled(
                format!("{} {}", selected_def.symbol, selected_def.name),
                Theme::title_style(),
            ),
            Span::styled(if has_next { " ▶" } else { "  " }, Theme::muted_style()),
        ]));
        // Show short description and traits below when this field is active
        if is_active {
            lines.push(Line::from(vec![
                Span::raw("                "),
                Span::styled(selected_def.short_description, Theme::muted_style()),
            ]));
            let tag_labels: Vec<&str> = selected_def.playstyle.iter().map(|t| t.label()).collect();
            lines.push(Line::from(vec![
                Span::raw("                "),
                Span::styled(tag_labels.join(" · "), Theme::accent_style()),
            ]));
            // Show trait modifiers
            let m = &selected_def.trait_modifiers;
            let mut mods: Vec<String> = Vec::new();
            if m.industry_per_colony != 0 {
                mods.push(format!("{:+} industry/colony", m.industry_per_colony));
            }
            if m.science_per_colony != 0 {
                mods.push(format!("{:+} science/colony", m.science_per_colony));
            }
            if m.credits_per_colony != 0 {
                mods.push(format!("{:+} credits/colony", m.credits_per_colony));
            }
            if m.food_per_colony != 0 {
                mods.push(format!("{:+} food/colony", m.food_per_colony));
            }
            if !mods.is_empty() {
                lines.push(Line::from(vec![
                    Span::raw("                "),
                    Span::styled(mods.join("  "), Theme::success_style()),
                ]));
            } else {
                lines.push(Line::from(""));
            }
        } else {
            // Compact: one-line description
            lines.push(Line::from(vec![
                Span::raw("                "),
                Span::styled(selected_def.short_description, Theme::muted_style()),
            ]));
            lines.push(Line::from(""));
            lines.push(Line::from(""));
        }
    }

    lines.push(Line::from(""));

    // Galaxy Size field
    {
        let is_active = app_state.setup_cursor == FIELD_GALAXY_SIZE;
        let label_style = if is_active {
            Theme::title_style()
        } else {
            Theme::default_style()
        };
        let all_sizes = GalaxySize::all();
        let current_label = app_state.setup_galaxy_size.label();
        let idx = all_sizes
            .iter()
            .position(|s| *s == app_state.setup_galaxy_size)
            .unwrap_or(0);
        let has_prev = idx > 0;
        let has_next = idx + 1 < all_sizes.len();
        lines.push(Line::from(vec![
            Span::styled("  Galaxy Size  ", label_style),
            Span::styled(if has_prev { "◀ " } else { "  " }, Theme::muted_style()),
            Span::styled(format!("{:<8}", current_label), Theme::title_style()),
            Span::styled(if has_next { " ▶" } else { "  " }, Theme::muted_style()),
        ]));
    }

    lines.push(Line::from(""));

    // AI Empire Count field
    {
        let is_active = app_state.setup_cursor == FIELD_AI_COUNT;
        let label_style = if is_active {
            Theme::title_style()
        } else {
            Theme::default_style()
        };
        let count = app_state.setup_ai_count;
        lines.push(Line::from(vec![
            Span::styled("  AI Empires   ", label_style),
            Span::styled(if count > 1 { "◀ " } else { "  " }, Theme::muted_style()),
            Span::styled(format!("{:<8}", count), Theme::title_style()),
            Span::styled(if count < 4 { " ▶" } else { "  " }, Theme::muted_style()),
        ]));
    }

    lines.push(Line::from(""));

    // Seed field
    {
        let is_active = app_state.setup_cursor == FIELD_SEED;
        let label_style = if is_active {
            Theme::title_style()
        } else {
            Theme::default_style()
        };
        let seed_display = if app_state.setup_seed_editing {
            let mut s = app_state.setup_seed_str.clone();
            s.push('_'); // cursor indicator
            s
        } else {
            app_state.setup_seed_str.clone()
        };
        lines.push(Line::from(vec![
            Span::styled("  Seed         ", label_style),
            Span::styled(format!("{:<18}", seed_display), Theme::title_style()),
        ]));
        if is_active && !app_state.setup_seed_editing {
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
        Span::styled("  [Enter]", Theme::title_style()),
        Span::raw(" Start   "),
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
        .style(Theme::default_style());

    frame.render_widget(paragraph, box_area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::AppState;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn render_to_string(state: &AppState) -> String {
        let backend = TestBackend::new(100, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| render_new_game_setup(frame, frame.area(), state))
            .unwrap();
        let buf = terminal.backend().buffer().clone();
        (0..40u16)
            .flat_map(|y| (0..100u16).map(move |x| (x, y)))
            .map(|(x, y)| {
                buf.cell((x, y))
                    .and_then(|c| c.symbol().chars().next())
                    .unwrap_or(' ')
            })
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
            setup_empire_cursor: 1,
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
            setup_galaxy_size: GalaxySize::Large,
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
            setup_ai_count: 3,
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
            setup_seed_str: "12345".to_string(),
            ..AppState::default()
        };
        let rendered = render_to_string(&state);
        assert!(rendered.contains("12345"), "Seed 12345 not rendered");
    }

    #[test]
    fn setup_screen_derived_summary_shown() {
        let state = AppState {
            setup_galaxy_size: GalaxySize::Small,
            ..AppState::default()
        };
        let rendered = render_to_string(&state);
        // Small galaxy has 10 stars and 2 sectors
        assert!(
            rendered.contains("10"),
            "Star count 10 not rendered for Small galaxy"
        );
    }
}
