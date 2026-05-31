use crate::assertions::validate_render_text;
use crate::report::{E2eFailureCategory, E2eRunReport, E2eSeverity, RenderSnapshot};
use anyhow::{Context, Result};
use game_core::{Engine, GameState};
use game_tui::e2e_support::{E2eRenderTarget, render_target_to_text};
use game_tui::screens::Screen;
use serde_json::json;
use std::fs;
use std::panic::{self, AssertUnwindSafe};

pub const TEST_TERMINAL_SIZES: &[(u16, u16)] = &[(120, 40), (100, 32), (80, 24)];

pub fn render_and_validate_major_screens(
    engine: &Engine,
    turn: u32,
    report: &mut E2eRunReport,
) -> Result<Vec<String>> {
    let mut rendered_texts = Vec::new();

    for (width, height) in TEST_TERMINAL_SIZES {
        for target in targets_for_state(&engine.state) {
            let target_name = target_name(&target);
            report.record_screen_visit(target_name.clone());

            let rendered = panic::catch_unwind(AssertUnwindSafe(|| {
                render_target_to_text(&engine.state, &target, *width, *height)
            }));

            match rendered {
                Ok(Ok(text)) => {
                    validate_render_text(turn, &target_name, &text, *width, *height, report);
                    rendered_texts.push(text.clone());

                    report.render_snapshots.push(RenderSnapshot {
                        turn,
                        target: target_name,
                        width: *width,
                        height: *height,
                        preview: preview(&text),
                        snapshot_path: None,
                    });
                }
                Ok(Err(error)) => {
                    let snapshot_path = write_failure_snapshot(
                        report,
                        turn,
                        &target_name,
                        *width,
                        *height,
                        &format!("render io error: {error}"),
                    )
                    .ok();
                    report.push_failure(
                        turn,
                        E2eSeverity::Error,
                        E2eFailureCategory::RenderFailure,
                        Some(target_name.clone()),
                        "rendering failed",
                        json!({"error": error.to_string(), "snapshot": snapshot_path}),
                    );
                }
                Err(_) => {
                    let snapshot_path = write_failure_snapshot(
                        report,
                        turn,
                        &target_name,
                        *width,
                        *height,
                        "render panic",
                    )
                    .ok();
                    report.push_failure(
                        turn,
                        E2eSeverity::Fatal,
                        E2eFailureCategory::Panic,
                        Some(target_name.clone()),
                        "panic during render",
                        json!({"snapshot": snapshot_path}),
                    );
                }
            }
        }
    }

    Ok(rendered_texts)
}

fn targets_for_state(state: &GameState) -> Vec<E2eRenderTarget> {
    let mut targets = vec![
        E2eRenderTarget::Screen(Screen::SectorOverview),
        E2eRenderTarget::Screen(Screen::SectorMap),
        E2eRenderTarget::Screen(Screen::System),
        E2eRenderTarget::Screen(Screen::Colony),
        E2eRenderTarget::Screen(Screen::Research),
        E2eRenderTarget::Screen(Screen::EmpireOverview),
        E2eRenderTarget::Screen(Screen::Diplomacy),
        E2eRenderTarget::Screen(Screen::ShipDesigner),
        E2eRenderTarget::Screen(Screen::Settings),
        E2eRenderTarget::HelpOverlay(Screen::SectorOverview),
        E2eRenderTarget::PaletteOverlay {
            screen: Screen::SectorOverview,
            input: ":help".to_string(),
        },
    ];

    if !state.galactic_dispatches.is_empty() {
        targets.push(E2eRenderTarget::DispatchOverlay {
            screen: Screen::SectorOverview,
            history_index: state.galactic_dispatches.len().saturating_sub(1),
        });
    }

    if !state.battle_reports.is_empty() {
        targets.push(E2eRenderTarget::BattleReportsOverlay {
            screen: Screen::SectorOverview,
            report_index: state.battle_reports.len().saturating_sub(1),
            inspect: false,
        });
    }

    targets
}

fn target_name(target: &E2eRenderTarget) -> String {
    match target {
        E2eRenderTarget::Screen(screen) => format!("{screen:?}"),
        E2eRenderTarget::HelpOverlay(screen) => format!("HelpOverlay({screen:?})"),
        E2eRenderTarget::PaletteOverlay { screen, .. } => {
            format!("CommandPaletteOverlay({screen:?})")
        }
        E2eRenderTarget::DispatchOverlay { screen, .. } => {
            format!("DispatchOverlay({screen:?})")
        }
        E2eRenderTarget::BattleReportsOverlay { screen, .. } => {
            format!("BattleReportsOverlay({screen:?})")
        }
    }
}

fn preview(text: &str) -> String {
    let mut chars = text.chars();
    let sample = chars.by_ref().take(180).collect::<String>();
    if chars.next().is_some() {
        format!("{sample}...")
    } else {
        sample
    }
}

fn write_failure_snapshot(
    report: &E2eRunReport,
    turn: u32,
    target: &str,
    width: u16,
    height: u16,
    body: &str,
) -> Result<String> {
    let path = report.snapshot_file_path(turn, target, width, height);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).context("create snapshot directory")?;
    }
    fs::write(&path, body).context("write snapshot")?;
    Ok(path.to_string_lossy().to_string())
}
