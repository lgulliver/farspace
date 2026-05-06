//! Screen types and rendering

pub mod galaxy;
pub mod menu;

use ratatui::{layout::Rect, Frame};

use crate::AppState;
use game_core::GameState;

/// Active screen in the application
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Screen {
    #[default]
    Menu,
    Galaxy,
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
            Screen::Menu => menu::render_menu(frame, area),
            Screen::Galaxy => {
                if let Some(state) = game_state {
                    galaxy::render_galaxy(frame, area, app_state, state);
                }
            }
        }
    }
}
