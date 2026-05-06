//! Galaxy map screen

use crate::components::{render_footer, render_header};
use crate::layout::{compose_layout, split_horizontal};
use crate::screens::Screen;
use crate::theme::Theme;
use crate::AppState;
use game_core::{GameState, StarId};
use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

/// Render the galaxy map screen
pub fn render_galaxy(frame: &mut Frame, area: Rect, app_state: &AppState, game_state: &GameState) {
    let (header_area, main_area, footer_area) = compose_layout(area);

    // Get empire info
    let empire = game_state.empires.get(&game_state.player_empire);
    let (credits, research, empire_name) = match empire {
        Some(e) => (e.credits, e.research_points, e.name.as_str()),
        None => (0, 0, "Unknown"),
    };

    // Render header
    render_header(
        frame,
        header_area,
        game_state.turn,
        empire_name,
        credits,
        research,
    );

    // Split main area: 60% map, 40% details
    let (map_area, details_area) = split_horizontal(main_area, 60);

    // Render star map
    render_star_map(frame, map_area, game_state, app_state.selected_star);

    // Render star details
    render_star_details(frame, details_area, game_state, app_state.selected_star);

    // Render footer
    render_footer(frame, footer_area, &Screen::Galaxy);
}

/// Render the star map
fn render_star_map(
    frame: &mut Frame,
    area: Rect,
    game_state: &GameState,
    selected_star: Option<StarId>,
) {
    let block = Block::default()
        .title(" Galaxy Map ")
        .borders(Borders::ALL)
        .style(Theme::default_style());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Calculate map bounds
    let map_width = inner.width.saturating_sub(2) as i32;
    let map_height = inner.height.saturating_sub(1) as i32;

    if map_width <= 0 || map_height <= 0 {
        return;
    }

    // Scale stars to fit the map area
    // Stars are in range -500..500, map to 0..map_width/height
    for star in game_state.stars.values() {
        let screen_x = ((star.x + 500) * map_width / 1000).clamp(0, map_width - 1);
        let screen_y = ((star.y + 500) * map_height / 1000).clamp(0, map_height - 1);

        let x = inner.x + screen_x as u16;
        let y = inner.y + screen_y as u16;

        // Check bounds
        if x >= inner.x + inner.width || y >= inner.y + inner.height {
            continue;
        }

        let is_selected = selected_star == Some(star.id);
        let style = if is_selected {
            Theme::highlight_style()
        } else {
            Style::default().fg(Theme::star_color(star.spectral_class))
        };

        let char = if is_selected { '@' } else { '*' };

        let star_widget = Paragraph::new(char.to_string()).style(style);
        frame.render_widget(star_widget, Rect::new(x, y, 1, 1));
    }
}

/// Render star details panel
fn render_star_details(
    frame: &mut Frame,
    area: Rect,
    game_state: &GameState,
    selected_star: Option<StarId>,
) {
    let block = Block::default()
        .title(" Star Details ")
        .borders(Borders::ALL)
        .style(Theme::default_style());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let star = match selected_star.and_then(|id| game_state.stars.get(&id)) {
        Some(s) => s,
        None => {
            let no_selection = Paragraph::new("No star selected").style(Theme::muted_style());
            frame.render_widget(no_selection, inner);
            return;
        }
    };

    let mut lines = vec![
        Line::from(vec![Span::styled(&star.name, Theme::title_style())]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Class: ", Theme::muted_style()),
            Span::styled(
                format!("{}", star.spectral_class.as_char()),
                Style::default().fg(Theme::star_color(star.spectral_class)),
            ),
        ]),
        Line::from(vec![
            Span::styled("Position: ", Theme::muted_style()),
            Span::raw(format!("({}, {})", star.x, star.y)),
        ]),
        Line::from(""),
        Line::from(Span::styled("Planets:", Theme::title_style())),
    ];

    for planet in &star.planets {
        let colony_info = match &planet.colony {
            Some(colony_id) => {
                if let Some(colony) = game_state.colonies.get(colony_id) {
                    format!(" [Colony - Pop: {}]", colony.population)
                } else {
                    String::new()
                }
            }
            None => String::new(),
        };

        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(&planet.name, Theme::default_style()),
            Span::styled(format!(" ({:?})", planet.size), Theme::muted_style()),
            Span::styled(colony_info, Theme::accent_style()),
        ]));
    }

    let paragraph = Paragraph::new(lines).style(Theme::default_style());
    frame.render_widget(paragraph, inner);
}

#[cfg(test)]
mod tests {
    use super::*;
    use game_core::Engine;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    #[test]
    fn galaxy_screen_renders_without_panic() {
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();

        let engine = Engine::new(42);
        let app_state = AppState::default();

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_galaxy(frame, area, &app_state, &engine.state);
            })
            .unwrap();
    }

    #[test]
    fn galaxy_screen_with_selection() {
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();

        let engine = Engine::new(42);
        let app_state = AppState {
            selected_star: engine.state.stars.keys().next().copied(),
            ..Default::default()
        };

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_galaxy(frame, area, &app_state, &engine.state);
            })
            .unwrap();
    }
}
