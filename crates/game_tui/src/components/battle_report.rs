//! Battle report modal component

use crate::layout::centered_rect;
use crate::theme::Theme;
use crate::{glyphs::glyphs_for_mode, visual_mode::VisualMode};
use game_core::BattleReport;
use ratatui::{
    Frame,
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};
use std::{collections::VecDeque, ops::Range};

const MAX_VISIBLE_REPORTS: usize = 8;

pub fn render_battle_reports(
    frame: &mut Frame,
    area: Rect,
    reports: &VecDeque<BattleReport>,
    selected_index: usize,
    inspect_mode: bool,
    mode: VisualMode,
) {
    let popup_area = centered_rect(92, 88, area);
    let glyphs = glyphs_for_mode(mode);
    frame.render_widget(Clear, popup_area);

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(Span::styled(
        format!("Recent reports: {}", reports.len()),
        Theme::title_style(),
    )));
    lines.push(Line::from(Span::styled(
        glyphs.horizontal_rule.to_string().repeat(64),
        Theme::dim_border_style(),
    )));

    if reports.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "No battle reports recorded yet.",
            Theme::muted_style(),
        )));
    } else {
        let idx = selected_index.min(reports.len().saturating_sub(1));
        let selected = &reports[idx];
        for i in visible_report_window(reports.len(), idx) {
            let report = &reports[i];
            let marker = if i == idx {
                glyphs.list_selected.to_string()
            } else {
                " ".to_string()
            };
            let summary = format!(
                "{} T{} Sys {}  F{} vs F{}  {}",
                marker,
                report.turn,
                report.star.0,
                report.fleet_a.0,
                report.fleet_b.0,
                report.system_outcome
            );
            let style = if i == idx {
                Theme::highlight_style()
            } else {
                Theme::default_style()
            };
            lines.push(Line::from(Span::styled(summary, style)));
        }

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!(
                "Selected: {} [{} / {}] · {}",
                selected.system_outcome,
                selected.role_a.label(),
                selected.role_b.label(),
                selected.star.0
            ),
            Theme::title_style(),
        )));
        lines.push(Line::from(Span::styled(
            format!(
                "Doctrines: {} vs {} · Formations: {} / {}",
                selected.doctrine_a,
                selected.doctrine_b,
                selected.formation_a.label(),
                selected.formation_b.label()
            ),
            Theme::muted_style(),
        )));
        lines.push(Line::from(vec![
            Span::styled("Supply: ", Theme::muted_style()),
            Span::styled(
                selected.supply_a.label(),
                Theme::fleet_supply_style(selected.supply_a),
            ),
            Span::styled(" vs ", Theme::muted_style()),
            Span::styled(
                selected.supply_b.label(),
                Theme::fleet_supply_style(selected.supply_b),
            ),
        ]));
        lines.push(Line::from(Span::styled(
            format!(
                "Integrity: {}→{} | {}→{} · Retreat: {} / {}",
                selected.integrity_a_start,
                selected.integrity_a_end,
                selected.integrity_b_start,
                selected.integrity_b_end,
                if selected.fleet_a_retreated { "A" } else { "-" },
                if selected.fleet_b_retreated { "B" } else { "-" }
            ),
            Theme::accent_style(),
        )));

        if inspect_mode {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "Phase summary:",
                Theme::title_style(),
            )));
            for phase in &selected.phases {
                lines.push(Line::from(Span::styled(
                    format!(
                        "  {}: {} / {} — {}",
                        phase.phase.label(),
                        phase.pressure_a,
                        phase.pressure_b,
                        phase.note
                    ),
                    Theme::muted_style(),
                )));
            }
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        glyphs.horizontal_rule.to_string().repeat(64),
        Theme::dim_border_style(),
    )));
    lines.push(Line::from(Span::styled(
        if inspect_mode {
            "↑/↓ select  Enter back to list  Esc/B close  supply affects attack/defense/travel"
        } else {
            "↑/↓ select  Enter inspect battle  Esc/B close  supply affects attack/defense/travel"
        },
        Theme::muted_style(),
    )));

    let widget = Paragraph::new(lines)
        .block(
            Block::default()
                .title(" Battle Reports ")
                .borders(Borders::ALL)
                .border_style(Theme::focused_border_style())
                .style(Theme::default_style()),
        )
        .wrap(Wrap { trim: false });

    frame.render_widget(widget, popup_area);
}

fn visible_report_window(report_count: usize, selected_index: usize) -> Range<usize> {
    if report_count <= MAX_VISIBLE_REPORTS {
        return 0..report_count;
    }

    let selected = selected_index.min(report_count.saturating_sub(1));
    let mut start = selected.saturating_sub(MAX_VISIBLE_REPORTS / 2);
    if start + MAX_VISIBLE_REPORTS > report_count {
        start = report_count - MAX_VISIBLE_REPORTS;
    }
    start..(start + MAX_VISIBLE_REPORTS)
}

#[cfg(test)]
mod tests {
    use super::*;
    use game_core::{
        BattleReport, CombatPhase, CombatPhaseSummary, EmpireId, FleetFormation, FleetId,
        FleetKind, FleetRole, FleetSupplyState, StarId,
    };
    use ratatui::{Terminal, backend::TestBackend};
    use std::collections::VecDeque;

    #[test]
    fn visible_report_window_keeps_older_selected_report_visible() {
        let window = visible_report_window(20, 2);
        assert_eq!(window, 0..MAX_VISIBLE_REPORTS);
        assert!(window.contains(&2));
    }

    #[test]
    fn visible_report_window_tracks_newest_selection_at_tail() {
        let window = visible_report_window(20, 19);
        assert_eq!(window, 12..20);
        assert!(window.contains(&19));
    }

    #[test]
    fn render_battle_reports_shows_supply_states() {
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut reports = VecDeque::new();
        reports.push_back(BattleReport {
            report_id: 1,
            turn: 7,
            star: StarId(3),
            fleet_a: FleetId(10),
            fleet_b: FleetId(11),
            empire_a: EmpireId(1),
            empire_b: EmpireId(2),
            role_a: FleetRole::StrikeFleet,
            role_b: FleetRole::DefenseFleet,
            formation_a: FleetFormation::Balanced,
            formation_b: FleetFormation::Balanced,
            doctrine_a: "Pressure".to_string(),
            doctrine_b: "Reserve".to_string(),
            supply_a: FleetSupplyState::OutOfSupply,
            supply_b: FleetSupplyState::Supplied,
            kind_a: FleetKind::Destroyer,
            kind_b: FleetKind::EscortFrigate,
            ships_a: 3,
            ships_b: 2,
            integrity_a_start: 100,
            integrity_b_start: 100,
            integrity_a_end: 45,
            integrity_b_end: 0,
            fleet_a_destroyed: false,
            fleet_b_destroyed: true,
            fleet_a_retreated: false,
            fleet_b_retreated: true,
            phases: vec![CombatPhaseSummary {
                phase: CombatPhase::OpeningVolley,
                pressure_a: 8,
                pressure_b: 3,
                note: "Logistics pressure".to_string(),
            }],
            system_outcome: "Attacker victory".to_string(),
        });

        terminal
            .draw(|frame| {
                render_battle_reports(
                    frame,
                    frame.area(),
                    &reports,
                    0,
                    false,
                    VisualMode::default(),
                );
            })
            .unwrap();

        let buf = terminal.backend().buffer();
        let rendered: String = (0..40u16)
            .flat_map(|y| {
                (0..120u16).map(move |x| {
                    buf.cell((x, y))
                        .and_then(|c| c.symbol().chars().next())
                        .unwrap_or(' ')
                })
            })
            .collect();
        assert!(rendered.contains("Supply:"));
        assert!(rendered.contains("Out of Supply"));
        assert!(rendered.contains("Supplied"));
    }
}

/// Render the Battle Reports modal showing v3 reports (card-driven).  When
/// `reports_v3` is non-empty it takes precedence; otherwise the v2 list is
/// used as a fallback for legacy saves.
pub fn render_battle_reports_v3(
    frame: &mut Frame,
    area: Rect,
    reports_v3: &std::collections::VecDeque<game_core::BattleReportV3Reexport>,
    selected_index: usize,
    inspect_mode: bool,
    mode: VisualMode,
) {
    use game_core::combat_v3::card::card_by_id;
    let popup_area = centered_rect(92, 88, area);
    let glyphs = glyphs_for_mode(mode);
    frame.render_widget(Clear, popup_area);

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(Span::styled(
        format!("Recent v3 reports: {}", reports_v3.len()),
        Theme::title_style(),
    )));
    lines.push(Line::from(Span::styled(
        glyphs.horizontal_rule.to_string().repeat(64),
        Theme::dim_border_style(),
    )));

    if reports_v3.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "No v3 battle reports recorded yet.",
            Theme::muted_style(),
        )));
    } else {
        let idx = selected_index.min(reports_v3.len().saturating_sub(1));
        for i in visible_report_window(reports_v3.len(), idx) {
            let report = &reports_v3[i];
            let marker = if i == idx {
                glyphs.list_selected.to_string()
            } else {
                " ".to_string()
            };
            let summary = format!(
                "{} T{} Sys {}  F{} vs F{}  {}",
                marker,
                report.turn,
                report.star.0,
                report.fleet_a.0,
                report.fleet_b.0,
                report.system_outcome
            );
            let style = if i == idx {
                Theme::highlight_style()
            } else {
                Theme::default_style()
            };
            lines.push(Line::from(Span::styled(summary, style)));
        }
        if inspect_mode {
            let selected = &reports_v3[idx];
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                format!(
                    "Selected: {} | T{} Sys {} | F{} vs F{}",
                    selected.system_outcome,
                    selected.turn,
                    selected.star.0,
                    selected.fleet_a.0,
                    selected.fleet_b.0
                ),
                Theme::title_style(),
            )));
            lines.push(Line::from(Span::styled(
                format!(
                    "Ships: {} vs {} | Integrity: {}→{} vs {}→{}",
                    selected.ships_a,
                    selected.ships_b,
                    selected.integrity_a_start,
                    selected.integrity_a_end,
                    selected.integrity_b_start,
                    selected.integrity_b_end
                ),
                Theme::muted_style(),
            )));
            lines.push(Line::from(Span::styled(
                "Hands (Attacker / Defender):",
                Theme::title_style(),
            )));
            for (i, id) in selected.hand_a.iter().enumerate() {
                let card = card_by_id(*id);
                lines.push(Line::from(Span::styled(
                    format!("  A{}: {} ({})", i + 1, card.name, card.verb.label()),
                    Theme::text_primary_style(),
                )));
            }
            for (i, id) in selected.hand_b.iter().enumerate() {
                let card = card_by_id(*id);
                lines.push(Line::from(Span::styled(
                    format!("  D{}: {} ({})", i + 1, card.name, card.verb.label()),
                    Theme::text_secondary_style(),
                )));
            }
            if !selected.rounds.is_empty() {
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled("Rounds:", Theme::title_style())));
                for round in &selected.rounds {
                    let a = round
                        .card_a
                        .map(|c| card_by_id(c).name.to_string())
                        .unwrap_or_else(|| "(no card)".to_string());
                    let b = round
                        .card_b
                        .map(|c| card_by_id(c).name.to_string())
                        .unwrap_or_else(|| "(no card)".to_string());
                    lines.push(Line::from(Span::styled(
                        format!(
                            "  R{}: A {} | D {}  →  YOU {}  ENEMY {}",
                            round.round + 1,
                            a,
                            b,
                            round.integrity_a_after,
                            round.integrity_b_after
                        ),
                        Theme::text_primary_style(),
                    )));
                }
            }
        }
    }

    let p = Paragraph::new(lines)
        .block(
            Block::default()
                .title(Span::styled(" Battle Reports (v3) ", Theme::title_style()))
                .borders(Borders::ALL)
                .border_style(Theme::focused_border_style()),
        )
        .wrap(Wrap { trim: false });
    frame.render_widget(p, popup_area);
}

#[cfg(test)]
mod tests_v3 {
    use super::*;
    use game_core::combat_v3::card::CardId;
    use game_core::state::{FleetFormation, FleetRole, FleetSupplyState};
    use ratatui::{Terminal, backend::TestBackend};

    fn make_report(turn: u32) -> game_core::BattleReportV3Reexport {
        game_core::BattleReportV3Reexport {
            report_id: 1,
            turn,
            star: game_core::state::StarId(0),
            fleet_a: game_core::state::FleetId(1),
            fleet_b: game_core::state::FleetId(2),
            empire_a: game_core::state::EmpireId(0),
            empire_b: game_core::state::EmpireId(1),
            role_a: FleetRole::StrikeFleet,
            role_b: FleetRole::DefenseFleet,
            formation_a: FleetFormation::Balanced,
            formation_b: FleetFormation::Balanced,
            supply_a: FleetSupplyState::Supplied,
            supply_b: FleetSupplyState::Supplied,
            ships_a: 1,
            ships_b: 1,
            integrity_a_start: 100,
            integrity_b_start: 100,
            integrity_a_end: 60,
            integrity_b_end: 40,
            fleet_a_destroyed: false,
            fleet_b_destroyed: false,
            fleet_a_retreated: false,
            fleet_b_retreated: false,
            hand_a: vec![CardId(1), CardId(2), CardId(9), CardId(0), CardId(0)],
            hand_b: vec![CardId(1), CardId(5), CardId(0), CardId(0), CardId(0)],
            rounds: vec![],
            system_outcome: "Attacker holds".to_string(),
        }
    }

    #[test]
    fn v3_render_with_reports_does_not_panic() {
        let backend = TestBackend::new(140, 50);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut reports: std::collections::VecDeque<_> = std::collections::VecDeque::new();
        reports.push_back(make_report(5));
        reports.push_back(make_report(6));
        terminal
            .draw(|frame| {
                render_battle_reports_v3(
                    frame,
                    frame.area(),
                    &reports,
                    0,
                    true,
                    VisualMode::Unicode,
                )
            })
            .unwrap();
    }

    #[test]
    fn v3_render_empty_does_not_panic() {
        let backend = TestBackend::new(140, 50);
        let mut terminal = Terminal::new(backend).unwrap();
        let reports: std::collections::VecDeque<_> = std::collections::VecDeque::new();
        terminal
            .draw(|frame| {
                render_battle_reports_v3(
                    frame,
                    frame.area(),
                    &reports,
                    0,
                    false,
                    VisualMode::Unicode,
                )
            })
            .unwrap();
    }
}
