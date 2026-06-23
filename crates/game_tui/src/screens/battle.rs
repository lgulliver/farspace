//! Combat v3 real battle overlay.
//!
//! This screen replaces the inert `battle_mock` prototype.  It reads
//! `GameState::pending_battle_session` and renders the player's hand,
//! the enemy hand (hidden by default), integrity gauges, and a round
//! log.  All combat decisions live in `game_core::combat_v3`; the
//! overlay only emits `Command::PlayBattleCard` and
//! `Command::RetreatFromBattle`.

use crate::layout::centered_rect;
use crate::theme::Theme;
use crossterm::event::{KeyCode, KeyEvent};
use game_core::GameState;
use game_core::combat_v3::{BattleSession, BattleSide, HAND_SIZE, MAX_ROUNDS, card_by_id};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

/// Outcome of the overlay's key handler.  The TUI dispatch layer turns
/// these into `Command` variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BattleAction {
    /// Play the card at this hand index (0..HAND_SIZE).
    PlayCard(usize),
    /// Free retreat.
    Retreat,
    /// Toggle the help overlay.
    ToggleHelp,
    /// Dismiss the help overlay (any key while help is showing).
    Dismiss,
}

/// State local to the battle overlay.  Keeps the help flag and the
/// current cursor position across re-renders.
#[derive(Debug, Clone, Default)]
pub struct BattleOverlayState {
    pub show_help: bool,
    pub cursor: usize,
}

impl BattleOverlayState {
    /// Construct a fresh overlay state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Reset state when a fresh session arrives.  Called by the TUI
    /// when a pending session first becomes visible.
    pub fn reset(&mut self) {
        self.show_help = false;
        self.cursor = 0;
    }

    /// Handle a key while the overlay is active.  Returns the action
    /// the TUI dispatch layer should take (if any).
    pub fn handle_key(&mut self, key: KeyEvent, hand_len: usize) -> Option<BattleAction> {
        if self.show_help {
            // Any key dismisses help.
            self.show_help = false;
            return Some(BattleAction::Dismiss);
        }

        match key.code {
            KeyCode::Char('1') if hand_len >= 1 => Some(BattleAction::PlayCard(0)),
            KeyCode::Char('2') if hand_len >= 2 => Some(BattleAction::PlayCard(1)),
            KeyCode::Char('3') if hand_len >= 3 => Some(BattleAction::PlayCard(2)),
            KeyCode::Char('4') if hand_len >= 4 => Some(BattleAction::PlayCard(3)),
            KeyCode::Char('5') if hand_len >= 5 => Some(BattleAction::PlayCard(4)),
            KeyCode::Char('r') | KeyCode::Char('R') => Some(BattleAction::Retreat),
            KeyCode::Char('?') => {
                self.show_help = true;
                Some(BattleAction::ToggleHelp)
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if hand_len > 0 {
                    self.cursor = (self.cursor + 1).min(hand_len - 1);
                }
                None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.cursor = self.cursor.saturating_sub(1);
                None
            }
            KeyCode::Enter => {
                if hand_len > 0 {
                    Some(BattleAction::PlayCard(self.cursor.min(hand_len - 1)))
                } else {
                    None
                }
            }
            KeyCode::Esc => {
                // Esc is ignored while a battle is pending.  The overlay
                // only closes automatically when the session finalises.
                None
            }
            _ => None,
        }
    }
}

/// Render the Combat v3 battle overlay.
pub fn render_battle(
    frame: &mut Frame,
    area: Rect,
    game_state: &GameState,
    overlay: &BattleOverlayState,
) {
    let Some(session) = game_state.pending_battle_session.as_ref() else {
        return;
    };

    let popup = centered_rect(92, 86, area);
    frame.render_widget(Clear, popup);

    let player = game_state.player_empire;
    let player_side = if session.empire_a == player {
        BattleSide::Attacker
    } else {
        BattleSide::Defender
    };

    let title = format!(
        " Combat v3 — Round {}/{} — Fleet {} vs Fleet {} ",
        session.round, MAX_ROUNDS, session.attacker.0, session.defender.0
    );
    let block = Block::default()
        .title(Line::from(Span::styled(title, Theme::title_style())))
        .borders(Borders::ALL)
        .border_style(Theme::focused_border_style())
        .style(Theme::default_style());
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    if overlay.show_help {
        render_help(frame, inner);
        return;
    }

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(12), // hands + detail
            Constraint::Min(4),     // integrity + log
            Constraint::Length(2),  // footer
        ])
        .split(inner);

    render_hands_and_detail(frame, rows[0], session, player_side, overlay);
    render_mid(frame, rows[1], session, player_side, game_state);
    render_footer(frame, rows[2], session, player_side);
}

fn render_hands_and_detail(
    frame: &mut Frame,
    area: Rect,
    session: &BattleSession,
    player_side: BattleSide,
    overlay: &BattleOverlayState,
) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(35),
            Constraint::Percentage(35),
            Constraint::Percentage(30),
        ])
        .split(area);

    let (player_hand, enemy_hand) = match player_side {
        BattleSide::Attacker => (&session.hand_a, &session.hand_b),
        BattleSide::Defender => (&session.hand_b, &session.hand_a),
    };

    render_hand(
        frame,
        cols[0],
        "Your hand",
        player_hand,
        Some(overlay.cursor),
        true,
    );
    render_hand(frame, cols[2], "Enemy hand", enemy_hand, None, false);
    render_detail(frame, cols[1], player_hand, overlay);
}

fn render_hand(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    hand: &[game_core::combat_v3::CardId],
    cursor: Option<usize>,
    is_player: bool,
) {
    let block = Block::default()
        .title(Line::from(Span::styled(
            format!(" {title} "),
            Theme::title_style(),
        )))
        .borders(Borders::ALL)
        .border_style(if is_player {
            Theme::focused_border_style()
        } else {
            Theme::dim_border_style()
        })
        .style(Theme::default_style());

    let mut lines: Vec<Line> = Vec::new();
    if hand.is_empty() {
        lines.push(Line::from(Span::styled(
            "  (no cards left)",
            Theme::muted_style(),
        )));
    } else {
        for (i, id) in hand.iter().enumerate() {
            let is_cursor = Some(i) == cursor;
            let marker = if is_cursor && is_player { "▸" } else { " " };
            let key_hint = if is_player && i < HAND_SIZE {
                format!("{}", i + 1)
            } else {
                " ".to_string()
            };
            let label = if is_player {
                let card = card_by_id(*id);
                let name = format!("{:<20}", truncate(card.name, 20));
                let verb = card.verb.label();
                format!(
                    " {key_hint} {marker} {name}  {verb:<10}",
                    name = name,
                    marker = marker,
                    verb = verb,
                )
            } else {
                format!(
                    " {key_hint} {marker} ?  (hidden)",
                    marker = marker,
                    key_hint = key_hint
                )
            };
            let style = if is_cursor && is_player {
                Theme::highlight_style()
            } else {
                Theme::text_primary_style()
            };
            lines.push(Line::from(Span::styled(label, style)));
        }
    }

    let p = Paragraph::new(lines)
        .block(block)
        .style(Theme::default_style())
        .wrap(Wrap { trim: false });
    frame.render_widget(p, area);
}

fn render_detail(
    frame: &mut Frame,
    area: Rect,
    hand: &[game_core::combat_v3::CardId],
    overlay: &BattleOverlayState,
) {
    let block = Block::default()
        .title(Line::from(Span::styled(
            " Card detail ",
            Theme::title_style(),
        )))
        .borders(Borders::ALL)
        .border_style(Theme::dim_border_style())
        .style(Theme::default_style());

    let cursor = overlay.cursor.min(hand.len().saturating_sub(1));
    let card = hand.get(cursor).map(|id| card_by_id(*id));

    let mut lines: Vec<Line> = Vec::new();
    if let Some(card) = card {
        lines.push(Line::from(Span::styled(card.name, Theme::title_style())));
        lines.push(Line::from(Span::styled(
            "─".repeat(area.width.saturating_sub(2) as usize),
            Theme::dim_border_style(),
        )));
        lines.push(Line::from(vec![
            Span::styled("Verb:      ", Theme::muted_style()),
            Span::styled(card.verb.label(), Theme::text_primary_style()),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Doctrine:  ", Theme::muted_style()),
            Span::styled(card.doctrine_bias, Theme::text_primary_style()),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Source:    ", Theme::muted_style()),
            Span::styled(card.source, Theme::text_primary_style()),
        ]));
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled("Effect", Theme::title_style())));
        lines.push(Line::from(Span::styled(
            card.effect_text,
            Theme::text_primary_style(),
        )));
    } else {
        lines.push(Line::from(Span::styled(
            "  (no card focused)",
            Theme::muted_style(),
        )));
    }

    let p = Paragraph::new(lines)
        .block(block)
        .style(Theme::default_style())
        .wrap(Wrap { trim: false });
    frame.render_widget(p, area);
}

fn render_mid(
    frame: &mut Frame,
    area: Rect,
    session: &BattleSession,
    player_side: BattleSide,
    game_state: &GameState,
) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(2)])
        .split(area);

    // Integrity bars: each side uses its *own* starting integrity as
    // the denominator so the gauges reflect the correct proportion.
    let (player_int, enemy_int, player_start, enemy_start) = match player_side {
        BattleSide::Attacker => (
            session.integrity_a,
            session.integrity_b,
            session.integrity_a_start,
            session.integrity_b_start,
        ),
        BattleSide::Defender => (
            session.integrity_b,
            session.integrity_a,
            session.integrity_b_start,
            session.integrity_a_start,
        ),
    };
    let bar = format!(
        "Integrity  YOU [{:>3}/{:>3}]   ENEMY [{:>3}/{:>3}]",
        player_int, player_start, enemy_int, enemy_start
    );
    let integrity_p = Paragraph::new(Line::from(Span::styled(bar, Theme::header_style())))
        .style(Theme::default_style());
    frame.render_widget(integrity_p, rows[0]);

    let mut log_lines: Vec<Line> = Vec::new();
    for round in session
        .rounds
        .iter()
        .rev()
        .take(6)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
    {
        // Show the round from the player's perspective: "YOU" is the
        // side the player controls this battle; "FOE" is the other.
        // effect_a/effect_b already carry the resolved text per side.
        let (you_effect, foe_effect) = match player_side {
            BattleSide::Attacker => (&round.effect_a, &round.effect_b),
            BattleSide::Defender => (&round.effect_b, &round.effect_a),
        };
        let entry = format!(
            "R{}: YOU={}  FOE={}  [{}hp / {}hp]",
            round.round, you_effect, foe_effect, round.integrity_a_after, round.integrity_b_after,
        );
        log_lines.push(Line::from(Span::styled(entry, Theme::text_primary_style())));
    }
    if log_lines.is_empty() {
        log_lines.push(Line::from(Span::styled(
            "(battle just started)",
            Theme::muted_style(),
        )));
    }
    // Annotate the most recent round for clarity.
    let _ = game_state;
    let log_p = Paragraph::new(log_lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Theme::dim_border_style())
                .style(Theme::default_style()),
        )
        .wrap(Wrap { trim: false })
        .style(Theme::default_style());
    frame.render_widget(log_p, rows[1]);
}

fn render_footer(
    frame: &mut Frame,
    area: Rect,
    _session: &BattleSession,
    _player_side: BattleSide,
) {
    let line = Line::from(vec![
        Span::styled(" [1-5]Play ", Theme::accent_style()),
        Span::styled(" [j/k]Nav ", Theme::accent_style()),
        Span::styled(" [r]Retreat ", Theme::accent_style()),
        Span::styled(" [?]Help ", Theme::accent_style()),
    ]);
    let p = Paragraph::new(line)
        .alignment(ratatui::layout::Alignment::Center)
        .style(Theme::default_style());
    frame.render_widget(p, area);
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
            "  Enter          play the focused card (j/k to move)",
            Theme::text_primary_style(),
        )),
        Line::from(Span::styled(
            "  j  k           move cursor down / up within the hand",
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
            "  Esc            ignored while a battle is pending",
            Theme::text_primary_style(),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "The card detail panel shows full text for the focused card.",
            Theme::muted_style(),
        )),
    ];
    let p = Paragraph::new(lines)
        .style(Theme::default_style())
        .wrap(Wrap { trim: false });
    frame.render_widget(p, area);
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

#[cfg(test)]
mod tests {
    use super::*;
    use game_core::combat_v3::{BattleSession, BattleSetupSummary};
    use game_core::{
        EmpireId, FleetFormation, FleetId, FleetKind, FleetRole, FleetSupplyState, StarId,
    };

    fn sample_session(player: EmpireId) -> BattleSession {
        let other = EmpireId(player.0 + 1);
        BattleSession::new(
            1,
            StarId(1),
            FleetId(1),
            FleetId(2),
            player,
            other,
            vec![
                game_core::combat_v3::CardId::KINETIC_SALVO,
                game_core::combat_v3::CardId::ABLATIVE_HULL,
                game_core::combat_v3::CardId::PHASED_SHIELD,
                game_core::combat_v3::CardId::DRIFT_BURN,
                game_core::combat_v3::CardId::SENSOR_SWEEP,
            ],
            vec![
                game_core::combat_v3::CardId::KINETIC_SALVO,
                game_core::combat_v3::CardId::ABLATIVE_HULL,
                game_core::combat_v3::HOLD_FIRE.id,
                game_core::combat_v3::HOLD_FIRE.id,
                game_core::combat_v3::HOLD_FIRE.id,
            ],
            100,
            100,
            BattleSetupSummary {
                role_a: FleetRole::StrikeFleet,
                role_b: FleetRole::DefenseFleet,
                formation_a: FleetFormation::Balanced,
                formation_b: FleetFormation::Defensive,
                doctrine_a: String::new(),
                doctrine_b: String::new(),
                supply_a: FleetSupplyState::Supplied,
                supply_b: FleetSupplyState::Supplied,
                kind_a: FleetKind::Destroyer,
                kind_b: FleetKind::EscortFrigate,
                ships_a: 1,
                ships_b: 1,
            },
        )
    }

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, crossterm::event::KeyModifiers::NONE)
    }

    #[test]
    fn keys_1_through_5_play_matching_card() {
        let mut state = BattleOverlayState::default();
        for (i, expected) in [0usize, 1, 2, 3, 4].iter().enumerate() {
            let k = match i {
                0 => KeyCode::Char('1'),
                1 => KeyCode::Char('2'),
                2 => KeyCode::Char('3'),
                3 => KeyCode::Char('4'),
                _ => KeyCode::Char('5'),
            };
            let action = state.handle_key(key(k), 5);
            assert_eq!(action, Some(BattleAction::PlayCard(*expected)));
        }
    }

    #[test]
    fn key_r_triggers_retreat() {
        let mut state = BattleOverlayState::default();
        let action = state.handle_key(key(KeyCode::Char('r')), 5);
        assert_eq!(action, Some(BattleAction::Retreat));
    }

    #[test]
    fn help_overlay_dismisses_on_next_key() {
        let mut state = BattleOverlayState {
            show_help: true,
            ..Default::default()
        };
        let action = state.handle_key(key(KeyCode::Char('x')), 5);
        assert_eq!(action, Some(BattleAction::Dismiss));
        assert!(!state.show_help);
    }

    #[test]
    fn esc_is_ignored_while_battle_pending() {
        let mut state = BattleOverlayState::default();
        let action = state.handle_key(key(KeyCode::Esc), 5);
        assert_eq!(action, None);
    }

    #[test]
    fn cursor_moves_with_jk() {
        let mut state = BattleOverlayState::default();
        let _ = state.handle_key(key(KeyCode::Char('j')), 5);
        assert_eq!(state.cursor, 1);
        let _ = state.handle_key(key(KeyCode::Char('j')), 5);
        assert_eq!(state.cursor, 2);
        let _ = state.handle_key(key(KeyCode::Char('k')), 5);
        assert_eq!(state.cursor, 1);
    }

    #[test]
    fn out_of_range_key_is_ignored() {
        let mut state = BattleOverlayState::default();
        // Hand has 3 cards; '5' should be ignored.
        let action = state.handle_key(key(KeyCode::Char('5')), 3);
        assert_eq!(action, None);
    }

    #[test]
    fn render_does_not_panic_when_no_pending_session() {
        let backend = ratatui::backend::TestBackend::new(140, 50);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        let state = GameState::default();
        let overlay = BattleOverlayState::default();
        terminal
            .draw(|frame| {
                render_battle(frame, frame.area(), &state, &overlay);
            })
            .unwrap();
    }

    #[test]
    fn render_does_not_panic_with_pending_session() {
        let backend = ratatui::backend::TestBackend::new(140, 50);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        let state = GameState {
            player_empire: EmpireId(1),
            pending_battle_session: Some(sample_session(EmpireId(1))),
            ..Default::default()
        };
        let overlay = BattleOverlayState::default();
        terminal
            .draw(|frame| {
                render_battle(frame, frame.area(), &state, &overlay);
            })
            .unwrap();
    }

    #[test]
    fn render_with_help_does_not_panic() {
        let backend = ratatui::backend::TestBackend::new(140, 50);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        let state = GameState {
            player_empire: EmpireId(1),
            pending_battle_session: Some(sample_session(EmpireId(1))),
            ..Default::default()
        };
        let overlay = BattleOverlayState {
            show_help: true,
            ..Default::default()
        };
        terminal
            .draw(|frame| {
                render_battle(frame, frame.area(), &state, &overlay);
            })
            .unwrap();
    }
}
