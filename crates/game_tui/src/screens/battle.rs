//! Combat v3 BattleScreen — real, command-driven TUI screen.
//!
//! Reads the live `BattleSession` from `GameState.pending_battle_session`
//! when a player-involved battle is paused.  When no session is active,
//! falls back to displaying the most recent `BattleReportV3` from
//! `GameState.battle_reports_v3` so the player can review the card log
//! of any past battle.
//!
//! In v1 the engine still computes the outcome via the v2 auto-resolve
//! formula; the v3 module drafts hands and writes a `BattleReportV3` next
//! to the v2 report.  Card plays recorded in the round log come from the
//! engine's wrap.  A follow-up PR will replace the v2 formula with true
//! per-verb damage rules driven by `Command::PlayBattleCard`.

use crate::layout::centered_rect;
use crate::theme::Theme;
use game_core::combat_v3::card::{CardDef, CardVerb, HOLD_FIRE, card_by_id};
use game_core::combat_v3::{BattleReportV3, BattleSession};
use game_core::state::GameState;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

/// Render the battle screen.  Prefers an active `BattleSession`; falls
/// back to the latest v3 report; renders a "no recent battles"
/// placeholder when neither is available.
pub fn render_battle_screen(
    frame: &mut Frame,
    area: Rect,
    state: Option<&GameState>,
    show_help: bool,
) {
    let popup = centered_rect(88, 84, area);
    frame.render_widget(Clear, popup);

    let title = " Combat v3 — Card-driven Battle ";
    let block = Block::default()
        .title(Line::from(Span::styled(title, Theme::title_style())))
        .borders(Borders::ALL)
        .border_style(Theme::focused_border_style())
        .style(Theme::default_style());
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    if show_help {
        render_help(frame, inner);
        return;
    }

    let Some(state) = state else {
        render_no_state(frame, inner);
        return;
    };

    if let Some(session) = state.pending_battle_session.as_ref() {
        render_session(frame, inner, session);
        return;
    }

    if let Some(report) = state.battle_reports_v3.back() {
        render_report(frame, inner, report);
        return;
    }

    render_no_battles(frame, inner);
}

fn render_no_state(frame: &mut Frame, area: Rect) {
    let p = Paragraph::new(Line::from(Span::styled(
        "No game state loaded.",
        Theme::muted_style(),
    )))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Theme::dim_border_style())
            .style(Theme::default_style()),
    )
    .style(Theme::default_style());
    frame.render_widget(p, area);
}

fn render_no_battles(frame: &mut Frame, area: Rect) {
    let lines = vec![
        Line::from(""),
        Line::from(Span::styled("No recent battles.", Theme::title_style())),
        Line::from(""),
        Line::from(Span::styled(
            "Trigger a fleet engagement (move a fleet onto a hostile",
            Theme::muted_style(),
        )),
        Line::from(Span::styled(
            "fleet) to see a card-driven battle report here.",
            Theme::muted_style(),
        )),
    ];
    let p = Paragraph::new(lines)
        .style(Theme::default_style())
        .wrap(Wrap { trim: false });
    frame.render_widget(p, area);
}

fn render_session(frame: &mut Frame, area: Rect, session: &BattleSession) {
    let title = format!(
        " Battle session #{} — Round {}/{} ",
        session.session_id,
        session.round + 1,
        5,
    );
    let block = Block::default()
        .title(Line::from(Span::styled(title, Theme::title_style())))
        .borders(Borders::ALL)
        .border_style(Theme::focused_border_style())
        .style(Theme::default_style());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // integrity
            Constraint::Min(4),    // hands + detail
            Constraint::Length(2), // footer
        ])
        .split(inner);

    let bar = format!(
        "Integrity  YOU [{:>3}]   ENEMY [{:>3}]",
        session.integrity_a, session.integrity_b
    );
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(bar, Theme::header_style())))
            .style(Theme::default_style()),
        rows[0],
    );

    render_session_hands(frame, rows[1], session);

    let footer = Line::from(vec![
        Span::styled(" [1-5]Play ", Theme::accent_style()),
        Span::styled(" [r]Retreat ", Theme::accent_style()),
        Span::styled(" [?]Help ", Theme::accent_style()),
        Span::styled(" [Esc]Close ", Theme::accent_style()),
    ]);
    frame.render_widget(
        Paragraph::new(footer)
            .alignment(ratatui::layout::Alignment::Center)
            .style(Theme::default_style()),
        rows[2],
    );
}

fn render_session_hands(frame: &mut Frame, area: Rect, session: &BattleSession) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);
    render_hand_list(frame, cols[0], "Your hand", &session.hand_a, true);
    render_hand_list(
        frame,
        cols[1],
        "Enemy hand (hidden)",
        &session.hand_b,
        false,
    );
}

fn render_report(frame: &mut Frame, area: Rect, report: &BattleReportV3) {
    let title = format!(
        " Last battle (report #{}) — Turn {} — {} ",
        report.report_id, report.turn, report.system_outcome
    );
    let block = Block::default()
        .title(Line::from(Span::styled(title, Theme::title_style())))
        .borders(Borders::ALL)
        .border_style(Theme::focused_border_style())
        .style(Theme::default_style());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // integrity
            Constraint::Min(4),    // hands + detail
            Constraint::Length(2), // footer
        ])
        .split(inner);

    let bar = format!(
        "Integrity  YOU [{:>3}]→[{:<3}]   ENEMY [{:>3}]→[{:<3}]",
        report.integrity_a_start,
        report.integrity_a_end,
        report.integrity_b_start,
        report.integrity_b_end
    );
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(bar, Theme::header_style())))
            .style(Theme::default_style()),
        rows[0],
    );

    render_report_hands_and_log(frame, rows[1], report);

    let footer = Line::from(vec![
        Span::styled(" [Tab]Side ", Theme::accent_style()),
        Span::styled(" [?]Help ", Theme::accent_style()),
        Span::styled(" [Esc]Close ", Theme::accent_style()),
    ]);
    frame.render_widget(
        Paragraph::new(footer)
            .alignment(ratatui::layout::Alignment::Center)
            .style(Theme::default_style()),
        rows[2],
    );
}

fn render_report_hands_and_log(frame: &mut Frame, area: Rect, report: &BattleReportV3) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8), // hand lists
            Constraint::Min(2),    // round log
        ])
        .split(area);
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(rows[0]);
    render_hand_list(frame, cols[0], "Attacker hand", &report.hand_a, true);
    render_hand_list(frame, cols[1], "Defender hand", &report.hand_b, true);

    let mut log_lines: Vec<Line> = Vec::new();
    log_lines.push(Line::from(Span::styled("Round log", Theme::title_style())));
    log_lines.push(Line::from(Span::styled(
        "─".repeat(area.width.saturating_sub(2) as usize),
        Theme::dim_border_style(),
    )));
    for round in &report.rounds {
        let card_a = round
            .card_a
            .map(|c| card_by_id(c).name.to_string())
            .unwrap_or_else(|| "(no card)".to_string());
        let card_b = round
            .card_b
            .map(|c| card_by_id(c).name.to_string())
            .unwrap_or_else(|| "(no card)".to_string());
        log_lines.push(Line::from(Span::styled(
            format!(
                "R{}: A {} | D {}  →  YOU {}  ENEMY {}",
                round.round + 1,
                card_a,
                card_b,
                round.integrity_a_after,
                round.integrity_b_after
            ),
            Theme::text_primary_style(),
        )));
    }
    let log_p = Paragraph::new(log_lines)
        .style(Theme::default_style())
        .wrap(Wrap { trim: false });
    frame.render_widget(log_p, rows[1]);
}

fn render_hand_list(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    hand: &[game_core::combat_v3::CardId],
    visible: bool,
) {
    let block = Block::default()
        .title(Line::from(Span::styled(
            format!(" {title} "),
            Theme::title_style(),
        )))
        .borders(Borders::ALL)
        .border_style(Theme::dim_border_style())
        .style(Theme::default_style());

    let mut lines: Vec<Line> = Vec::new();
    if hand.is_empty() {
        lines.push(Line::from(Span::styled(
            "  (no cards)",
            Theme::muted_style(),
        )));
    } else {
        for (i, id) in hand.iter().enumerate() {
            let label = if visible {
                let card = card_by_id(*id);
                let verb = card.verb.label();
                let doctrine = card.doctrine;
                let name = truncate(card.name, 18);
                format!(
                    "  {} {} {:<18}  {:<10}  {}",
                    i + 1,
                    verb_glyph(card.verb),
                    name,
                    verb,
                    doctrine
                )
            } else {
                format!("  {} ?  (hidden)", i + 1)
            };
            lines.push(Line::from(Span::styled(
                label,
                if visible {
                    Theme::text_primary_style()
                } else {
                    Theme::muted_style()
                },
            )));
        }
    }
    let p = Paragraph::new(lines)
        .block(block)
        .style(Theme::default_style())
        .wrap(Wrap { trim: false });
    frame.render_widget(p, area);
}

fn verb_glyph(verb: CardVerb) -> &'static str {
    match verb {
        CardVerb::Strike => "✦",
        CardVerb::Guard => "▣",
        CardVerb::Maneuver => "↯",
        CardVerb::Evasive => "≋",
        CardVerb::Salvo => "✸",
        CardVerb::Fortify => "▤",
        CardVerb::Disrupt => "✕",
        CardVerb::Probe => "◎",
        CardVerb::Mark => "◉",
        CardVerb::Overcharge => "⚡",
        CardVerb::Withdraw => "↩",
        CardVerb::Bolster => "❖",
        CardVerb::Inspire => "✺",
        CardVerb::Noop => " ",
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
        out.push('…');
        out
    }
}

fn render_help(frame: &mut Frame, area: Rect) {
    let lines = vec![
        Line::from(Span::styled("Combat v3 — keys", Theme::title_style())),
        Line::from(""),
        Line::from(Span::styled(
            "  1  2  3  4  5   play card N from your hand",
            Theme::text_primary_style(),
        )),
        Line::from(Span::styled(
            "  Tab            toggle side view (when on a battle report)",
            Theme::text_primary_style(),
        )),
        Line::from(Span::styled(
            "  r              retreat (free command, burns round)",
            Theme::text_primary_style(),
        )),
        Line::from(Span::styled(
            "  ?              this help overlay (any key dismisses)",
            Theme::text_primary_style(),
        )),
        Line::from(Span::styled(
            "  Esc            close screen",
            Theme::text_primary_style(),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Damage is still computed by the v2 auto-resolve formula; a",
            Theme::muted_style(),
        )),
        Line::from(Span::styled(
            "follow-up PR will replace the formula with per-verb card rules",
            Theme::muted_style(),
        )),
        Line::from(Span::styled(
            "and wire Command::PlayBattleCard to the engine.",
            Theme::muted_style(),
        )),
    ];
    let p = Paragraph::new(lines)
        .style(Theme::default_style())
        .wrap(Wrap { trim: false });
    frame.render_widget(p, area);
}

// Suppress unused-import warnings for the symbol aliases kept for the
// card catalog and the placeholder id.
#[allow(dead_code)]
fn _card_ref(_c: &CardDef) {}
#[allow(dead_code)]
fn _hold_ref() -> CardDef {
    HOLD_FIRE
}

#[cfg(test)]
mod tests {
    use super::*;
    use game_core::combat_v3::{BattleReportV3, BattleRoundSummary, CardId};
    use game_core::state::{Fleet, FleetKind, GameState, ScenarioSetup, StarId};
    use ratatui::{Terminal, backend::TestBackend};

    fn new_state() -> GameState {
        game_core::engine::Engine::new_from_setup(ScenarioSetup::default_for_seed(42)).state
    }

    #[test]
    fn render_with_no_state_does_not_panic() {
        let backend = TestBackend::new(140, 50);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| render_battle_screen(frame, frame.area(), None, false))
            .unwrap();
    }

    #[test]
    fn render_with_no_battles_does_not_panic() {
        let backend = TestBackend::new(140, 50);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = new_state();
        terminal
            .draw(|frame| render_battle_screen(frame, frame.area(), Some(&state), false))
            .unwrap();
    }

    #[test]
    fn render_with_help_does_not_panic() {
        let backend = TestBackend::new(140, 50);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| render_battle_screen(frame, frame.area(), None, true))
            .unwrap();
    }

    #[test]
    fn render_with_report_does_not_panic() {
        let backend = TestBackend::new(140, 50);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = new_state();
        let report = BattleReportV3 {
            report_id: 1,
            turn: 5,
            star: StarId(0),
            fleet_a: game_core::state::FleetId(1),
            fleet_b: game_core::state::FleetId(2),
            empire_a: state.player_empire,
            empire_b: state.player_empire,
            role_a: game_core::state::FleetRole::StrikeFleet,
            role_b: game_core::state::FleetRole::DefenseFleet,
            formation_a: game_core::state::FleetFormation::Balanced,
            formation_b: game_core::state::FleetFormation::Balanced,
            supply_a: game_core::state::FleetSupplyState::Supplied,
            supply_b: game_core::state::FleetSupplyState::Supplied,
            ships_a: 1,
            ships_b: 1,
            integrity_a_start: 100,
            integrity_b_start: 100,
            integrity_a_end: 50,
            integrity_b_end: 25,
            fleet_a_destroyed: false,
            fleet_b_destroyed: false,
            fleet_a_retreated: false,
            fleet_b_retreated: false,
            hand_a: vec![CardId(1), CardId(2), CardId(9), CardId(0), CardId(0)],
            hand_b: vec![CardId(1), CardId(5), CardId(0), CardId(0), CardId(0)],
            rounds: vec![
                BattleRoundSummary {
                    round: 0,
                    card_a: Some(CardId(1)),
                    card_b: Some(CardId(1)),
                    effect_a: "Strike 18".to_string(),
                    effect_b: "Strike 14".to_string(),
                    integrity_a_after: 86,
                    integrity_b_after: 82,
                },
                BattleRoundSummary {
                    round: 1,
                    card_a: Some(CardId(2)),
                    card_b: Some(CardId(5)),
                    effect_a: "Guard".to_string(),
                    effect_b: "Evasive".to_string(),
                    integrity_a_after: 86,
                    integrity_b_after: 78,
                },
            ],
            system_outcome: "Attacker holds".to_string(),
        };
        state.battle_reports_v3.push_back(report);
        terminal
            .draw(|frame| render_battle_screen(frame, frame.area(), Some(&state), false))
            .unwrap();
    }

    #[test]
    fn render_with_pending_session_does_not_panic() {
        let backend = TestBackend::new(140, 50);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = new_state();
        // Add a player fleet.
        let fleet_id = game_core::state::FleetId(9001);
        state.fleets.insert(
            fleet_id,
            Fleet {
                id: fleet_id,
                owner: state.player_empire,
                kind: FleetKind::Destroyer,
                location: StarId(0),
                ships: 1,
                strength: 10,
                integrity: 100,
            },
        );
        // Synthesise a session manually.
        state.pending_battle_session = Some(game_core::combat_v3::BattleSession {
            session_id: 1,
            setup: game_core::combat_v3::BattleSetupSummary {
                star: StarId(0),
                fleet_a: fleet_id,
                fleet_b: game_core::state::FleetId(2),
                empire_a: state.player_empire,
                empire_b: state.player_empire,
                role_a: game_core::state::FleetRole::StrikeFleet,
                role_b: game_core::state::FleetRole::DefenseFleet,
                formation_a: game_core::state::FleetFormation::Balanced,
                formation_b: game_core::state::FleetFormation::Balanced,
                supply_a: game_core::state::FleetSupplyState::Supplied,
                supply_b: game_core::state::FleetSupplyState::Supplied,
                ships_a: 1,
                ships_b: 1,
                integrity_a_start: 100,
                integrity_b_start: 100,
                doctrine_a: String::new(),
                doctrine_b: String::new(),
            },
            hand_a: vec![CardId(1), CardId(2), CardId(9), CardId(0), CardId(0)],
            hand_b: vec![CardId(1), CardId(5), CardId(0), CardId(0), CardId(0)],
            round: 0,
            integrity_a: 100,
            integrity_b: 100,
            phase: game_core::combat_v3::BattlePhase::AwaitingInput,
        });
        terminal
            .draw(|frame| render_battle_screen(frame, frame.area(), Some(&state), false))
            .unwrap();
    }
}
