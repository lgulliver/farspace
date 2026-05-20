//! Battle report modal component

use crate::layout::centered_rect;
use crate::theme::Theme;
use crate::{glyphs::glyphs_for_mode, visual_mode::VisualMode};
use game_core::BattleReport;
use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

const MAX_VISIBLE_REPORTS: usize = 8;

pub fn render_battle_reports(
    frame: &mut Frame,
    area: Rect,
    reports: &[BattleReport],
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
        for (i, report) in reports.iter().enumerate().rev().take(MAX_VISIBLE_REPORTS) {
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
            "↑/↓ select  Enter back to list  Esc/B close"
        } else {
            "↑/↓ select  Enter inspect battle  Esc/B close"
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
