//! Empire selection screen — choose your faction before configuring the galaxy.
//!
//! Two-panel layout:
//!   Left  (~35%)  — scrollable faction list with coloured symbol + name
//!   Right (~65%)  — faction detail: ASCII-art emblem, lore, traits, bonuses

use crate::components::render_footer;
use crate::layout::compose_layout;
use crate::screens::Screen;
use crate::theme::Theme;
use crate::AppState;
use game_core::{all_empire_definitions, EmpireDefinitionId};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
    Frame,
};

// ── Faction ASCII-art emblems ──────────────────────────────────────────────────
//
// Each emblem is 14 chars wide × 9 lines tall (not counting the surrounding
// border). Designed for monospace fonts; whitespace padding is intentional.
//
// Rules: original art only — no motifs copied from Master of Orion.

const EMBLEM_ASHVERAN: &[&str] = &[
    " ░░░░░░░░░░░░ ",
    " ░▓▓▓▓▓▓▓▓▓░ ",
    " ░▓  ═══  ▓░ ",
    " ░▓ ▓ ⚙ ▓ ▓░ ",
    " ░▓  ═══  ▓░ ",
    " ░▓▓▓▓▓▓▓▓▓░ ",
    " ░░░░░░░░░░░░ ",
    "  ▐██▌  ▐██▌ ",
    "  ████  ████ ",
];

const EMBLEM_LUMINAL: &[&str] = &[
    "  · ✦ · ✦ ·  ",
    " · ╱─────╲ · ",
    "·╱ · ─◎─ · ╲·",
    "│ ·  ╱|╲  · │",
    "·╲ ·  |  · ╱·",
    " · ╲─────╱ · ",
    "  · ✦ · ✦ ·  ",
    "             ",
    "             ",
];

const EMBLEM_SYLVARAN: &[&str] = &[
    "  ✦  ✿  ✦   ",
    " ✿ ✿✿✿✿✿ ✿  ",
    "✿✿✿✿✿✿✿✿✿✿✿ ",
    " ✿ ✿✿✿✿✿ ✿  ",
    "  · ✿✿✿✿✿·  ",
    "    ·✿✿✿·   ",
    "    ·· ▲ ·  ",
    "    ·▓▓▓▓·  ",
    "    ·████·  ",
];

const EMBLEM_THALORI: &[&str] = &[
    "     ╱╲      ",
    "    ╱◈◈╲     ",
    "   ╱◈◈◈◈╲    ",
    "  ╱◈◈◈◈◈◈╲   ",
    "  ╲◈◈◈◈◈◈╱   ",
    "   ╲◈◈◈◈╱    ",
    "    ╲◈◈╱     ",
    "     ╲╱      ",
    "             ",
];

const EMBLEM_VORATH: &[&str] = &[
    "  ▓▓▓▓▓▓▓▓▓  ",
    "  ▓ ╲   ╱ ▓  ",
    "  ▓  ╲ ╱  ▓  ",
    "  ▓  ─⚔─  ▓  ",
    "  ▓  ╱ ╲  ▓  ",
    "  ▓ ╱   ╲ ▓  ",
    "  ▓▓▓▓▓▓▓▓▓  ",
    "  ░░░░░░░░░  ",
    "             ",
];

const EMBLEM_ELARITH: &[&str] = &[
    "    · ⟁ ·    ",
    "   · ╱│╲ ·   ",
    "  · ╱ │ ╲ ·  ",
    " ·╱   │   ╲· ",
    " ·────┼────· ",
    " ·╲   │   ╱· ",
    "  · ╲ │ ╱ ·  ",
    "   · ╲│╱ ·   ",
    "    · ⟁ ·    ",
];

const EMBLEM_TERRAN_CONCORD: &[&str] = &[
    "   · \\ | / ·  ",
    "  · ─ ☼ ─ ·  ",
    "   · / | \\ ·  ",
    "    · · · ·   ",
    "  · ─ ─ ─ ·  ",
    " ·  ╔═════╗ ·",
    " ·  ║ ☼ ☼ ║ ·",
    " ·  ╚═════╝ ·",
    "             ",
];

const EMBLEM_TERRAN_DOMINION: &[&str] = &[
    "     ▲▲▲     ",
    "    ▲▲▲▲▲    ",
    "   ▲▲▲▲▲▲▲   ",
    "  ╔═══════╗  ",
    "  ║▓▓░░░▓▓║  ",
    "  ║▓░▲░▲░▓║  ",
    "  ║▓▓░░░▓▓║  ",
    "  ╚═══════╝  ",
    "             ",
];

const EMBLEMS: [&[&str]; 8] = [
    EMBLEM_ASHVERAN,
    EMBLEM_LUMINAL,
    EMBLEM_SYLVARAN,
    EMBLEM_THALORI,
    EMBLEM_VORATH,
    EMBLEM_ELARITH,
    EMBLEM_TERRAN_CONCORD,
    EMBLEM_TERRAN_DOMINION,
];

// ── Faction colour lookup (all 8 factions) ────────────────────────────────────

pub fn faction_accent(def_id: EmpireDefinitionId) -> Color {
    match def_id.0 {
        0 => Color::Rgb(214, 133, 63),  // Ashveran — orange
        1 => Color::Rgb(96, 193, 255),  // Luminal  — cyan
        2 => Color::Rgb(121, 212, 136), // Sylvaran — green
        3 => Color::Rgb(225, 176, 73),  // Thalori  — gold
        4 => Color::Rgb(217, 92, 92),   // Vorath   — red
        5 => Color::Rgb(184, 122, 255), // Elarith  — violet
        6 => Color::Rgb(100, 181, 246), // Terran Concord  — sky-blue
        _ => Color::Rgb(255, 140, 100), // Terran Dominion — rust-orange
    }
}

// ── Public render entry point ─────────────────────────────────────────────────

pub fn render_empire_select(frame: &mut Frame, area: Rect, app_state: &AppState) {
    let (_header_area, main_area, footer_area) = compose_layout(area);

    render_footer(
        frame,
        footer_area,
        &Screen::EmpireSelect,
        Some("↑↓ k/j  Browse   Enter  Confirm   Esc  Back"),
    );

    // Heading strip above the panels
    let [heading_area, panels_area] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Fill(1)])
        .areas(main_area);

    render_heading(frame, heading_area);

    // Two-column layout: list | detail
    let [list_area, detail_area] = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(33), Constraint::Percentage(67)])
        .areas(panels_area);

    render_faction_list(frame, list_area, app_state);
    render_faction_detail(frame, detail_area, app_state);
}

// ── Heading ────────────────────────────────────────────────────────────────────

fn render_heading(frame: &mut Frame, area: Rect) {
    let line = Line::from(vec![
        Span::styled("  ◈  CHOOSE YOUR FACTION", Theme::title_style()),
        Span::styled(
            "  —  your people. your doctrine. your galaxy.",
            Theme::muted_style(),
        ),
    ]);
    let p = Paragraph::new(line)
        .style(Theme::default_style())
        .alignment(Alignment::Left);
    frame.render_widget(p, area);
}

// ── Faction list panel (left) ─────────────────────────────────────────────────

fn render_faction_list(frame: &mut Frame, area: Rect, app_state: &AppState) {
    let all_defs = all_empire_definitions();
    let selected = app_state.new_game_setup.empire_cursor;

    let block = Block::default()
        .title(" FACTIONS ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Theme::focused_border_style());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Scroll to keep selection visible
    let visible_height = inner.height as usize;
    let scroll_offset = if selected >= visible_height {
        selected - visible_height + 1
    } else {
        0
    };

    let lines: Vec<Line> = all_defs
        .iter()
        .enumerate()
        .skip(scroll_offset)
        .take(visible_height)
        .map(|(i, def)| {
            let color = faction_accent(def.id);
            if i == selected {
                Line::from(vec![
                    Span::styled(
                        " ▶ ",
                        Style::default().fg(color).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!("{} {}", def.symbol, def.name),
                        Style::default()
                            .fg(Color::Black)
                            .bg(color)
                            .add_modifier(Modifier::BOLD),
                    ),
                ])
            } else {
                Line::from(vec![
                    Span::raw("   "),
                    Span::styled(format!("{} ", def.symbol), Style::default().fg(color)),
                    Span::styled(def.name, Theme::default_style()),
                ])
            }
        })
        .collect();

    let p = Paragraph::new(lines).style(Theme::default_style());
    frame.render_widget(p, inner);
}

// ── Faction detail panel (right) ─────────────────────────────────────────────

fn render_faction_detail(frame: &mut Frame, area: Rect, app_state: &AppState) {
    let all_defs = all_empire_definitions();
    let idx = app_state
        .new_game_setup
        .empire_cursor
        .min(all_defs.len().saturating_sub(1));
    let def = &all_defs[idx];
    let color = faction_accent(def.id);

    // Split detail: emblem on left, info on right
    let [emblem_area, info_area] = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(18), Constraint::Fill(1)])
        .areas(area);

    render_emblem_panel(frame, emblem_area, def.id.0 as usize, color, def.symbol);
    render_info_panel(frame, info_area, def, color);
}

fn render_emblem_panel(
    frame: &mut Frame,
    area: Rect,
    faction_idx: usize,
    color: Color,
    symbol: char,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(color));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let emblem = EMBLEMS.get(faction_idx).copied().unwrap_or(&[]);

    // Center the emblem vertically
    let total_lines = emblem.len();
    let v_pad = (inner.height as usize).saturating_sub(total_lines) / 2;

    let mut lines: Vec<Line> = Vec::new();

    for _ in 0..v_pad {
        lines.push(Line::from(""));
    }
    for row in emblem {
        lines.push(Line::from(Span::styled(
            *row,
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        )));
    }
    // Large symbol centred below art if space permits
    lines.push(Line::from(Span::styled(
        format!("    {symbol}"),
        Style::default().fg(color).add_modifier(Modifier::BOLD),
    )));

    let p = Paragraph::new(lines)
        .alignment(Alignment::Center)
        .style(Theme::default_style());
    frame.render_widget(p, inner);
}

fn render_info_panel(
    frame: &mut Frame,
    area: Rect,
    def: &game_core::EmpireDefinition,
    color: Color,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(color));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines: Vec<Line> = Vec::new();

    // Faction name
    lines.push(Line::from(Span::styled(
        format!(" {} {}", def.symbol, def.name.to_uppercase()),
        Style::default().fg(color).add_modifier(Modifier::BOLD),
    )));
    // Tone
    lines.push(Line::from(vec![
        Span::raw(" "),
        Span::styled(def.tone, Theme::muted_style()),
    ]));
    lines.push(Line::from(""));

    // Lore
    lines.push(Line::from(Span::styled(" ◆ LORE", Theme::title_style())));
    for chunk in word_wrap(
        def.short_description,
        inner.width.saturating_sub(2) as usize,
    ) {
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(chunk, Theme::default_style()),
        ]));
    }
    lines.push(Line::from(""));

    // Playstyle
    lines.push(Line::from(Span::styled(
        " ◆ PLAYSTYLE",
        Theme::title_style(),
    )));
    for chunk in word_wrap(
        def.playstyle_summary,
        inner.width.saturating_sub(2) as usize,
    ) {
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled(chunk, Theme::muted_style()),
        ]));
    }
    lines.push(Line::from(""));

    // Playstyle tags
    let tags: Vec<Span> = {
        let mut v = vec![Span::raw(" ")];
        for (i, tag) in def.playstyle.iter().enumerate() {
            if i > 0 {
                v.push(Span::styled("  ·  ", Theme::muted_style()));
            }
            v.push(Span::styled(
                tag.label(),
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            ));
        }
        v
    };
    lines.push(Line::from(tags));
    lines.push(Line::from(""));

    // Bonuses
    let bonuses = def.effect_summaries();
    if !bonuses.is_empty() {
        lines.push(Line::from(Span::styled(" ◆ BONUSES", Theme::title_style())));
        for bonus in &bonuses {
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled("+ ", Style::default().fg(Color::Green)),
                Span::styled(bonus.clone(), Theme::default_style()),
            ]));
        }
        lines.push(Line::from(""));
    }

    // Diplomacy flavour
    lines.push(Line::from(Span::styled(
        " ◆ FIRST CONTACT",
        Theme::title_style(),
    )));
    let first_contact_label = match def.diplomacy_profile.first_contact_status {
        game_core::RelationshipStatus::Neutral => "Neutral",
        game_core::RelationshipStatus::Tense => "Tense",
        game_core::RelationshipStatus::Hostile => "Hostile",
        game_core::RelationshipStatus::War => "War",
        game_core::RelationshipStatus::Contacted => "Contacted",
        game_core::RelationshipStatus::Unknown => "Unknown",
    };
    let (fc_color, fc_note) = match def.diplomacy_profile.first_contact_status {
        game_core::RelationshipStatus::Neutral | game_core::RelationshipStatus::Contacted => {
            (Color::Green, "Opens peacefully")
        }
        game_core::RelationshipStatus::Tense => (Color::Yellow, "Opens with tension"),
        _ => (Color::LightRed, "Opens as adversary"),
    };
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled(
            first_contact_label,
            Style::default().fg(fc_color).add_modifier(Modifier::BOLD),
        ),
        Span::styled(format!("  — {fc_note}"), Theme::muted_style()),
    ]));

    let p = Paragraph::new(lines)
        .style(Theme::default_style())
        .wrap(Wrap { trim: false });
    frame.render_widget(p, inner);
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Wrap `text` to lines of at most `width` characters, splitting on whitespace.
fn word_wrap(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![text.to_string()];
    }
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        let sep = usize::from(!current.is_empty());
        if !current.is_empty() && current.len() + sep + word.len() > width {
            lines.push(current.clone());
            current.clear();
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(word);
    }
    if !current.is_empty() {
        lines.push(current);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{backend::TestBackend, Terminal};

    fn render_to_buffer(width: u16, height: u16) -> ratatui::buffer::Buffer {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| render_empire_select(frame, frame.area(), &AppState::default()))
            .unwrap();
        terminal.backend().buffer().clone()
    }

    #[test]
    fn renders_without_panic_wide() {
        render_to_buffer(120, 35);
    }

    #[test]
    fn renders_without_panic_narrow() {
        render_to_buffer(60, 20);
    }

    #[test]
    fn emblem_table_complete() {
        // Every faction must have an emblem entry.
        let defs = all_empire_definitions();
        assert_eq!(
            EMBLEMS.len(),
            defs.len(),
            "EMBLEMS count must match faction count"
        );
    }

    #[test]
    fn faction_accent_covers_all_ids() {
        for def in all_empire_definitions() {
            let _ = faction_accent(def.id);
        }
    }

    #[test]
    fn word_wrap_empty() {
        assert_eq!(word_wrap("", 20), vec![String::new()]);
    }

    #[test]
    fn word_wrap_long_line() {
        let result = word_wrap("one two three four five", 10);
        // All output lines must be ≤ 10 chars
        for line in &result {
            assert!(line.len() <= 10, "line too long: {line:?}");
        }
    }
}
