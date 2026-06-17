//! Combat v3 BattleScreen — mock prototype
//!
//! Standalone TUI prototype of the card-driven battle resolution screen
//! described in `docs/design/combat-v3.md`. The mock uses static fixtures and
//! has no dependency on `game_core::combat_v3` (which is not yet
//! implemented). The real implementation will replace this mock with a
//! command-driven screen that consumes `BattleSession` from
//! `GameState.pending_battle_session`.
//!
//! Open with the global `M` key (uppercase; no conflict with existing
//! in-game bindings). Close with `Esc`. The mock is otherwise inert — it
//! does not touch the engine or save data.

use crate::layout::centered_rect;
use crate::theme::Theme;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

/// Maximum hand size per side.
pub const HAND_SIZE: usize = 5;
/// Maximum number of rounds.
pub const MAX_ROUNDS: u8 = 5;
/// Maximum log entries kept on screen.
const LOG_CAP: usize = 6;

/// Static card verb.  Mirrors the verbs listed in the design doc.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardVerb {
    Strike,
    Guard,
    Maneuver,
    Evasive,
    Salvo,
    Fortify,
    Disrupt,
    Probe,
    Mark,
    Overcharge,
    Withdraw,
    Bolster,
    Inspire,
    Noop,
}

impl CardVerb {
    pub fn label(self) -> &'static str {
        match self {
            CardVerb::Strike => "Strike",
            CardVerb::Guard => "Guard",
            CardVerb::Maneuver => "Maneuver",
            CardVerb::Evasive => "Evasive",
            CardVerb::Salvo => "Salvo",
            CardVerb::Fortify => "Fortify",
            CardVerb::Disrupt => "Disrupt",
            CardVerb::Probe => "Probe",
            CardVerb::Mark => "Mark",
            CardVerb::Overcharge => "Overcharge",
            CardVerb::Withdraw => "Withdraw",
            CardVerb::Bolster => "Bolster",
            CardVerb::Inspire => "Inspire",
            CardVerb::Noop => "(no-op)",
        }
    }
}

/// Static card descriptor.  Mirrors the v1 pool from the design doc.
#[derive(Debug, Clone, Copy)]
pub struct MockCard {
    pub id: u16,
    pub name: &'static str,
    pub verb: CardVerb,
    pub doctrine: &'static str,
    pub source: &'static str,
}

/// Static v1 pool.  Counts to 23 unique cards + 1 `Hold Fire` fallback.
pub static POOL: &[MockCard] = &[
    MockCard {
        id: 1,
        name: "Kinetic Salvo",
        verb: CardVerb::Strike,
        doctrine: "Militarist",
        source: "Hull",
    },
    MockCard {
        id: 2,
        name: "Ablative Hull",
        verb: CardVerb::Guard,
        doctrine: "Isolationist",
        source: "Component",
    },
    MockCard {
        id: 3,
        name: "Phased Shield",
        verb: CardVerb::Guard,
        doctrine: "Isolationist",
        source: "Component",
    },
    MockCard {
        id: 4,
        name: "CIWS Grid",
        verb: CardVerb::Disrupt,
        doctrine: "—",
        source: "Component",
    },
    MockCard {
        id: 5,
        name: "Burn Maneuver",
        verb: CardVerb::Evasive,
        doctrine: "Explorer",
        source: "Component",
    },
    MockCard {
        id: 6,
        name: "Drift Burn",
        verb: CardVerb::Maneuver,
        doctrine: "Explorer",
        source: "Hull",
    },
    MockCard {
        id: 7,
        name: "Targeting Lock",
        verb: CardVerb::Mark,
        doctrine: "Militarist",
        source: "Component",
    },
    MockCard {
        id: 8,
        name: "Sensor Sweep",
        verb: CardVerb::Probe,
        doctrine: "Explorer",
        source: "Component",
    },
    MockCard {
        id: 9,
        name: "Orbital Bombardment",
        verb: CardVerb::Salvo,
        doctrine: "Militarist",
        source: "Hull",
    },
    MockCard {
        id: 10,
        name: "Defensive Screen",
        verb: CardVerb::Fortify,
        doctrine: "Isolationist",
        source: "Hull",
    },
    MockCard {
        id: 11,
        name: "Troop Drop",
        verb: CardVerb::Bolster,
        doctrine: "Militarist",
        source: "Mission",
    },
    MockCard {
        id: 12,
        name: "Warp Retreat",
        verb: CardVerb::Withdraw,
        doctrine: "Explorer",
        source: "Tech",
    },
    MockCard {
        id: 13,
        name: "Ordnance Overcharge",
        verb: CardVerb::Overcharge,
        doctrine: "Militarist",
        source: "Tech",
    },
    MockCard {
        id: 14,
        name: "Formation Rally",
        verb: CardVerb::Inspire,
        doctrine: "Unity",
        source: "Tech",
    },
    MockCard {
        id: 15,
        name: "Surveyor's Gambit",
        verb: CardVerb::Probe,
        doctrine: "Explorer",
        source: "Hull",
    },
    MockCard {
        id: 16,
        name: "Coercive Mandate",
        verb: CardVerb::Strike,
        doctrine: "Militarist",
        source: "Faction: Vorath",
    },
    MockCard {
        id: 17,
        name: "Siege Doctrine",
        verb: CardVerb::Strike,
        doctrine: "Militarist",
        source: "Faction: Terran Dominion",
    },
    MockCard {
        id: 18,
        name: "Industrial Juggernaut",
        verb: CardVerb::Guard,
        doctrine: "Industrialist",
        source: "Faction: Ashveran",
    },
    MockCard {
        id: 19,
        name: "Algorithmic Defense",
        verb: CardVerb::Guard,
        doctrine: "Technologist",
        source: "Faction: Elarith",
    },
    MockCard {
        id: 20,
        name: "Pathfinder's Wager",
        verb: CardVerb::Evasive,
        doctrine: "Explorer",
        source: "Faction: Luminal",
    },
    MockCard {
        id: 21,
        name: "Council of Voices",
        verb: CardVerb::Inspire,
        doctrine: "Unity",
        source: "Faction: Terran Concord",
    },
    MockCard {
        id: 22,
        name: "Trade Barge Stand",
        verb: CardVerb::Guard,
        doctrine: "Merchant",
        source: "Faction: Thalori",
    },
    MockCard {
        id: 23,
        name: "Bloom Shield",
        verb: CardVerb::Guard,
        doctrine: "Biologist",
        source: "Faction: Sylvaran",
    },
];

/// Last-resort filler card.
pub const HOLD_FIRE: MockCard = MockCard {
    id: 0,
    name: "Hold Fire",
    verb: CardVerb::Noop,
    doctrine: "—",
    source: "Fallback",
};

fn card_by_id(id: u16) -> &'static MockCard {
    if id == HOLD_FIRE.id {
        return &HOLD_FIRE;
    }
    POOL.iter().find(|c| c.id == id).unwrap_or(&HOLD_FIRE)
}

/// Which side the player is currently viewing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MockSide {
    Self_,
    Enemy,
}

impl MockSide {
    fn toggle(self) -> Self {
        match self {
            MockSide::Self_ => MockSide::Enemy,
            MockSide::Enemy => MockSide::Self_,
        }
    }
}

/// Mock battle session state.  Holds the round counter, both hands, the
/// integrity gauges, the side toggle, the log buffer, and the help flag.
///
/// The mock is fully deterministic: integrity changes and the log are
/// computed locally from the played card; no RNG is consumed.  The real
/// implementation will source these values from `BattleSession` and pull
/// RNG from `GameState.rng`.
#[derive(Debug, Clone)]
pub struct MockBattleState {
    pub round: u8,
    pub integrity_self: u32,
    pub integrity_enemy: u32,
    pub hand_self: Vec<u16>,
    pub hand_enemy: Vec<u16>,
    pub enemy_revealed: bool,
    pub side: MockSide,
    pub show_help: bool,
    pub log: Vec<String>,
    pub finished: bool,
    pub outcome: Option<&'static str>,
}

impl Default for MockBattleState {
    fn default() -> Self {
        Self::new()
    }
}

impl MockBattleState {
    /// Create a new mock state with fixture hands.
    pub fn new() -> Self {
        let hand_self = vec![2, 10, 18, 4, 6];
        let hand_enemy = vec![1, 9, 13, 7, 14];
        Self {
            round: 1,
            integrity_self: 100,
            integrity_enemy: 100,
            hand_self,
            hand_enemy,
            enemy_revealed: false,
            side: MockSide::Self_,
            show_help: false,
            log: vec!["Battle joined. Round 1 begins.".to_string()],
            finished: false,
            outcome: None,
        }
    }

    /// Reset the mock to its initial state.
    pub fn reset(&mut self) {
        *self = Self::new();
    }

    fn push_log(&mut self, msg: String) {
        self.log.push(msg);
        if self.log.len() > LOG_CAP {
            let drop = self.log.len() - LOG_CAP;
            self.log.drain(0..drop);
        }
    }

    /// Apply the player's chosen card.  Returns true if the battle
    /// continues; false if the battle ended this round.
    pub fn play_self(&mut self, card_index: usize) {
        if self.finished {
            return;
        }
        let Some(card_id) = self.hand_self.get(card_index).copied() else {
            return;
        };
        let card = card_by_id(card_id);
        let damage = match card.verb {
            CardVerb::Strike => 18,
            CardVerb::Salvo => 24,
            CardVerb::Overcharge => 28,
            CardVerb::Withdraw => 0,
            CardVerb::Probe => 0,
            CardVerb::Maneuver => 0,
            CardVerb::Fortify => 0,
            CardVerb::Disrupt => 0,
            CardVerb::Mark => 0,
            CardVerb::Inspire => 0,
            CardVerb::Bolster => 0,
            CardVerb::Evasive => 0,
            CardVerb::Guard => 0,
            CardVerb::Noop => 0,
        };
        if damage > 0 {
            self.integrity_enemy = self.integrity_enemy.saturating_sub(damage);
            self.push_log(format!(
                "R{}: You played {} ({}) — enemy -{}.hp",
                self.round,
                card.name,
                card.verb.label(),
                damage
            ));
        } else if matches!(card.verb, CardVerb::Withdraw) {
            self.push_log(format!(
                "R{}: You played {} — auto-retreat.",
                self.round, card.name
            ));
            self.finished = true;
            self.outcome = Some("Retreated (Warp Retreat card)");
            return;
        } else if matches!(card.verb, CardVerb::Probe) {
            self.enemy_revealed = true;
            self.push_log(format!(
                "R{}: You played {} — enemy hand revealed.",
                self.round, card.name
            ));
        } else {
            self.push_log(format!(
                "R{}: You played {} ({})",
                self.round,
                card.name,
                card.verb.label()
            ));
        }
        self.hand_self.remove(card_index);
        self.maybe_advance();
    }

    /// Apply the enemy's chosen card (the deterministic AI fixture).
    /// In the real implementation this is computed by `ai_pick_card`.
    pub fn play_enemy(&mut self) {
        if self.finished {
            return;
        }
        let Some(card_id) = self.hand_enemy.first().copied() else {
            return;
        };
        let card = card_by_id(card_id);
        let damage = match card.verb {
            CardVerb::Strike => 14,
            CardVerb::Salvo => 20,
            CardVerb::Overcharge => 22,
            _ => 5,
        };
        if damage > 0 {
            self.integrity_self = self.integrity_self.saturating_sub(damage);
            self.push_log(format!(
                "R{}: Enemy played {} ({}) — you -{}.hp",
                self.round,
                card.name,
                card.verb.label(),
                damage
            ));
        } else {
            self.push_log(format!(
                "R{}: Enemy played {} ({})",
                self.round,
                card.name,
                card.verb.label()
            ));
        }
        self.hand_enemy.remove(0);
    }

    fn maybe_advance(&mut self) {
        if self.integrity_enemy == 0 {
            self.finished = true;
            self.outcome = Some("Victory — enemy annihilated");
            return;
        }
        if self.integrity_self == 0 {
            self.finished = true;
            self.outcome = Some("Defeat — fleet destroyed");
            return;
        }
        // Enemy always plays a card in response during the same round.
        self.play_enemy();
        if self.finished {
            return;
        }
        if self.hand_self.is_empty() && self.hand_enemy.is_empty() || self.round >= MAX_ROUNDS {
            self.finished = true;
            if self.integrity_self > self.integrity_enemy {
                self.outcome = Some("Victory — higher integrity");
            } else if self.integrity_enemy > self.integrity_self {
                self.outcome = Some("Defeat — higher integrity");
            } else {
                self.outcome = Some("Draw — tiebreaker goes to defender");
            }
            return;
        }
        if !self.hand_self.is_empty() && !self.hand_enemy.is_empty() {
            self.round = (self.round + 1).min(MAX_ROUNDS);
            self.push_log(format!("--- Round {} ---", self.round));
        }
    }

    /// Handle a key event.  Returns true when the screen should close
    /// (Esc on the main view, or any key after the battle has ended).
    pub fn handle_key(&mut self, key: KeyEvent) -> bool {
        // Help overlay captures the next key to dismiss it.
        if self.show_help {
            self.show_help = false;
            return false;
        }

        if self.finished {
            // Any key closes after the outcome is shown.
            return true;
        }

        match key.code {
            KeyCode::Esc => true,
            KeyCode::Tab => {
                self.side = self.side.toggle();
                false
            }
            KeyCode::Char('?') => {
                self.show_help = true;
                false
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                self.push_log(
                    "R{}: You retreated (free command, burns round)."
                        .replace("R{}", &format!("R{}", self.round)),
                );
                self.finished = true;
                self.outcome = Some("Retreated (free command)");
                self.integrity_self = (self.integrity_self * 25) / 100;
                true
            }
            KeyCode::Char('1') => {
                self.play_self(0);
                false
            }
            KeyCode::Char('2') => {
                self.play_self(1);
                false
            }
            KeyCode::Char('3') => {
                self.play_self(2);
                false
            }
            KeyCode::Char('4') => {
                self.play_self(3);
                false
            }
            KeyCode::Char('5') => {
                self.play_self(4);
                false
            }
            _ => false,
        }
    }
}

/// Render the mock into the given area as a centered overlay.
pub fn render_battle_mock(frame: &mut Frame, area: Rect, state: &MockBattleState) {
    let popup = centered_rect(86, 80, area);
    frame.render_widget(Clear, popup);

    let title = format!(
        " Battle v3 (mock) — Round {}/{} — {} vs {} ",
        state.round, MAX_ROUNDS, "Escort Frigate", "Missile Frigate"
    );
    let block = Block::default()
        .title(Line::from(Span::styled(title, Theme::title_style())))
        .borders(Borders::ALL)
        .border_style(Theme::focused_border_style())
        .style(Theme::default_style());
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    if state.show_help {
        render_help_inner(frame, inner);
        return;
    }

    if state.finished {
        render_finished_inner(frame, inner, state);
        return;
    }

    // Layout: hands (top) | integrity + log (mid) | footer (bottom).
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(10), // hands
            Constraint::Min(4),     // integrity + log
            Constraint::Length(2),  // footer
        ])
        .split(inner);

    render_hands(frame, rows[0], state);
    render_mid(frame, rows[1], state);
    render_footer(frame, rows[2]);
}

fn render_hands(frame: &mut Frame, area: Rect, state: &MockBattleState) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // Self hand
    let mut self_lines: Vec<Line> = Vec::new();
    self_lines.push(Line::from(Span::styled(
        "Your hand (Ashveran Compact)",
        Theme::title_style(),
    )));
    self_lines.push(Line::from(Span::styled(
        "─".repeat(cols[0].width.saturating_sub(2) as usize),
        Theme::dim_border_style(),
    )));
    for (i, id) in state.hand_self.iter().enumerate() {
        let card = card_by_id(*id);
        let label = format!(
            "  {} {}{} {:<22}  {:<10}  {}",
            i + 1,
            match state.side {
                MockSide::Self_ => "▸",
                MockSide::Enemy => " ",
            },
            glyph_for_verb(card.verb),
            card.name,
            card.verb.label(),
            card.doctrine
        );
        self_lines.push(Line::from(Span::styled(label, Theme::text_primary_style())));
    }
    if state.hand_self.is_empty() {
        self_lines.push(Line::from(Span::styled(
            "  (no cards left)",
            Theme::muted_style(),
        )));
    }
    let self_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Theme::focused_border_style())
        .style(Theme::default_style());
    let self_p = Paragraph::new(self_lines)
        .block(self_block)
        .style(Theme::default_style());
    frame.render_widget(self_p, cols[0]);

    // Enemy hand
    let mut enemy_lines: Vec<Line> = Vec::new();
    enemy_lines.push(Line::from(Span::styled(
        "Enemy hand (unknown)",
        Theme::title_style(),
    )));
    enemy_lines.push(Line::from(Span::styled(
        "─".repeat(cols[1].width.saturating_sub(2) as usize),
        Theme::dim_border_style(),
    )));
    let enemy_visible = state.enemy_revealed;
    for (i, id) in state.hand_enemy.iter().enumerate() {
        let label = if enemy_visible {
            let card = card_by_id(*id);
            format!(
                "  {} {}{} {:<22}  {:<10}  {}",
                i + 1,
                match state.side {
                    MockSide::Enemy => "▸",
                    MockSide::Self_ => " ",
                },
                glyph_for_verb(card.verb),
                card.name,
                card.verb.label(),
                card.doctrine
            )
        } else {
            format!(
                "  {} {}  ?  {:<10}  ?",
                i + 1,
                match state.side {
                    MockSide::Enemy => "▸",
                    MockSide::Self_ => " ",
                },
                "?"
            )
        };
        enemy_lines.push(Line::from(Span::styled(
            label,
            Theme::text_secondary_style(),
        )));
    }
    if state.hand_enemy.is_empty() {
        enemy_lines.push(Line::from(Span::styled(
            "  (no cards left)",
            Theme::muted_style(),
        )));
    }
    let enemy_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Theme::dim_border_style())
        .style(Theme::default_style());
    let enemy_p = Paragraph::new(enemy_lines)
        .block(enemy_block)
        .style(Theme::default_style());
    frame.render_widget(enemy_p, cols[1]);
}

fn render_mid(frame: &mut Frame, area: Rect, state: &MockBattleState) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(2)])
        .split(area);

    // Integrity bars
    let bar = format!(
        "Integrity  YOU [{:>3}/100]   ENEMY [{:>3}/100]",
        state.integrity_self, state.integrity_enemy
    );
    let integrity_p = Paragraph::new(Line::from(Span::styled(bar, Theme::header_style())))
        .style(Theme::default_style());
    frame.render_widget(integrity_p, rows[0]);

    // Round log
    let mut log_lines: Vec<Line> = Vec::new();
    for entry in state.log.iter() {
        log_lines.push(Line::from(Span::styled(
            entry.clone(),
            Theme::text_primary_style(),
        )));
    }
    if log_lines.is_empty() {
        log_lines.push(Line::from(Span::styled(
            "(no events yet)",
            Theme::muted_style(),
        )));
    }
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

fn render_footer(frame: &mut Frame, area: Rect) {
    let line = Line::from(vec![
        Span::styled(" [1-5]Play ", Theme::accent_style()),
        Span::styled(" [Tab]Side ", Theme::accent_style()),
        Span::styled(" [r]Retreat ", Theme::accent_style()),
        Span::styled(" [?]Help ", Theme::accent_style()),
        Span::styled(" [Esc]Close ", Theme::accent_style()),
    ]);
    let p = Paragraph::new(line)
        .alignment(ratatui::layout::Alignment::Center)
        .style(Theme::default_style());
    frame.render_widget(p, area);
}

fn render_finished_inner(frame: &mut Frame, area: Rect, state: &MockBattleState) {
    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Battle finished",
        Theme::title_style(),
    )));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        format!("Outcome: {}", state.outcome.unwrap_or("Unknown")),
        Theme::header_style(),
    )));
    lines.push(Line::from(Span::styled(
        format!(
            "Final integrity — You: {}  Enemy: {}",
            state.integrity_self, state.integrity_enemy
        ),
        Theme::text_secondary_style(),
    )));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled("Round log:", Theme::title_style())));
    for entry in state.log.iter() {
        lines.push(Line::from(Span::styled(
            format!("  {entry}"),
            Theme::text_primary_style(),
        )));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Press any key to close.",
        Theme::muted_style(),
    )));
    let p = Paragraph::new(lines)
        .style(Theme::default_style())
        .wrap(Wrap { trim: false });
    frame.render_widget(p, area);
}

fn render_help_inner(frame: &mut Frame, area: Rect) {
    let lines = vec![
        Line::from(Span::styled("Battle v3 — keys", Theme::title_style())),
        Line::from(""),
        Line::from(Span::styled(
            "  1  2  3  4  5   play card N from your hand",
            Theme::text_primary_style(),
        )),
        Line::from(Span::styled(
            "  Tab              toggle side view (your hand / enemy hand)",
            Theme::text_primary_style(),
        )),
        Line::from(Span::styled(
            "  r                retreat (free command, burns round)",
            Theme::text_primary_style(),
        )),
        Line::from(Span::styled(
            "  ?                this help overlay (any key dismisses)",
            Theme::text_primary_style(),
        )),
        Line::from(Span::styled(
            "  Esc              close mock",
            Theme::text_primary_style(),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Enemy hand is hidden until you play a Probe card.",
            Theme::muted_style(),
        )),
        Line::from(Span::styled(
            "Integrity bars deplete per damage. 0 integrity = fleet destroyed.",
            Theme::muted_style(),
        )),
    ];
    let p = Paragraph::new(lines)
        .style(Theme::default_style())
        .wrap(Wrap { trim: false });
    frame.render_widget(p, area);
}

fn glyph_for_verb(verb: CardVerb) -> &'static str {
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

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{Terminal, backend::TestBackend};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, crossterm::event::KeyModifiers::NONE)
    }

    #[test]
    fn pool_has_expected_count() {
        assert_eq!(POOL.len(), 23, "v1 pool must hold 23 unique cards");
    }

    #[test]
    fn pool_ids_are_unique() {
        let mut ids: Vec<u16> = POOL.iter().map(|c| c.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), POOL.len(), "card IDs must be unique");
    }

    #[test]
    fn hold_fire_is_distinct_from_pool() {
        assert_eq!(HOLD_FIRE.id, 0);
        assert!(POOL.iter().all(|c| c.id != HOLD_FIRE.id));
    }

    #[test]
    fn card_by_id_finds_pool_and_fallback() {
        assert_eq!(card_by_id(1).name, "Kinetic Salvo");
        assert_eq!(card_by_id(23).name, "Bloom Shield");
        assert_eq!(card_by_id(0).name, "Hold Fire");
        assert_eq!(card_by_id(999).name, "Hold Fire");
    }

    #[test]
    fn new_state_is_round_one_with_full_integrity() {
        let state = MockBattleState::new();
        assert_eq!(state.round, 1);
        assert_eq!(state.integrity_self, 100);
        assert_eq!(state.integrity_enemy, 100);
        assert_eq!(state.hand_self.len(), HAND_SIZE);
        assert_eq!(state.hand_enemy.len(), HAND_SIZE);
        assert!(!state.finished);
        assert!(!state.enemy_revealed);
        assert_eq!(state.side, MockSide::Self_);
    }

    #[test]
    fn tab_toggles_side() {
        let mut state = MockBattleState::new();
        assert!(!state.handle_key(key(KeyCode::Tab)));
        assert_eq!(state.side, MockSide::Enemy);
        assert!(!state.handle_key(key(KeyCode::Tab)));
        assert_eq!(state.side, MockSide::Self_);
    }

    #[test]
    fn help_overlay_dismisses_on_next_key() {
        let mut state = MockBattleState::new();
        state.show_help = true;
        assert!(!state.handle_key(key(KeyCode::Char('x'))));
        assert!(!state.show_help);
    }

    #[test]
    fn esc_closes_active_battle() {
        let mut state = MockBattleState::new();
        assert!(state.handle_key(key(KeyCode::Esc)));
    }

    #[test]
    fn playing_strike_reduces_enemy_integrity() {
        let mut state = MockBattleState::new();
        // Hand slot 0 is Ablative Hull (Guard). Pick a real Strike via direct
        // hand assignment for determinism.
        state.hand_self = vec![1, 2, 3, 4, 5];
        let before = state.integrity_enemy;
        state.play_self(0);
        assert!(state.integrity_enemy < before);
        assert_eq!(state.hand_self.len(), 4);
    }

    #[test]
    fn playing_probe_reveals_enemy_hand() {
        let mut state = MockBattleState::new();
        state.hand_self = vec![8, 2, 3, 4, 5]; // Sensor Sweep
        state.play_self(0);
        assert!(state.enemy_revealed);
    }

    #[test]
    fn withdraw_card_finishes_battle_immediately() {
        let mut state = MockBattleState::new();
        state.hand_self = vec![12, 2, 3, 4, 5]; // Warp Retreat
        state.play_self(0);
        assert!(state.finished);
        assert!(state.outcome.is_some());
    }

    #[test]
    fn out_of_range_index_is_noop() {
        let mut state = MockBattleState::new();
        let before = state.integrity_enemy;
        state.play_self(99);
        assert_eq!(state.integrity_enemy, before);
        assert_eq!(state.hand_self.len(), HAND_SIZE);
    }

    #[test]
    fn free_retreat_command_finishes_battle() {
        let mut state = MockBattleState::new();
        let before = state.integrity_self;
        assert!(state.handle_key(key(KeyCode::Char('r'))));
        assert!(state.finished);
        assert!(state.integrity_self < before);
    }

    #[test]
    fn finished_battle_closes_on_any_key() {
        let mut state = MockBattleState::new();
        state.finished = true;
        state.outcome = Some("Test");
        assert!(state.handle_key(key(KeyCode::Char('q'))));
    }

    #[test]
    fn render_does_not_panic_in_minimal_area() {
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = MockBattleState::new();
        terminal
            .draw(|frame| render_battle_mock(frame, frame.area(), &state))
            .unwrap();
    }

    #[test]
    fn render_with_help_does_not_panic() {
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = MockBattleState::new();
        state.show_help = true;
        terminal
            .draw(|frame| render_battle_mock(frame, frame.area(), &state))
            .unwrap();
    }

    #[test]
    fn render_finished_battle_does_not_panic() {
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = MockBattleState::new();
        state.finished = true;
        state.outcome = Some("Victory");
        state.integrity_self = 40;
        state.integrity_enemy = 0;
        terminal
            .draw(|frame| render_battle_mock(frame, frame.area(), &state))
            .unwrap();
    }

    #[test]
    fn full_round_progression_reaches_finished() {
        let mut state = MockBattleState::new();
        // Auto-play the entire hand.
        for _ in 0..HAND_SIZE {
            if state.finished {
                break;
            }
            state.play_self(0);
        }
        assert!(state.finished, "battle must finish within 5 cards");
    }

    #[test]
    fn reset_restores_initial_state() {
        let mut state = MockBattleState::new();
        state.play_self(0);
        state.reset();
        assert_eq!(state.round, 1);
        assert_eq!(state.integrity_self, 100);
        assert_eq!(state.integrity_enemy, 100);
        assert_eq!(state.hand_self.len(), HAND_SIZE);
        assert!(!state.finished);
    }
}
