//! Sector map screen - shows systems within a selected sector

use std::borrow::Cow;

use crate::components::{render_footer, render_header, render_log};
use crate::layout::{compose_layout, split_horizontal};
use crate::screens::Screen;
use crate::theme::Theme;
use crate::AppState;
use game_core::{GameState, StarId};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub fn render_sector_map(
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

    render_local_map(frame, map_area, game_state, app_state);

    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(right_area);

    render_system_list(frame, right_chunks[0], game_state, app_state);
    render_log(frame, right_chunks[1], &app_state.log);

    render_footer(frame, footer_area, &Screen::SectorMap);
}

fn render_local_map(frame: &mut Frame, area: Rect, game_state: &GameState, app_state: &AppState) {
    let sector_name = app_state
        .selected_sector
        .and_then(|id| game_state.sectors.get(&id))
        .map(|s| s.name.as_str())
        .unwrap_or("Unknown");

    let title = format!(" {} — Systems ", sector_name);

    let block = Block::default()
        .title(title)
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

    let sector_id = match app_state.selected_sector {
        Some(id) => id,
        None => return,
    };

    // Verify sector exists
    if !game_state.sectors.contains_key(&sector_id) {
        return;
    }

    let stars_in_sector: Vec<_> = game_state
        .stars
        .values()
        .filter(|s| s.sector == sector_id)
        .collect();

    if stars_in_sector.is_empty() {
        return;
    }

    let min_x = stars_in_sector.iter().map(|s| s.x).min().unwrap_or(-100);
    let max_x = stars_in_sector.iter().map(|s| s.x).max().unwrap_or(100);
    let min_y = stars_in_sector.iter().map(|s| s.y).min().unwrap_or(-100);
    let max_y = stars_in_sector.iter().map(|s| s.y).max().unwrap_or(100);

    let range_x = (max_x - min_x).max(1);
    let range_y = (max_y - min_y).max(1);

    let scout_destinations: std::collections::BTreeSet<StarId> = game_state
        .scout_missions
        .values()
        .map(|m| m.destination)
        .collect();

    let fleet_destinations: std::collections::BTreeSet<StarId> = game_state
        .fleet_missions
        .values()
        .map(|m| m.destination)
        .collect();

    for star in &stars_in_sector {
        let rel_x = star.x - min_x;
        let rel_y = star.y - min_y;

        let screen_x = ((rel_x * map_width) / range_x).clamp(0, map_width - 1);
        let screen_y = ((rel_y * map_height) / range_y).clamp(0, map_height - 1);

        let x = inner.x + screen_x as u16;
        let y = inner.y + screen_y as u16;

        if x >= inner.x + inner.width || y >= inner.y + inner.height {
            continue;
        }

        let is_selected = app_state.selected_star == Some(star.id);
        let is_explored = game_state.explored_stars.contains(&star.id);
        let scout_en_route = scout_destinations.contains(&star.id);
        let fleet_en_route = fleet_destinations.contains(&star.id);

        let (render_char, style) = if is_selected {
            ('@', Theme::highlight_style())
        } else if scout_en_route {
            (
                '+',
                Style::default()
                    .fg(ratatui::style::Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
        } else if fleet_en_route {
            (
                '~',
                Style::default()
                    .fg(ratatui::style::Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
        } else if is_explored {
            (
                '*',
                Style::default().fg(Theme::star_color(star.spectral_class)),
            )
        } else {
            ('?', Theme::muted_style())
        };

        let star_widget = Paragraph::new(render_char.to_string()).style(style);
        frame.render_widget(star_widget, Rect::new(x, y, 1, 1));
    }

    if inner.height >= 2 && inner.width >= 10 {
        render_local_legend(
            frame,
            Rect::new(inner.x, inner.y + inner.height - 1, inner.width, 1),
        );
    }
}

fn render_local_legend(frame: &mut Frame, area: Rect) {
    let spans = vec![
        Span::styled("@", Theme::highlight_style()),
        Span::styled(" Sel  ", Theme::dim_border_style()),
        Span::styled("*", Theme::default_style()),
        Span::styled(" Explored  ", Theme::dim_border_style()),
        Span::styled("?", Theme::muted_style()),
        Span::styled(" Unknown  ", Theme::dim_border_style()),
        Span::styled("+", Style::default().fg(ratatui::style::Color::Yellow)),
        Span::styled(" Scout  ", Theme::dim_border_style()),
        Span::styled("~", Style::default().fg(ratatui::style::Color::Cyan)),
        Span::styled(" Fleet", Theme::dim_border_style()),
    ];

    let legend = Paragraph::new(Line::from(spans)).style(Theme::muted_style());
    frame.render_widget(legend, area);
}

fn render_system_list(frame: &mut Frame, area: Rect, game_state: &GameState, app_state: &AppState) {
    let border_style = Theme::dim_border_style();

    let block = Block::default()
        .title(" Systems ")
        .borders(Borders::ALL)
        .border_style(border_style)
        .style(Theme::default_style());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let sector_id = match app_state.selected_sector {
        Some(id) => id,
        None => {
            let no_selection = Paragraph::new("No sector selected").style(Theme::muted_style());
            frame.render_widget(no_selection, inner);
            return;
        }
    };

    let stars_in_sector: Vec<_> = game_state
        .stars
        .values()
        .filter(|s| s.sector == sector_id)
        .collect();

    if stars_in_sector.is_empty() {
        frame.render_widget(
            Paragraph::new("No systems in this sector").style(Theme::muted_style()),
            inner,
        );
        return;
    }

    let mut lines = Vec::new();
    lines.push(Line::from(vec![Span::styled(
        "Sector Systems:",
        Theme::title_style(),
    )]));
    lines.push(Line::from(""));

    for star in &stars_in_sector {
        let is_selected = app_state.selected_star == Some(star.id);
        let is_explored = game_state.explored_stars.contains(&star.id);

        let prefix = if is_selected { "▶" } else { " " };

        let name: Cow<'_, str> = if is_explored {
            Cow::Borrowed(star.name.as_str())
        } else {
            Cow::Owned("???".to_string())
        };

        let style = if is_selected {
            Theme::highlight_style()
        } else if is_explored {
            Theme::default_style()
        } else {
            Theme::muted_style()
        };

        let spectral_char = if is_explored {
            format!("{}", star.spectral_class.as_char())
        } else {
            "?".to_string()
        };

        lines.push(Line::from(vec![
            Span::raw(format!("{} ", prefix)),
            Span::styled(spectral_char, style),
            Span::raw(" "),
            Span::styled(name, style),
        ]));
    }

    frame.render_widget(Paragraph::new(lines).style(Theme::default_style()), inner);
}

#[cfg(test)]
mod tests {
    use super::*;
    use game_core::Engine;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn create_app_with_sector() -> (AppState, GameState) {
        let engine = Engine::new(42);
        let first_sector = engine.state.sectors.keys().next().copied();
        let first_star_in_sector = engine
            .state
            .stars
            .values()
            .find(|s| Some(s.sector) == first_sector)
            .map(|s| s.id);

        let app_state = AppState {
            selected_sector: first_sector,
            selected_star: first_star_in_sector,
            ..Default::default()
        };

        (app_state, engine.state)
    }

    #[test]
    fn sector_map_renders_without_panic() {
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();

        let (app_state, game_state) = create_app_with_sector();

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_sector_map(frame, area, &app_state, &game_state);
            })
            .unwrap();
    }

    #[test]
    fn sector_map_small_terminal_does_not_panic() {
        let backend = TestBackend::new(40, 15);
        let mut terminal = Terminal::new(backend).unwrap();

        let (app_state, game_state) = create_app_with_sector();

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_sector_map(frame, area, &app_state, &game_state);
            })
            .unwrap();
    }

    #[test]
    fn sector_map_no_selection_renders() {
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();

        let engine = Engine::new(42);
        let app_state = AppState {
            selected_sector: None,
            ..Default::default()
        };

        terminal
            .draw(|frame| {
                let area = frame.area();
                render_sector_map(frame, area, &app_state, &engine.state);
            })
            .unwrap();
    }
}
