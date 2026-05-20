//! Screen types and rendering

pub mod colony;
pub mod diplomacy;
pub mod empire_overview;
pub mod empire_select;
pub mod menu;
pub mod new_game_setup;
pub mod research;
pub mod sector_map;
pub mod sector_overview;
pub mod settings;
pub mod ship_designer;
pub mod system;

use ratatui::{
    layout::{Alignment, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::theme::Theme;
use crate::AppState;
use game_core::GameState;

/// Active screen in the application
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Screen {
    #[default]
    Menu,
    EmpireSelect,
    NewGameSetup,
    SectorOverview,
    SectorMap,
    System,
    Colony,
    EmpireOverview,
    Research,
    Diplomacy,
    ShipDesigner,
    Settings,
}

impl Screen {
    /// Render this screen
    pub fn render(
        &self,
        frame: &mut Frame,
        area: Rect,
        app_state: &AppState,
        game_state: Option<&GameState>,
    ) {
        match self {
            Screen::Menu => menu::render_menu(frame, area, app_state),
            Screen::EmpireSelect => empire_select::render_empire_select(frame, area, app_state),
            Screen::NewGameSetup => new_game_setup::render_new_game_setup(frame, area, app_state),
            Screen::SectorOverview => {
                if let Some(state) = game_state {
                    sector_overview::render_sector_overview(frame, area, app_state, state);
                } else {
                    render_unavailable_screen(frame, area, "Sector Overview");
                }
            }
            Screen::SectorMap => {
                if let Some(state) = game_state {
                    sector_map::render_sector_map(frame, area, app_state, state);
                } else {
                    render_unavailable_screen(frame, area, "Sector Map");
                }
            }
            Screen::System => {
                if let Some(state) = game_state {
                    system::render_system(frame, area, app_state, state);
                } else {
                    render_unavailable_screen(frame, area, "System");
                }
            }
            Screen::Colony => {
                if let Some(state) = game_state {
                    colony::render_colony(frame, area, app_state, state);
                } else {
                    render_unavailable_screen(frame, area, "Colony");
                }
            }
            Screen::EmpireOverview => {
                if let Some(state) = game_state {
                    empire_overview::render_empire_overview(frame, area, app_state, state);
                } else {
                    render_unavailable_screen(frame, area, "Empire Overview");
                }
            }
            Screen::Research => {
                if let Some(state) = game_state {
                    research::render_research(frame, area, app_state, state);
                } else {
                    render_unavailable_screen(frame, area, "Research");
                }
            }
            Screen::Diplomacy => {
                if let Some(state) = game_state {
                    diplomacy::render_diplomacy(frame, area, app_state, state);
                } else {
                    render_unavailable_screen(frame, area, "Diplomacy");
                }
            }
            Screen::ShipDesigner => {
                if let Some(state) = game_state {
                    ship_designer::render_ship_designer(frame, area, app_state, state);
                } else {
                    render_unavailable_screen(frame, area, "Ship Designer");
                }
            }
            Screen::Settings => settings::render_settings(frame, area, app_state),
        }
    }
}

fn render_unavailable_screen(frame: &mut Frame, area: Rect, screen_name: &str) {
    let lines = vec![
        Line::from(Span::styled(
            format!("{screen_name} unavailable"),
            Theme::error_style(),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "No game is currently loaded. Return to the menu and start or load a game.",
            Theme::muted_style(),
        )),
    ];

    let paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .title(" Screen State Error ")
                .borders(Borders::ALL)
                .border_style(Theme::error_style())
                .style(Theme::default_style()),
        )
        .alignment(Alignment::Center)
        .style(Theme::default_style());
    frame.render_widget(paragraph, area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{backend::TestBackend, Terminal};

    #[test]
    fn game_screen_without_game_state_renders_fallback() {
        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).unwrap();
        let app_state = AppState {
            active: Screen::SectorMap,
            ..Default::default()
        };

        terminal
            .draw(|frame| {
                app_state
                    .active
                    .render(frame, frame.area(), &app_state, None);
            })
            .unwrap();

        let rendered: String = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect();
        assert!(rendered.contains("Sector Map unavailable"));
        assert!(rendered.contains("No game is currently loaded"));
    }
}
