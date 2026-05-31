use crate::components::{render_battle_reports, render_dispatch, render_help, render_palette};
use crate::screens::Screen;
use crate::visual_mode::VisualMode;
use crate::AppState;
use game_core::GameState;
use ratatui::backend::TestBackend;
use ratatui::Terminal;

#[derive(Debug, Clone)]
pub enum E2eRenderTarget {
    Screen(Screen),
    HelpOverlay(Screen),
    PaletteOverlay {
        screen: Screen,
        input: String,
    },
    DispatchOverlay {
        screen: Screen,
        history_index: usize,
    },
    BattleReportsOverlay {
        screen: Screen,
        report_index: usize,
        inspect: bool,
    },
}

pub fn render_target_to_text(
    state: &GameState,
    target: &E2eRenderTarget,
    width: u16,
    height: u16,
) -> std::io::Result<String> {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app_state = seeded_app_state(state);

    terminal.draw(|frame| {
        let area = frame.area();

        let screen = match target {
            E2eRenderTarget::Screen(screen)
            | E2eRenderTarget::HelpOverlay(screen)
            | E2eRenderTarget::DispatchOverlay { screen, .. }
            | E2eRenderTarget::BattleReportsOverlay { screen, .. }
            | E2eRenderTarget::PaletteOverlay { screen, .. } => *screen,
        };

        app_state.active = screen;
        screen.render(frame, area, &app_state, Some(state));

        match target {
            E2eRenderTarget::Screen(_) => {}
            E2eRenderTarget::HelpOverlay(screen) => {
                render_help(frame, area, screen);
            }
            E2eRenderTarget::PaletteOverlay { input, .. } => {
                render_palette(frame, area, input, app_state.visual_mode);
            }
            E2eRenderTarget::DispatchOverlay { history_index, .. } => {
                if !state.galactic_dispatches.is_empty() {
                    let idx =
                        (*history_index).min(state.galactic_dispatches.len().saturating_sub(1));
                    render_dispatch(
                        frame,
                        area,
                        &state.galactic_dispatches[idx],
                        idx,
                        state.galactic_dispatches.len(),
                        app_state.visual_mode,
                    );
                }
            }
            E2eRenderTarget::BattleReportsOverlay {
                report_index,
                inspect,
                ..
            } => {
                render_battle_reports(
                    frame,
                    area,
                    &state.battle_reports,
                    *report_index,
                    *inspect,
                    app_state.visual_mode,
                );
            }
        }
    }).unwrap();

    Ok(buffer_to_text(terminal.backend().buffer(), width, height))
}

fn seeded_app_state(state: &GameState) -> AppState {
    let mut app_state = AppState {
        visual_mode: VisualMode::Ascii,
        ..Default::default()
    };
    app_state.navigation.selected_sector = state.sectors.keys().next().copied();
    app_state.navigation.selected_star = state
        .explored_stars
        .iter()
        .next()
        .copied()
        .or_else(|| state.stars.keys().next().copied());
    app_state.colony.selected_colony = state
        .colonies
        .values()
        .find(|colony| colony.owner == state.player_empire)
        .map(|colony| colony.id)
        .or_else(|| state.colonies.keys().next().copied());
    app_state
}

fn buffer_to_text(buffer: &ratatui::buffer::Buffer, width: u16, height: u16) -> String {
    let mut rows = Vec::with_capacity(height as usize);
    for y in 0..height {
        let mut row = String::with_capacity(width as usize);
        for x in 0..width {
            row.push_str(buffer[(x, y)].symbol());
        }
        rows.push(row.trim_end().to_string());
    }
    rows.join("\n")
}
