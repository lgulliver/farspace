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
//! in-game bindings except on SectorMap where M means "move fleet"). Close
//! with `Esc`. The mock is otherwise inert — it does not touch the engine
//! or save data.

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

/// Static card descriptor.  Mirrors the v1 pool from the design doc and
/// carries the full text shown in the detail panel.
#[derive(Debug, Clone, Copy)]
pub struct MockCard {
    pub id: u16,
    pub name: &'static str,
    pub verb: CardVerb,
    pub doctrine: &'static str,
    pub source: &'static str,
    /// One-line effect description shown in the detail panel and in tight
    /// hand rows.  Should be a single self-contained sentence.
    pub effect: &'static str,
    /// What the card targets (enemy fleet, self, queued card, post-battle).
    pub target: &'static str,
    /// Numeric magnitude where applicable (e.g. damage, healing, percent).
    /// `None` for variable-effect cards.
    pub magnitude: Option<&'static str>,
    /// Synergy note describing which verbs or cards it combines with.
    pub synergies: &'static str,
    /// Free-form rules note.  Used for edge cases (self-damage, post-battle
    /// only, etc.).
    pub notes: &'static str,
}

/// Static v1 pool.  Counts to 23 unique cards + 1 `Hold Fire` fallback.
pub static POOL: &[MockCard] = &[
    MockCard {
        id: 1,
        name: "Kinetic Salvo",
        verb: CardVerb::Strike,
        doctrine: "Militarist (+2)",
        source: "Hull — Escort Frigate / Missile Frigate / Destroyer / Patrol Corvette",
        effect: "Deal direct damage to the enemy fleet.",
        target: "Enemy fleet",
        magnitude: Some("18 hp"),
        synergies: "Stacks with Mark (+25% dmg) and Salvo.",
        notes: "Baseline kinetic armament. Reliable, no drawbacks.",
    },
    MockCard {
        id: 2,
        name: "Ablative Hull",
        verb: CardVerb::Guard,
        doctrine: "Isolationist (+2)",
        source: "Component — Reinforced Plating",
        effect: "Reduce damage taken this round by your defense value.",
        target: "Self",
        magnitude: Some("def × 1.0"),
        synergies: "Stacks additively with Fortify and Evasive.",
        notes: "Passive defensive layer; always useful.",
    },
    MockCard {
        id: 3,
        name: "Phased Shield",
        verb: CardVerb::Guard,
        doctrine: "Isolationist (+2)",
        source: "Component — Shield Matrix",
        effect: "Guard this round plus absorb up to 1 hp on the next round.",
        target: "Self",
        magnitude: Some("def × 1.0 + 1 hp buffer"),
        synergies: "Absorb persists into the next round if unused.",
        notes: "Trade plating for a delayed-damage buffer.",
    },
    MockCard {
        id: 4,
        name: "CIWS Grid",
        verb: CardVerb::Disrupt,
        doctrine: "—",
        source: "Component — Point Defense Grid",
        effect: "Cancel one enemy card queued for this round.",
        target: "Enemy queued card",
        magnitude: Some("1 card cancelled"),
        synergies: "Best used to nullify a Strike or Salvo.",
        notes: "Resolves before the enemy card's effect fires.",
    },
    MockCard {
        id: 5,
        name: "Burn Maneuver",
        verb: CardVerb::Evasive,
        doctrine: "Explorer (+2)",
        source: "Component — Ion Drive",
        effect: "Reduce incoming damage this round by 50%.",
        target: "Self",
        magnitude: Some("incoming × 0.5"),
        synergies: "Stacks with Guard and Fortify (multiplicative).",
        notes: "Best when enemy has a high-damage round queued.",
    },
    MockCard {
        id: 6,
        name: "Drift Burn",
        verb: CardVerb::Maneuver,
        doctrine: "Explorer (+2)",
        source: "Hull — Scout / Fast Scout",
        effect: "Gain +1 initiative this round; your card resolves first.",
        target: "Self",
        magnitude: Some("+1 initiative"),
        synergies: "Pairs with Disrupt or pre-emptive Guard.",
        notes: "Tempo card. Useful when initiative matters.",
    },
    MockCard {
        id: 7,
        name: "Targeting Lock",
        verb: CardVerb::Mark,
        doctrine: "Militarist (+1)",
        source: "Component — Targeting Suite",
        effect: "Buff: your next Strike card this battle deals +25% damage.",
        target: "Self (queued buff)",
        magnitude: Some("+25% next Strike"),
        synergies: "Apply before Strike, Salvo, or Overcharge.",
        notes: "Buff persists until consumed or battle ends.",
    },
    MockCard {
        id: 8,
        name: "Sensor Sweep",
        verb: CardVerb::Probe,
        doctrine: "Explorer (+1)",
        source: "Component — Long-Range Sensors",
        effect: "Reveal the enemy hand (names, verbs, doctrine).",
        target: "Enemy info",
        magnitude: Some("1 reveal"),
        synergies: "Stacks — repeat Probes reveal nothing new but log it.",
        notes: "Reveal persists for the rest of the battle.",
    },
    MockCard {
        id: 9,
        name: "Orbital Bombardment",
        verb: CardVerb::Salvo,
        doctrine: "Militarist (+3)",
        source: "Hull — Destroyer",
        effect: "Deal heavy damage that persists across remaining rounds.",
        target: "Enemy fleet",
        magnitude: Some("24 hp (×atk all rounds)"),
        synergies: "Pairs with Mark (+25%) and Overcharge (×1.5).",
        notes: "Best opener against high-HP enemy fleets.",
    },
    MockCard {
        id: 10,
        name: "Defensive Screen",
        verb: CardVerb::Fortify,
        doctrine: "Isolationist (+1)",
        source: "Hull — Escort Frigate / Patrol Corvette",
        effect: "Add 50% to your defense multiplier this round.",
        target: "Self",
        magnitude: Some("def × 1.5"),
        synergies: "Stacks additively with Guard.",
        notes: "Bigger bump than Ablative Hull but limited to one round.",
    },
    MockCard {
        id: 11,
        name: "Troop Drop",
        verb: CardVerb::Bolster,
        doctrine: "Militarist (+2)",
        source: "Component — Troop Bays / Hull — Troop Transport",
        effect: "Add invasion strength for the post-battle colony capture.",
        target: "Enemy colony (post-battle)",
        magnitude: Some("+5 invasion"),
        synergies: "Stacks with other Bolster cards in a multi-card hand.",
        notes: "Only applies if you win the engagement.",
    },
    MockCard {
        id: 12,
        name: "Warp Retreat",
        verb: CardVerb::Withdraw,
        doctrine: "Explorer (+2), Merchant (+1)",
        source: "Tech — Rapid Transit Drives",
        effect: "Auto-retreat, preserving 50% of current integrity.",
        target: "Self",
        magnitude: Some("retreat at 50% integrity"),
        synergies: "Counts as a card play; preserves your turn slot.",
        notes: "Use when outmatched; losing is worse than retreating.",
    },
    MockCard {
        id: 13,
        name: "Ordnance Overcharge",
        verb: CardVerb::Overcharge,
        doctrine: "Militarist (+2)",
        source: "Tech — Long-Range Strike Doctrine",
        effect: "Deal 28 hp damage, but take 1 hp self-damage.",
        target: "Enemy fleet (self-damage)",
        magnitude: Some("28 hp enemy / 1 hp self"),
        synergies: "Combines with Mark for +25% (self-damage unchanged).",
        notes: "Ignores Evasive. High risk, high reward.",
    },
    MockCard {
        id: 14,
        name: "Formation Rally",
        verb: CardVerb::Inspire,
        doctrine: "Unity (+3), Militarist (+1)",
        source: "Tech — Battle Doctrine",
        effect: "Refill 1 hand slot with the top of your deck mid-battle.",
        target: "Self (hand)",
        magnitude: Some("+1 card"),
        synergies: "Refill is deterministic — no RNG.",
        notes: "Breaks the strict 5-round cap when used late.",
    },
    MockCard {
        id: 15,
        name: "Surveyor's Gambit",
        verb: CardVerb::Probe,
        doctrine: "Explorer (+3), Merchant (+1)",
        source: "Hull — Science Vessel / Survey Cutter",
        effect: "Probe (reveal enemy hand) combined with Evasive (50% dmg cut).",
        target: "Self + enemy info",
        magnitude: Some("Probe + Evasive"),
        synergies: "Best opener against unknown enemy loadouts.",
        notes: "Rare dual-effect card from survey hulls.",
    },
    MockCard {
        id: 16,
        name: "Coercive Mandate",
        verb: CardVerb::Strike,
        doctrine: "Militarist",
        source: "Faction signature — Vorath Dominion",
        effect: "Strike plus bleed: enemy cards next round cost 1 hp each.",
        target: "Enemy fleet",
        magnitude: Some("14 hp + 1 hp/round bleed"),
        synergies: "Pairs with multi-round enemy hands.",
        notes: "Vorath signature. Pressure over precision.",
    },
    MockCard {
        id: 17,
        name: "Siege Doctrine",
        verb: CardVerb::Strike,
        doctrine: "Militarist + Imperial",
        source: "Faction signature — Terran Dominion",
        effect: "Strike plus Bolster: damage plus +3 post-battle invasion.",
        target: "Enemy fleet + post-battle colony",
        magnitude: Some("16 hp + +3 invasion"),
        synergies: "Combines damage and capture in one card.",
        notes: "Terran Dominion signature. Siege-oriented.",
    },
    MockCard {
        id: 18,
        name: "Industrial Juggernaut",
        verb: CardVerb::Guard,
        doctrine: "Industrialist",
        source: "Faction signature — Ashveran Compact",
        effect: "Guard plus end-of-round heal of 2 hp.",
        target: "Self",
        magnitude: Some("def × 1.0 + 2 hp heal"),
        synergies: "Heal persists across rounds until you stop guarding.",
        notes: "Ashveran signature. Sustainable defense.",
    },
    MockCard {
        id: 19,
        name: "Algorithmic Defense",
        verb: CardVerb::Guard,
        doctrine: "Technologist",
        source: "Faction signature — Elarith Confluence",
        effect: "Guard plus Probe: defend and reveal the enemy hand.",
        target: "Self + enemy info",
        magnitude: Some("def × 1.0 + reveal"),
        synergies: "Defensive Probe. Information while you turtle.",
        notes: "Elarith signature. Intel + armor in one card.",
    },
    MockCard {
        id: 20,
        name: "Pathfinder's Wager",
        verb: CardVerb::Evasive,
        doctrine: "Explorer",
        source: "Faction signature — Luminal Traverse",
        effect: "Probe plus Evasive: reveal enemy and cut incoming 50%.",
        target: "Self + enemy info",
        magnitude: Some("Probe + incoming × 0.5"),
        synergies: "Offensive intel: see the threat and dodge half of it.",
        notes: "Luminal signature. Risky but rewarding opener.",
    },
    MockCard {
        id: 21,
        name: "Council of Voices",
        verb: CardVerb::Inspire,
        doctrine: "Unity + Explorer",
        source: "Faction signature — Terran Concord",
        effect: "Inspire plus Guard: refill 1 hand and reduce incoming dmg.",
        target: "Self (hand) + Self (defense)",
        magnitude: Some("+1 card + def × 0.5"),
        synergies: "Plays the rally and defense in one turn.",
        notes: "Terran Concord signature. Cooperative, low-aggression.",
    },
    MockCard {
        id: 22,
        name: "Trade Barge Stand",
        verb: CardVerb::Guard,
        doctrine: "Merchant",
        source: "Faction signature — Thalori Exchange",
        effect: "Cheap Guard with +25% defense bonus.",
        target: "Self",
        magnitude: Some("def × 1.25"),
        synergies: "Pure defense. Frees other cards for offense.",
        notes: "Thalori signature. Trade-route protection.",
    },
    MockCard {
        id: 23,
        name: "Bloom Shield",
        verb: CardVerb::Guard,
        doctrine: "Biologist",
        source: "Faction signature — Sylvaran Accord",
        effect: "Guard plus regen: heal 1 hp at round start for 2 rounds.",
        target: "Self",
        magnitude: Some("def × 1.0 + 1 hp/round × 2"),
        synergies: "Sustained healing pairs with longer battles.",
        notes: "Sylvaran signature. Slow but persistent recovery.",
    },
];

/// Last-resort filler card.
pub const HOLD_FIRE: MockCard = MockCard {
    id: 0,
    name: "Hold Fire",
    verb: CardVerb::Noop,
    doctrine: "—",
    source: "Fallback pad",
    effect: "Burn the round. No damage taken or dealt.",
    target: "(none)",
    magnitude: None,
    synergies: "Filler when a hand cannot reach 5 cards.",
    notes: "Used as the universal pad for sub-5 hands.",
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
/// integrity gauges, the side toggle, the cursors, the log buffer, and the
/// help flag.
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
    pub cursor_self: usize,
    pub cursor_enemy: usize,
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
        let hand_self = vec![1, 2, 9, 14, 18];
        let hand_enemy = vec![1, 9, 13, 7, 14];
        Self {
            round: 1,
            integrity_self: 100,
            integrity_enemy: 100,
            hand_self,
            hand_enemy,
            cursor_self: 0,
            cursor_enemy: 0,
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

    /// Currently focused card for the active side, or `None` if the
    /// hand is empty.
    pub fn focused_card(&self) -> Option<&'static MockCard> {
        let hand = match self.side {
            MockSide::Self_ => &self.hand_self,
            MockSide::Enemy => &self.hand_enemy,
        };
        let cursor = match self.side {
            MockSide::Self_ => self.cursor_self,
            MockSide::Enemy => self.cursor_enemy,
        };
        hand.get(cursor).map(|id| card_by_id(*id))
    }

    fn clamp_cursors(&mut self) {
        if !self.hand_self.is_empty() {
            if self.cursor_self >= self.hand_self.len() {
                self.cursor_self = self.hand_self.len() - 1;
            }
        } else {
            self.cursor_self = 0;
        }
        if !self.hand_enemy.is_empty() {
            if self.cursor_enemy >= self.hand_enemy.len() {
                self.cursor_enemy = self.hand_enemy.len() - 1;
            }
        } else {
            self.cursor_enemy = 0;
        }
    }

    fn move_cursor(&mut self, delta: i32) {
        let (hand_len, cursor) = match self.side {
            MockSide::Self_ => (self.hand_self.len(), self.cursor_self),
            MockSide::Enemy => (self.hand_enemy.len(), self.cursor_enemy),
        };
        if hand_len == 0 {
            return;
        }
        let new = if delta >= 0 {
            (cursor + delta as usize).min(hand_len - 1)
        } else {
            cursor.saturating_sub((-delta) as usize)
        };
        match self.side {
            MockSide::Self_ => self.cursor_self = new,
            MockSide::Enemy => self.cursor_enemy = new,
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
            self.hand_self.remove(card_index);
            self.clamp_cursors();
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
        self.clamp_cursors();
        self.maybe_advance();
    }

    /// Play the card currently focused by the cursor on the active side.
    pub fn play_focused(&mut self) {
        if self.finished {
            return;
        }
        match self.side {
            MockSide::Self_ => {
                if self.cursor_self < self.hand_self.len() {
                    self.play_self(self.cursor_self);
                }
            }
            MockSide::Enemy => {
                // Mock enemy hand is read-only; ignore plays on the enemy side.
            }
        }
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
        self.clamp_cursors();
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
                self.push_log(format!(
                    "R{}: You retreated (free command, burns round).",
                    self.round
                ));
                self.finished = true;
                self.outcome = Some("Retreated (free command)");
                self.integrity_self = (self.integrity_self * 25) / 100;
                true
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.move_cursor(1);
                false
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.move_cursor(-1);
                false
            }
            KeyCode::Enter => {
                self.play_focused();
                false
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
    let popup = centered_rect(88, 84, area);
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

    // Layout: hands + detail (top) | integrity + log (mid) | footer (bottom).
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(14), // hands + detail
            Constraint::Min(4),     // integrity + log
            Constraint::Length(2),  // footer
        ])
        .split(inner);

    render_hands_and_details(frame, rows[0], state);
    render_mid(frame, rows[1], state);
    render_footer(frame, rows[2]);
}

fn render_hands_and_details(frame: &mut Frame, area: Rect, state: &MockBattleState) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(35), // self hand list
            Constraint::Percentage(35), // detail panel
            Constraint::Percentage(30), // enemy hand list
        ])
        .split(area);

    render_hand_list(
        frame,
        cols[0],
        "Your hand",
        MockSide::Self_,
        &state.hand_self,
        state.cursor_self,
        state.side == MockSide::Self_,
    );
    render_detail_panel(frame, cols[1], state);
    render_hand_list(
        frame,
        cols[2],
        "Enemy hand",
        MockSide::Enemy,
        &state.hand_enemy,
        state.cursor_enemy,
        state.side == MockSide::Enemy,
    );
}

fn render_hand_list(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    side: MockSide,
    hand: &[u16],
    cursor: usize,
    is_active_view: bool,
) {
    let title_owned = title.to_string();
    let block = Block::default()
        .title(Line::from(Span::styled(
            format!(" {title_owned} "),
            Theme::title_style(),
        )))
        .borders(Borders::ALL)
        .border_style(if is_active_view {
            Theme::focused_border_style()
        } else {
            Theme::dim_border_style()
        })
        .style(Theme::default_style());

    let visible = match side {
        MockSide::Self_ => true,
        MockSide::Enemy =>
        /* read by `is_active_view` only; hidden state
        is read separately by the detail panel; we still render
        hand rows but blank out names for the enemy when not
        revealed */
        {
            is_active_view
        }
    };

    let mut lines: Vec<Line> = Vec::new();
    if hand.is_empty() {
        lines.push(Line::from(Span::styled(
            "  (no cards left)",
            Theme::muted_style(),
        )));
    } else {
        for (i, id) in hand.iter().enumerate() {
            let is_cursor = i == cursor;
            let marker = if is_cursor && is_active_view {
                "▸"
            } else {
                " "
            };
            let label = if visible {
                let card = card_by_id(*id);
                let name = format!("{:<20}", truncate(card.name, 20));
                let verb = format!("{:<10}", card.verb.label());
                format!(
                    "  {} {} {} {}  {}  {}",
                    i + 1,
                    marker,
                    glyph_for_verb(card.verb),
                    name,
                    verb,
                    card.doctrine
                )
            } else {
                format!("  {} {} ?  (hidden)", i + 1, marker)
            };
            let style = if is_cursor && is_active_view {
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

fn render_detail_panel(frame: &mut Frame, area: Rect, state: &MockBattleState) {
    let block = Block::default()
        .title(Line::from(Span::styled(
            " Card detail ",
            Theme::title_style(),
        )))
        .borders(Borders::ALL)
        .border_style(Theme::dim_border_style())
        .style(Theme::default_style());

    let visible = match state.side {
        MockSide::Self_ => true,
        MockSide::Enemy => state.enemy_revealed,
    };
    let card = state.focused_card();

    let mut lines: Vec<Line> = Vec::new();
    if !visible {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Enemy hand not revealed.",
            Theme::muted_style(),
        )));
        lines.push(Line::from(Span::styled(
            "Play a Probe (Sensor Sweep, Surveyor's Gambit,",
            Theme::muted_style(),
        )));
        lines.push(Line::from(Span::styled(
            "Pathfinder's Wager, Algorithmic Defense) to reveal.",
            Theme::muted_style(),
        )));
    } else if let Some(card) = card {
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
            Span::styled(card.doctrine, Theme::text_primary_style()),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Source:    ", Theme::muted_style()),
            Span::styled(card.source, Theme::text_primary_style()),
        ]));
        lines.push(Line::from(vec![
            Span::styled("Target:    ", Theme::muted_style()),
            Span::styled(card.target, Theme::text_primary_style()),
        ]));
        if let Some(mag) = card.magnitude {
            lines.push(Line::from(vec![
                Span::styled("Magnitude: ", Theme::muted_style()),
                Span::styled(mag, Theme::text_primary_style()),
            ]));
        }
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled("Effect", Theme::title_style())));
        lines.push(Line::from(Span::styled(
            card.effect,
            Theme::text_primary_style(),
        )));
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled("Synergies", Theme::title_style())));
        lines.push(Line::from(Span::styled(
            card.synergies,
            Theme::text_primary_style(),
        )));
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled("Notes", Theme::title_style())));
        lines.push(Line::from(Span::styled(
            card.notes,
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
        Span::styled(" [Enter]Focused ", Theme::accent_style()),
        Span::styled(" [j/k]Nav ", Theme::accent_style()),
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
            "  Enter          play the focused card (j/k to move)",
            Theme::text_primary_style(),
        )),
        Line::from(Span::styled(
            "  j  k           move cursor down / up within the hand",
            Theme::text_primary_style(),
        )),
        Line::from(Span::styled(
            "  Tab            toggle side view (your hand / enemy hand)",
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
            "  Esc            close mock",
            Theme::text_primary_style(),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "The card detail panel on the right shows full text for the",
            Theme::muted_style(),
        )),
        Line::from(Span::styled(
            "focused card: verb, doctrine, source, target, magnitude,",
            Theme::muted_style(),
        )),
        Line::from(Span::styled(
            "effect, synergies, and notes. Use j/k to step through cards.",
            Theme::muted_style(),
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
    fn all_pool_cards_have_effect_text() {
        for card in POOL {
            assert!(!card.effect.is_empty(), "{} missing effect text", card.name);
            assert!(!card.target.is_empty(), "{} missing target text", card.name);
            assert!(
                !card.synergies.is_empty(),
                "{} missing synergies text",
                card.name
            );
            assert!(!card.notes.is_empty(), "{} missing notes text", card.name);
        }
    }

    #[test]
    fn damage_cards_have_magnitudes() {
        for card in POOL {
            if matches!(
                card.verb,
                CardVerb::Strike | CardVerb::Salvo | CardVerb::Overcharge | CardVerb::Bolster
            ) {
                assert!(
                    card.magnitude.is_some(),
                    "{} (verb {}) should have a magnitude",
                    card.name,
                    card.verb.label()
                );
            }
        }
    }

    #[test]
    fn new_state_is_round_one_with_full_integrity() {
        let state = MockBattleState::new();
        assert_eq!(state.round, 1);
        assert_eq!(state.integrity_self, 100);
        assert_eq!(state.integrity_enemy, 100);
        assert_eq!(state.hand_self.len(), HAND_SIZE);
        assert_eq!(state.hand_enemy.len(), HAND_SIZE);
        assert_eq!(state.cursor_self, 0);
        assert_eq!(state.cursor_enemy, 0);
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
    fn jk_navigates_cursor_within_bounds() {
        let mut state = MockBattleState::new();
        // Hand has 5 cards. Move down twice, up once.
        state.handle_key(key(KeyCode::Char('j')));
        state.handle_key(key(KeyCode::Char('j')));
        assert_eq!(state.cursor_self, 2);
        state.handle_key(key(KeyCode::Char('k')));
        assert_eq!(state.cursor_self, 1);
    }

    #[test]
    fn jk_caps_at_hand_end() {
        let mut state = MockBattleState::new();
        for _ in 0..10 {
            state.handle_key(key(KeyCode::Char('j')));
        }
        assert_eq!(state.cursor_self, HAND_SIZE - 1);
    }

    #[test]
    fn k_floors_at_zero() {
        let mut state = MockBattleState::new();
        for _ in 0..3 {
            state.handle_key(key(KeyCode::Char('k')));
        }
        assert_eq!(state.cursor_self, 0);
    }

    #[test]
    fn enter_plays_focused_card() {
        let mut state = MockBattleState::new();
        // Default hand is [1, 2, 9, 14, 18]. Cursor at 0 -> Kinetic Salvo.
        let before = state.integrity_enemy;
        state.handle_key(key(KeyCode::Enter));
        assert!(state.integrity_enemy < before, "focused Strike must damage");
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
    fn focused_card_returns_hand_entry() {
        let state = MockBattleState::new();
        let card = state.focused_card().expect("cursor at 0 has a card");
        assert_eq!(card.id, state.hand_self[0]);
    }

    #[test]
    fn focused_card_returns_none_when_hand_empty() {
        let mut state = MockBattleState::new();
        state.hand_self.clear();
        assert!(state.focused_card().is_none());
    }

    #[test]
    fn render_does_not_panic_in_minimal_area() {
        let backend = TestBackend::new(140, 50);
        let mut terminal = Terminal::new(backend).unwrap();
        let state = MockBattleState::new();
        terminal
            .draw(|frame| render_battle_mock(frame, frame.area(), &state))
            .unwrap();
    }

    #[test]
    fn render_with_help_does_not_panic() {
        let backend = TestBackend::new(140, 50);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = MockBattleState::new();
        state.show_help = true;
        terminal
            .draw(|frame| render_battle_mock(frame, frame.area(), &state))
            .unwrap();
    }

    #[test]
    fn render_with_enemy_view_does_not_panic() {
        let backend = TestBackend::new(140, 50);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut state = MockBattleState::new();
        state.side = MockSide::Enemy;
        terminal
            .draw(|frame| render_battle_mock(frame, frame.area(), &state))
            .unwrap();
    }

    #[test]
    fn render_finished_battle_does_not_panic() {
        let backend = TestBackend::new(140, 50);
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
        assert_eq!(state.cursor_self, 0);
        assert!(!state.finished);
    }

    #[test]
    fn cursor_clamps_after_card_removal() {
        let mut state = MockBattleState::new();
        state.cursor_self = HAND_SIZE - 1;
        state.play_self(HAND_SIZE - 1);
        assert!(state.cursor_self < state.hand_self.len());
    }
}
