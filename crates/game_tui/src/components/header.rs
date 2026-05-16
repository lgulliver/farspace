//! Header component

use crate::theme::Theme;
use game_core::{tech_by_id, GameState};
use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

fn join_segments(segments: &[String]) -> String {
    segments.join(" │ ")
}

/// Snapshot of top-bar values for the player empire.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeaderData {
    pub turn: u32,
    pub empire_name: String,
    pub credits: i64,
    pub food: i64,
    pub science: i64,
    pub active_research: String,
    pub colonies: usize,
    pub fleets: usize,
}

/// Build `HeaderData` from the current game state.
pub fn derive_header_data(game_state: &GameState) -> HeaderData {
    let empire = game_state.empires.get(&game_state.player_empire);
    let active_research = empire
        .and_then(|e| e.research.current_tech)
        .and_then(|tech_id| tech_by_id(tech_id).map(|tech| tech.name.to_string()))
        .unwrap_or_else(|| "None".to_string());
    let empire_name = empire
        .map(|e| e.name.clone())
        .unwrap_or_else(|| "Unknown".to_string());
    let credits = empire.map(|e| e.credits).unwrap_or(0);
    let food = empire.map(|e| e.food).unwrap_or(0);
    let science = empire.map(|e| e.research_points).unwrap_or(0);
    let colonies = game_state
        .colonies
        .values()
        .filter(|c| c.owner == game_state.player_empire)
        .count();
    let fleets = game_state
        .fleets
        .values()
        .filter(|f| f.owner == game_state.player_empire)
        .count();

    HeaderData {
        turn: game_state.turn,
        empire_name,
        credits,
        food,
        science,
        active_research,
        colonies,
        fleets,
    }
}

/// Render the header bar showing turn, empire name, and economy summary.
pub fn render_header(frame: &mut Frame, area: Rect, data: &HeaderData) {
    let credits_style = if data.credits < 0 {
        Theme::warning_style()
    } else {
        Theme::default_style()
    };
    let food_style = if data.food < 0 {
        Theme::warning_style()
    } else {
        Theme::default_style()
    };

    let wide_segments = vec![
        format!("Turn {}", data.turn),
        data.empire_name.clone(),
        format!("Credits: {}", data.credits),
        format!("Food: {}", data.food),
        format!("Science: {}", data.science),
        format!("Research: {}", data.active_research),
        format!("Colonies: {}", data.colonies),
        format!("Fleets: {}", data.fleets),
    ];
    let medium_segments = vec![
        format!("T{}", data.turn),
        data.empire_name.clone(),
        format!("Cr {}", data.credits),
        format!("Fd {}", data.food),
        format!("Sci {}", data.science),
        format!("Res {}", data.active_research),
        format!("Col {}", data.colonies),
        format!("Fl {}", data.fleets),
    ];
    let narrow_segments = vec![
        format!("T{}", data.turn),
        format!("Cr {}", data.credits),
        format!("Fd {}", data.food),
        format!("Sci {}", data.science),
        format!("Col {}", data.colonies),
        format!("Fl {}", data.fleets),
    ];

    let area_width = usize::from(area.width);
    let chosen = if join_segments(&wide_segments).chars().count() <= area_width {
        wide_segments
    } else if join_segments(&medium_segments).chars().count() <= area_width {
        medium_segments
    } else {
        narrow_segments
    };

    let mut spans = Vec::new();
    for (index, segment) in chosen.iter().enumerate() {
        if index > 0 {
            spans.push(Span::styled(" │ ", Theme::dim_border_style()));
        }

        let span = if index == 0 {
            Span::styled(format!(" {} ", segment), Theme::header_style())
        } else if segment.starts_with(&data.empire_name) {
            Span::styled(segment.clone(), Theme::title_style())
        } else if segment.starts_with("Credits:") || segment.starts_with("Cr ") {
            let label = if segment.starts_with("Credits:") {
                "Credits: "
            } else {
                "Cr "
            };
            spans.push(Span::styled(label, Theme::muted_style()));
            Span::styled(segment[label.len()..].to_string(), credits_style)
        } else if segment.starts_with("Food:") || segment.starts_with("Fd ") {
            let label = if segment.starts_with("Food:") {
                "Food: "
            } else {
                "Fd "
            };
            spans.push(Span::styled(label, Theme::muted_style()));
            Span::styled(segment[label.len()..].to_string(), food_style)
        } else if let Some(value) = segment.strip_prefix("Science: ") {
            spans.push(Span::styled("Science: ", Theme::muted_style()));
            Span::raw(value.to_string())
        } else if let Some(value) = segment.strip_prefix("Sci ") {
            spans.push(Span::styled("Sci ", Theme::muted_style()));
            Span::raw(value.to_string())
        } else if let Some(value) = segment.strip_prefix("Research: ") {
            spans.push(Span::styled("Research: ", Theme::muted_style()));
            Span::raw(value.to_string())
        } else if let Some(value) = segment.strip_prefix("Res ") {
            spans.push(Span::styled("Res ", Theme::muted_style()));
            Span::raw(value.to_string())
        } else if let Some(value) = segment.strip_prefix("Colonies: ") {
            spans.push(Span::styled("Colonies: ", Theme::muted_style()));
            Span::raw(value.to_string())
        } else if let Some(value) = segment.strip_prefix("Col ") {
            spans.push(Span::styled("Col ", Theme::muted_style()));
            Span::raw(value.to_string())
        } else if let Some(value) = segment.strip_prefix("Fleets: ") {
            spans.push(Span::styled("Fleets: ", Theme::muted_style()));
            Span::raw(value.to_string())
        } else if let Some(value) = segment.strip_prefix("Fl ") {
            spans.push(Span::styled("Fl ", Theme::muted_style()));
            Span::raw(value.to_string())
        } else {
            Span::raw(segment.clone())
        };
        spans.push(span);
    }

    let paragraph = Paragraph::new(Line::from(spans)).style(Theme::default_style());

    frame.render_widget(paragraph, area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    #[test]
    fn render_header_no_panic() {
        let backend = TestBackend::new(80, 1);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| {
                let area = frame.area();
                let data = HeaderData {
                    turn: 5,
                    empire_name: "Test Empire".to_string(),
                    credits: 1000,
                    food: 50,
                    science: 500,
                    active_research: "Survey Drones".to_string(),
                    colonies: 1,
                    fleets: 2,
                };
                render_header(frame, area, &data);
            })
            .unwrap();
    }

    #[test]
    fn render_header_negative_food_no_panic() {
        let backend = TestBackend::new(80, 1);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| {
                let area = frame.area();
                let data = HeaderData {
                    turn: 3,
                    empire_name: "Test Empire".to_string(),
                    credits: 20,
                    food: -5,
                    science: 10,
                    active_research: "None".to_string(),
                    colonies: 1,
                    fleets: 1,
                };
                render_header(frame, area, &data);
            })
            .unwrap();
    }

    #[test]
    fn render_header_negative_credits_no_panic() {
        let backend = TestBackend::new(80, 1);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| {
                let area = frame.area();
                let data = HeaderData {
                    turn: 7,
                    empire_name: "Test Empire".to_string(),
                    credits: -100,
                    food: 3,
                    science: 200,
                    active_research: "Hyperlane Theory".to_string(),
                    colonies: 2,
                    fleets: 3,
                };
                render_header(frame, area, &data);
            })
            .unwrap();
    }

    #[test]
    fn render_header_both_deficits_no_panic() {
        let backend = TestBackend::new(80, 1);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| {
                let area = frame.area();
                let data = HeaderData {
                    turn: 10,
                    empire_name: "Test Empire".to_string(),
                    credits: -50,
                    food: -3,
                    science: 0,
                    active_research: "None".to_string(),
                    colonies: 1,
                    fleets: 0,
                };
                render_header(frame, area, &data);
            })
            .unwrap();
    }

    #[test]
    fn derive_header_data_no_panic() {
        let state = game_core::Engine::new(42).state;
        let _ = derive_header_data(&state);
    }

    #[test]
    fn derive_header_data_counts_player_assets() {
        let state = game_core::Engine::new(42).state;
        let data = derive_header_data(&state);
        assert_eq!(data.turn, state.turn);
        assert_eq!(
            data.colonies,
            state
                .colonies
                .values()
                .filter(|c| c.owner == state.player_empire)
                .count()
        );
        assert_eq!(
            data.fleets,
            state
                .fleets
                .values()
                .filter(|f| f.owner == state.player_empire)
                .count()
        );
    }

    #[test]
    fn derive_header_data_active_research_defaults_to_none() {
        let mut state = game_core::Engine::new(42).state;
        if let Some(empire) = state.empires.get_mut(&state.player_empire) {
            empire.research.current_tech = None;
        }
        let data = derive_header_data(&state);
        assert_eq!(data.active_research, "None");
    }

    #[test]
    fn derive_header_data_unknown_empire_fallback() {
        let mut state = game_core::Engine::new(42).state;
        let player = state.player_empire;
        state.empires.remove(&player);
        let data = derive_header_data(&state);
        assert_eq!(data.empire_name, "Unknown");
        assert_eq!(data.credits, 0);
        assert_eq!(data.food, 0);
        assert_eq!(data.science, 0);
        assert_eq!(data.active_research, "None");
    }

    #[test]
    fn render_header_with_long_research_name_no_panic() {
        let backend = TestBackend::new(160, 1);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                let area = frame.area();
                let data = HeaderData {
                    turn: 12,
                    empire_name: "Axiom Collective".to_string(),
                    credits: 250,
                    food: 18,
                    science: 72,
                    active_research: "Interstellar Infrastructure Optimization".to_string(),
                    colonies: 6,
                    fleets: 9,
                };
                render_header(frame, area, &data);
            })
            .unwrap();
    }
}
