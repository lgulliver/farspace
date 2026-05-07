//! Sector overview screen - shows all sectors in the galaxy

use std::borrow::Cow;

use crate::components::{render_footer, render_header, render_log};
use crate::layout::{compose_layout, split_horizontal};
use crate::screens::Screen;
use crate::theme::Theme;
use crate::AppState;
use game_core::{GameState, SectorId};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub fn render_sector_overview(
    frame: &mut Frame,
    area: Rect,
    app_state: &AppState,
    game_state: &GameState,
) {
    let (header_area, main_area, footer_area) = compose_layout(area);

    let empire = game_state.empires.get(&game_state.player_empire);
    let (credits, food, research, empire_name) = match empire {
        Some(e) => (e.credits, e.food, e.research_points, e.name.as_str()),
        None => (0, 0, 0, "Unknown"),
    };

    render_header(
        frame,
        header_area,
        game_state.turn,
        empire_name,
        credits,
        food,
        research,
    );

    let (map_area, right_area) = split_horizontal(main_area, 55);

    render_sector_map(frame, map_area, game_state, app_state.selected_sector);

    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(right_area);

    render_sector_details(
        frame,
        right_chunks[0],
        game_state,
        app_state.selected_sector,
    );
    render_log(frame, right_chunks[1], &app_state.log);

    render_footer(frame, footer_area, &Screen::SectorOverview);
}

fn render_sector_map(
    frame: &mut Frame,
    area: Rect,
    game_state: &GameState,
    selected_sector: Option<SectorId>,
) {
    let block = Block::default()
        .title(" Galaxy — Sector Overview ")
        .title_style(Theme::title_style())
        .borders(Borders::ALL)
        .border_style(Theme::focused_border_style())
        .style(Theme::default_style());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let map_height = inner.height.saturating_sub(2) as i32;
    let map_width = inner.width.saturating_sub(2) as i32;

    if map_width <= 0 || map_height <= 0 {
        return;
    }

    for sector in game_state.sectors.values() {
        let screen_x = ((sector.x + 500) * map_width / 1000).clamp(0, map_width - 1);
        let screen_y = ((sector.y + 500) * map_height / 1000).clamp(0, map_height - 1);

        let x = inner.x + screen_x as u16;
        let y = inner.y + screen_y as u16;

        if x >= inner.x + inner.width || y >= inner.y + inner.height {
            continue;
        }

        let is_selected = selected_sector == Some(sector.id);

        let count = game_state
            .stars
            .values()
            .filter(|s| s.sector == sector.id)
            .count();

        let (render_char, style) = if is_selected {
            ('@', Theme::highlight_style())
        } else {
            ('*', Theme::default_style())
        };

        let label = format!("{}{}", render_char, count);

        let sector_widget = Paragraph::new(label).style(style);
        frame.render_widget(sector_widget, Rect::new(x, y, 2, 1));
    }

    if inner.height >= 2 && inner.width >= 10 {
        render_map_legend(
            frame,
            Rect::new(inner.x, inner.y + inner.height - 1, inner.width, 1),
        );
    }
}

fn render_map_legend(frame: &mut Frame, area: Rect) {
    let spans = vec![
        Span::styled("@", Theme::highlight_style()),
        Span::styled(" Sel  ", Theme::dim_border_style()),
        Span::styled("*n", Theme::default_style()),
        Span::styled(" Sector (n stars)", Theme::dim_border_style()),
    ];

    let legend = Paragraph::new(Line::from(spans)).style(Theme::muted_style());
    frame.render_widget(legend, area);
}

fn render_sector_details(
    frame: &mut Frame,
    area: Rect,
    game_state: &GameState,
    selected_sector: Option<SectorId>,
) {
    let border_style = Theme::dim_border_style();

    let block = Block::default()
        .title(" Sector Details ")
        .borders(Borders::ALL)
        .border_style(border_style)
        .style(Theme::default_style());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let sector = match selected_sector.and_then(|id| game_state.sectors.get(&id)) {
        Some(s) => s,
        None => {
            let no_selection = Paragraph::new("No sector selected").style(Theme::muted_style());
            frame.render_widget(no_selection, inner);
            return;
        }
    };

    let mut lines = vec![
        Line::from(vec![Span::styled(&sector.name, Theme::title_style())]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Systems: ", Theme::muted_style()),
            Span::raw(count_systems_in_sector(game_state, sector.id).to_string()),
        ]),
        Line::from(vec![
            Span::styled("Position: ", Theme::muted_style()),
            Span::raw(format!("({}, {})", sector.x, sector.y)),
        ]),
        Line::from(""),
        Line::from(Span::styled("Systems in Sector:", Theme::title_style())),
    ];

    let stars_in_sector: Vec<_> = game_state
        .stars
        .values()
        .filter(|s| s.sector == sector.id)
        .collect();

    for star in stars_in_sector {
        let is_explored = game_state.explored_stars.contains(&star.id);
        let name: Cow<'_, str> = if is_explored {
            Cow::Borrowed(star.name.as_str())
        } else {
            Cow::Owned("???".to_string())
        };
        let style = if is_explored {
            Theme::default_style()
        } else {
            Theme::muted_style()
        };
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(format!("{}", star.spectral_class.as_char()), style),
            Span::raw(" "),
            Span::styled(name, style),
        ]));
    }

    frame.render_widget(Paragraph::new(lines).style(Theme::default_style()), inner);
}

fn count_systems_in_sector(game_state: &GameState, sector_id: SectorId) -> usize {
    game_state
        .stars
        .values()
        .filter(|s| s.sector == sector_id)
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use game_core::Engine;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    #[test]
    fn sector_overview_renders_without_panic() {
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();

        let engine = Engine::new(42);
        let app_state = AppState::default();

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_sector_overview(frame, area, &app_state, &engine.state);
            })
            .unwrap();
    }

    #[test]
    fn sector_overview_with_selection() {
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();

        let engine = Engine::new(42);
        let first_sector = engine.state.sectors.keys().next().copied();
        let app_state = AppState {
            selected_sector: first_sector,
            ..Default::default()
        };

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_sector_overview(frame, area, &app_state, &engine.state);
            })
            .unwrap();
    }

    #[test]
    fn sector_overview_small_terminal_does_not_panic() {
        let backend = TestBackend::new(40, 15);
        let mut terminal = Terminal::new(backend).unwrap();

        let engine = Engine::new(42);
        let app_state = AppState::default();

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_sector_overview(frame, area, &app_state, &engine.state);
            })
            .unwrap();
    }
}
