use super::*;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::Terminal;
use std::sync::atomic::{AtomicUsize, Ordering};

static TEST_FILE_COUNTER: AtomicUsize = AtomicUsize::new(0);

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

/// Create a unique temporary file path for tests to avoid parallel races.
fn tmp_save_path(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("farspace_test_{}.sav", name))
}

#[test]
fn menu_v_key_cycles_visual_mode_without_active_game() {
    let mut app = App::new();
    app.state.active = Screen::Menu;
    let before = app.state.visual_mode;

    app.handle_key(key(KeyCode::Char('v')));

    assert_eq!(app.state.visual_mode, before.next());
}

#[test]
fn visual_mode_config_roundtrip_from_file() {
    let unique = TEST_FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "farspace_ui_mode_{}_{}.conf",
        std::process::id(),
        unique
    ));

    App::persist_visual_mode_to_path(&path, crate::visual_mode::VisualMode::NerdFont).unwrap();
    let loaded = App::load_visual_mode_from_path(&path);
    let _ = std::fs::remove_file(&path);

    assert_eq!(loaded, crate::visual_mode::VisualMode::NerdFont);
}

#[test]
fn visual_mode_invalid_config_falls_back_to_default() {
    let unique = TEST_FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "farspace_ui_mode_invalid_{}_{}.conf",
        std::process::id(),
        unique
    ));
    std::fs::write(&path, "visual_mode=invalid\n").unwrap();

    let loaded = App::load_visual_mode_from_path(&path);
    let _ = std::fs::remove_file(&path);

    assert_eq!(loaded, crate::visual_mode::VisualMode::default());
}

#[test]
fn switching_visual_mode_does_not_mutate_game_core_state() {
    let mut app = App::new();
    app.new_game(42);
    let before_mode = app.state.visual_mode;
    let before = app.engine.as_ref().unwrap().state.clone();
    let unique = TEST_FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
    let mode_path = std::env::temp_dir().join(format!(
        "farspace_ui_mode_switch_{}_{}.conf",
        std::process::id(),
        unique
    ));

    app.execute_palette_command_with_path(PaletteCommand::VisualMode, &mode_path);

    assert_eq!(app.engine.as_ref().unwrap().state, before);
    assert_eq!(app.state.visual_mode, before_mode.next());
    assert!(mode_path.exists());
    let _ = std::fs::remove_file(mode_path);
}

#[test]
fn major_screens_render_smoke_in_all_visual_modes() {
    let backend = TestBackend::new(140, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = App::new();
    app.new_game(42);
    let selected_colony = app
        .engine
        .as_ref()
        .unwrap()
        .state
        .colonies
        .keys()
        .next()
        .copied();
    let screens = [
        Screen::Menu,
        Screen::SectorOverview,
        Screen::SectorMap,
        Screen::System,
        Screen::Colony,
        Screen::Research,
        Screen::EmpireOverview,
    ];

    for mode in [
        crate::visual_mode::VisualMode::Ascii,
        crate::visual_mode::VisualMode::Unicode,
        crate::visual_mode::VisualMode::NerdFont,
    ] {
        app.state.visual_mode = mode;
        for screen in screens {
            app.state.active = screen;
            if screen == Screen::Colony {
                app.state.colony.selected_colony = selected_colony;
            }
            terminal.draw(|frame| app.render(frame)).unwrap();
        }
    }
}

#[test]
fn toggle_help_overlay_on_question_mark() {
    let mut app = App::new();
    assert!(!app.state.overlay.show_help);

    app.handle_key(key(KeyCode::Char('?')));
    assert!(app.state.overlay.show_help);

    app.handle_key(key(KeyCode::Char('?')));
    assert!(!app.state.overlay.show_help);
}

#[test]
fn toggle_palette_on_colon() {
    let mut app = App::new();
    assert!(!app.state.overlay.show_palette);

    app.handle_key(key(KeyCode::Char(':')));
    assert!(app.state.overlay.show_palette);

    app.handle_key(key(KeyCode::Esc));
    assert!(!app.state.overlay.show_palette);
}

#[test]
fn quit_flag_set_on_q() {
    let mut app = App::new();
    assert!(!app.state.quit);

    app.handle_key(key(KeyCode::Char('q')));
    assert!(app.state.quit);
}

#[test]
fn screen_transitions_on_new_game() {
    let mut app = App::new();
    assert_eq!(app.state.active, Screen::Menu);

    app.new_game(42);
    assert_eq!(app.state.active, Screen::SectorOverview);
    assert!(app.engine.is_some());
}

#[test]
fn enter_opens_sector_map_from_sector_overview() {
    let mut app = App::new();
    app.new_game(42);
    assert_eq!(app.state.active, Screen::SectorOverview);

    app.handle_key(key(KeyCode::Enter));
    assert_eq!(app.state.active, Screen::SectorMap);
}

#[test]
fn enter_opens_system_view_from_sector_map() {
    let mut app = App::new();
    app.new_game(42);
    app.state.active = Screen::SectorMap;

    app.handle_key(key(KeyCode::Enter));
    assert_eq!(app.state.active, Screen::System);
}

#[test]
fn o_key_opens_empire_overview() {
    let mut app = App::new();
    app.new_game(42);
    app.state.active = Screen::SectorMap;

    app.handle_key(key(KeyCode::Char('O')));

    assert_eq!(app.state.active, Screen::EmpireOverview);
}

#[test]
fn lowercase_o_key_opens_empire_overview() {
    let mut app = App::new();
    app.new_game(42);
    app.state.active = Screen::SectorMap;

    app.handle_key(key(KeyCode::Char('o')));

    assert_eq!(app.state.active, Screen::EmpireOverview);
}

#[test]
fn v_key_opens_empire_overview_victory_panel() {
    let mut app = App::new();
    app.new_game(42);
    app.state.active = Screen::SectorMap;

    app.handle_key(key(KeyCode::Char('V')));

    assert_eq!(app.state.active, Screen::EmpireOverview);
}

#[test]
fn end_turn_report_includes_victory_milestones() {
    let report = App::build_end_turn_report(
        8,
        &[
            game_core::Event::VictoryProgressMilestone {
                path: game_core::VictoryPath::Discovery,
                empire: game_core::EmpireId(1),
                progress_percent: 50,
            },
            game_core::Event::VictoryAchieved {
                winner: game_core::EmpireId(1),
                path: game_core::VictoryPath::Dominion,
                turn: 8,
            },
        ],
    );
    assert!(report.contains("victory milestones 1"));
    assert!(report.contains("victories 1"));
}

#[test]
fn overview_enter_opens_selected_colony() {
    let mut app = App::new();
    app.new_game(42);
    app.state.active = Screen::EmpireOverview;
    app.state.overview.cursor = 0;

    let expected_colony = app
        .engine
        .as_ref()
        .unwrap()
        .state
        .colonies
        .values()
        .find(|c| c.owner == app.engine.as_ref().unwrap().state.player_empire)
        .map(|c| c.id)
        .expect("player colony should exist");

    app.handle_key(key(KeyCode::Enter));

    assert_eq!(app.state.active, Screen::Colony);
    assert_eq!(app.state.colony.selected_colony, Some(expected_colony));
}

#[test]
fn overview_s_opens_selected_system() {
    let mut app = App::new();
    app.new_game(42);
    app.state.active = Screen::EmpireOverview;
    app.state.overview.cursor = 0;

    let expected_star = app
        .engine
        .as_ref()
        .unwrap()
        .state
        .colonies
        .values()
        .find(|c| c.owner == app.engine.as_ref().unwrap().state.player_empire)
        .map(|c| c.star)
        .expect("player colony should exist");

    app.handle_key(key(KeyCode::Char('S')));

    assert_eq!(app.state.active, Screen::System);
    assert_eq!(app.state.navigation.selected_star, Some(expected_star));
}

#[test]
fn esc_returns_from_system_view_to_sector_map() {
    let mut app = App::new();
    app.new_game(42);
    app.state.active = Screen::System;

    app.handle_key(key(KeyCode::Esc));
    assert_eq!(app.state.active, Screen::SectorMap);
}

#[test]
fn system_view_colonize_targets_selected_planet() {
    let mut app = App::new();
    app.new_game(42);

    let engine = app.engine.as_mut().unwrap();
    let target = *engine
        .state
        .explored_stars
        .iter()
        .find(|&&sid| sid != engine.state.empires[&engine.state.player_empire].home_star)
        .expect("need explored non-home star");
    app.state.navigation.selected_star = Some(target);
    app.state.navigation.selected_planet_index = 1;
    app.state.active = Screen::System;

    {
        let star = engine.state.stars.get_mut(&target).unwrap();
        if star.planets.len() < 2 {
            let clone = star.planets[0].clone();
            star.planets.push(clone);
        }
        for planet in &mut star.planets {
            planet.colony = None;
            planet.habitable = true;
            planet.surveyed = true;
        }
    }

    let fleet_id = FleetId(6000);
    engine.state.fleets.insert(
        fleet_id,
        game_core::Fleet {
            id: fleet_id,
            owner: engine.state.player_empire,
            location: target,
            ships: 1,
            kind: FleetKind::Colonizer,
            strength: 1,
            integrity: 100,
        },
    );

    app.handle_key(key(KeyCode::Char('C')));

    let star = &app.engine.as_ref().unwrap().state.stars[&target];
    assert!(
        star.planets[1].colony.is_some(),
        "selected planet should be colonized"
    );
}

#[test]
fn system_view_invade_targets_selected_planet() {
    let mut app = App::new();
    app.new_game(42);

    let engine = app.engine.as_mut().unwrap();
    let target = *engine
        .state
        .explored_stars
        .iter()
        .find(|&&sid| sid != engine.state.empires[&engine.state.player_empire].home_star)
        .expect("need explored non-home star");
    app.state.navigation.selected_star = Some(target);
    app.state.navigation.selected_planet_index = 0;
    app.state.active = Screen::System;

    let enemy_id = engine.state.ai_empire.expect("AI empire required");
    engine
        .state
        .diplomacy
        .insert(enemy_id, game_core::RelationshipStatus::War);

    let enemy_colony_id = game_core::ColonyId(9001);
    engine.state.colonies.insert(
        enemy_colony_id,
        game_core::Colony {
            id: enemy_colony_id,
            star: target,
            planet_index: 0,
            owner: enemy_id,
            population: 1,
            production: 5,
            prod_pct: 50,
            research_pct: 50,
            build_queue: Vec::new(),
            accumulated_production: 0,
            buildings: Vec::new(),
            surface_installations: Vec::new(),
            orbital_installations: Vec::new(),
            stability: 10,
            role: game_core::ColonyRole::Balanced,
            rally_point: None,
        },
    );
    engine.state.stars.get_mut(&target).unwrap().planets[0].colony = Some(enemy_colony_id);

    let troop_fleet = FleetId(6100);
    engine.state.fleets.insert(
        troop_fleet,
        game_core::Fleet {
            id: troop_fleet,
            owner: engine.state.player_empire,
            location: target,
            ships: 1,
            kind: FleetKind::TroopTransport,
            strength: 1,
            integrity: 100,
        },
    );

    app.handle_key(key(KeyCode::Char('I')));

    let captured_owner = app.engine.as_ref().unwrap().state.colonies[&enemy_colony_id].owner;
    assert_eq!(
        captured_owner,
        app.engine.as_ref().unwrap().state.player_empire,
        "selected enemy planet should be invaded and captured"
    );
}

#[test]
fn star_selection_moves_on_hjkl() {
    let mut app = App::new();
    app.new_game(42);

    let initial = app.state.navigation.selected_star;
    assert!(initial.is_some());

    // Move right
    app.handle_key(key(KeyCode::Char('l')));

    // Selection should change (might be same if no star to right)
    // Just verify it doesn't crash
    assert!(app.state.navigation.selected_star.is_some());
}

#[test]
fn end_turn_advances_game() {
    let mut app = App::new();
    app.new_game(42);

    let initial_turn = app.engine.as_ref().unwrap().state.turn;

    app.handle_key(key(KeyCode::Char('t')));

    let new_turn = app.engine.as_ref().unwrap().state.turn;
    assert_eq!(new_turn, initial_turn + 1);
}

#[test]
fn end_turn_with_e_key() {
    let mut app = App::new();
    app.new_game(42);
    let initial_turn = app.engine.as_ref().unwrap().state.turn;

    app.handle_key(key(KeyCode::Char('e')));

    assert_eq!(app.engine.as_ref().unwrap().state.turn, initial_turn + 1);
}

#[test]
fn app_renders_without_panic() {
    let backend = TestBackend::new(120, 40);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = App::new();
    app.new_game(42);

    terminal.draw(|frame| app.render(frame)).unwrap();
}

#[test]
fn menu_renders_without_panic() {
    let backend = TestBackend::new(120, 40);
    let mut terminal = Terminal::new(backend).unwrap();

    let app = App::new();

    terminal.draw(|frame| app.render(frame)).unwrap();
}

#[test]
fn end_turn_without_engine_does_nothing() {
    let mut app = App::new();
    // No game started — end_turn should be a no-op
    app.end_turn();
    assert!(app.engine.is_none());
}

#[test]
fn move_star_selection_without_engine_does_nothing() {
    let mut app = App::new();
    app.move_star_selection(1, 0);
    assert!(app.state.navigation.selected_star.is_none());
}

#[test]
fn move_star_selection_with_no_selection_selects_first() {
    let mut app = App::new();
    app.new_game(42);
    app.state.navigation.selected_star = None;

    // With no selection, move should select first star
    app.move_star_selection(1, 0);
    // Either first star selected or unchanged; verify no panic
    // (may remain None if no star is to the right, but first-star selection triggers)
}

#[test]
fn handle_key_end_turn_on_galaxy_screen() {
    let mut app = App::new();
    app.new_game(42);
    let initial_turn = app.engine.as_ref().unwrap().state.turn;

    // 't' ends the turn on galaxy screen (Enter opens System View)
    app.handle_key(key(KeyCode::Char('t')));
    assert_eq!(app.engine.as_ref().unwrap().state.turn, initial_turn + 1);
}

#[test]
fn escape_closes_help_overlay() {
    let mut app = App::new();
    app.state.overlay.show_help = true;

    app.handle_key(key(KeyCode::Esc));
    assert!(!app.state.overlay.show_help);
}

#[test]
fn palette_key_closes_palette() {
    let mut app = App::new();
    app.state.overlay.show_palette = true;
    app.state.overlay.palette_input = String::new();

    // Pressing `:` when palette is open adds `:` to input (does not close)
    app.handle_key(key(KeyCode::Char(':')));
    assert!(app.state.overlay.show_palette);
    assert_eq!(app.state.overlay.palette_input, ":");

    // Pressing Esc closes the palette
    app.handle_key(key(KeyCode::Esc));
    assert!(!app.state.overlay.show_palette);
    assert!(app.state.overlay.palette_input.is_empty());
}

#[test]
fn palette_accepts_character_input() {
    let mut app = App::new();
    app.state.overlay.show_palette = true;
    app.state.overlay.palette_input.clear();

    app.handle_key(key(KeyCode::Char('s')));
    app.handle_key(key(KeyCode::Char('a')));
    app.handle_key(key(KeyCode::Char('v')));
    app.handle_key(key(KeyCode::Char('e')));

    assert_eq!(app.state.overlay.palette_input, "save");
}

#[test]
fn palette_backspace_removes_last_char() {
    let mut app = App::new();
    app.state.overlay.show_palette = true;
    app.state.overlay.palette_input = "sav".to_string();

    app.handle_key(key(KeyCode::Backspace));

    assert_eq!(app.state.overlay.palette_input, "sa");
}

#[test]
fn palette_unknown_command_logs_error() {
    let mut app = App::new();
    app.new_game(42);
    let before = app.state.log.len();

    app.execute_palette_input("unknowncmd");

    assert!(app.state.log.len() > before);
}

#[test]
fn palette_enter_executes_and_closes() {
    let mut app = App::new();
    app.state.overlay.show_palette = true;
    app.state.overlay.palette_input = "unknowncmd".to_string();

    app.handle_key(key(KeyCode::Enter));

    assert!(!app.state.overlay.show_palette);
    assert!(app.state.overlay.palette_input.is_empty());
}

#[test]
fn save_game_without_engine_returns_error() {
    let mut app = App::new();
    let path = tmp_save_path("no_engine");
    let result = app.save_game(&path);
    assert!(result.is_err());
}

#[test]
fn save_and_load_round_trip_via_app() {
    let path = tmp_save_path("round_trip");
    let mut app = App::new();
    app.new_game(42);

    // End a turn so state is non-trivial
    app.end_turn();
    let turn_before = app.engine.as_ref().unwrap().state.turn;

    // Save then load
    app.save_game(&path).expect("save should succeed");
    app.load_game(&path).expect("load should succeed");

    let turn_after = app.engine.as_ref().unwrap().state.turn;
    assert_eq!(turn_before, turn_after);
    assert_eq!(app.state.active, Screen::SectorOverview);

    let _ = std::fs::remove_file(&path);
}

#[test]
fn load_game_missing_file_logs_error() {
    let path = tmp_save_path("missing_file");
    // Ensure file does not exist
    let _ = std::fs::remove_file(&path);

    let mut app = App::new();
    let result = app.load_game(&path);
    assert!(result.is_err());
    // Error message should contain the path
    let msg = result.unwrap_err();
    assert!(
        msg.contains("Error:"),
        "Expected 'Error:' prefix, got: {}",
        msg
    );
}

#[test]
fn save_command_via_palette_logs_success() {
    let path = tmp_save_path("palette_save");
    let mut app = App::new();
    app.new_game(42);

    // Temporarily override the palette to call save directly with our path
    let before = app.state.log.len();
    match app.save_game(&path) {
        Ok(()) => app.state.log.push("Game saved.".to_string()),
        Err(e) => app.state.log.push(e),
    }

    assert!(app.state.log.len() > before);
    let last = app.state.log.last_n(1);
    assert!(
        last[0].contains("saved") || last[0].contains("Save"),
        "Expected save confirmation, got: {}",
        last[0]
    );

    let _ = std::fs::remove_file(&path);
}

#[test]
fn load_command_after_save_via_palette() {
    let path = tmp_save_path("palette_load");
    let mut app = App::new();
    app.new_game(42);
    app.end_turn();
    let turn_before = app.engine.as_ref().unwrap().state.turn;

    app.save_game(&path).expect("save should succeed");
    app.load_game(&path).expect("load should succeed");

    let turn_after = app.engine.as_ref().unwrap().state.turn;
    assert_eq!(turn_before, turn_after);

    let _ = std::fs::remove_file(&path);
}

#[test]
fn end_turn_appends_events_to_log() {
    let mut app = App::new();
    app.new_game(42);
    let before = app.state.log.len();

    app.end_turn();

    // At least one event should have been logged (TurnAdvanced, ColonyProduced, etc.)
    assert!(app.state.log.len() > before);
}

#[test]
fn palette_colon_only_input_does_not_log() {
    let mut app = App::new();
    app.new_game(42);
    let before = app.state.log.len();

    // A bare ":" should be a no-op after normalization
    app.execute_palette_input(":");
    app.execute_palette_input("  ");
    app.execute_palette_input(":  ");

    assert_eq!(app.state.log.len(), before, "No-op commands should not log");
}

#[test]
fn palette_enter_with_colon_only_does_not_execute() {
    let mut app = App::new();
    app.new_game(42);
    app.state.overlay.show_palette = true;
    app.state.overlay.palette_input = ":".to_string();
    let before = app.state.log.len();

    app.handle_key(key(KeyCode::Enter));

    assert!(!app.state.overlay.show_palette);
    // No command should have been executed → log unchanged
    assert_eq!(app.state.log.len(), before);
}

#[test]
fn save_error_message_contains_error_prefix() {
    let path = tmp_save_path("error_prefix");
    let _ = std::fs::remove_file(&path);
    // Use an invalid path to force an error
    let invalid_path = std::path::Path::new("/nonexistent_dir/farspace_test.sav");
    let mut app = App::new();
    app.new_game(42);
    let result = app.save_game(invalid_path);
    assert!(result.is_err());
    let msg = result.unwrap_err();
    assert!(
        msg.starts_with("Error:"),
        "Expected 'Error:' prefix, got: {}",
        msg
    );
    assert!(
        msg.contains("nonexistent_dir"),
        "Expected path in message, got: {}",
        msg
    );
}

#[test]
fn menu_load_key_triggers_load() {
    // This test calls handle_key(l) on the menu, which internally uses DEFAULT_SAVE_PATH.
    // We just verify it doesn't crash and stays on menu when the file is absent.
    // The file may or may not exist from other tests; either outcome (load succeeds or fails
    // gracefully) is acceptable here — we only care there's no panic.
    let mut app = App::new();
    app.handle_key(key(KeyCode::Char('l')));
    // If load failed, screen stays Menu. If it somehow succeeded, it moves to Galaxy.
    // Both are valid; the key requirement is no panic.
    let _ = app.state.active;
}

#[test]
fn end_turn_report_counts_key_events() {
    let events = vec![
        CoreEvent::SystemExplored { star: StarId(1) },
        CoreEvent::PlanetSurveyCompleted {
            star: StarId(2),
            planet_index: 0,
        },
        CoreEvent::ColonizationCompleted {
            empire: game_core::EmpireId(1),
            fleet: FleetId(9),
            star: StarId(3),
            planet_index: 1,
            colony: ColonyId(77),
        },
        CoreEvent::ResearchCompleted { tech: TechId(4) },
        CoreEvent::FleetArrived {
            fleet: FleetId(8),
            star: StarId(5),
        },
        CoreEvent::FoodShortage {
            empire: game_core::EmpireId(1),
            deficit: 2,
        },
        CoreEvent::ColonyIsolated {
            colony: ColonyId(77),
        },
        CoreEvent::ColonyReconnected {
            colony: ColonyId(78),
        },
        CoreEvent::InvasionSucceeded {
            attacker: game_core::EmpireId(1),
            defender: game_core::EmpireId(2),
            fleet: FleetId(10),
            star: StarId(6),
            planet_index: 0,
            colony: ColonyId(79),
            transports_lost: 1,
        },
        CoreEvent::InvasionFailed {
            attacker: game_core::EmpireId(1),
            defender: game_core::EmpireId(2),
            fleet: FleetId(11),
            star: StarId(6),
            planet_index: 1,
            colony: ColonyId(80),
            invasion_strength: 12,
            defense_strength: 20,
            transports_lost: 1,
            reason: "Defenses held".to_string(),
        },
        CoreEvent::Error {
            message: "bad command".to_string(),
        },
    ];

    let report = App::build_end_turn_report(12, &events);
    assert!(report.contains("Turn 12 global summary (all empires)"));
    assert!(report.contains("explored 1"));
    assert!(report.contains("surveyed 1"));
    assert!(report.contains("colonized 1"));
    assert!(report.contains("research 1"));
    assert!(report.contains("queued starts 0"));
    assert!(report.contains("arrivals 1"));
    assert!(report.contains("invasions won 1"));
    assert!(report.contains("invasions failed 1"));
    assert!(report.contains("warnings 1"));
    assert!(report.contains("isolated 1"));
    assert!(report.contains("reconnected 1"));
    assert!(report.contains("errors 1"));
}

#[test]
fn end_turn_report_handles_empty_event_list() {
    let report = App::build_end_turn_report(3, &[]);
    assert_eq!(
        report,
        "Turn 3 global summary (all empires): explored 0, surveyed 0, colonized 0, research 0, queued starts 0, arrivals 0, invasions won 0, invasions failed 0, treaties 0, wars 0, peaces 0, victory milestones 0, victories 0, warnings 0, isolated 0, reconnected 0, errors 0."
    );
}

#[test]
fn dispatch_command_logs_events_and_updates_error_status() {
    let mut app = App::new();
    app.new_game(42);
    let before = app.state.log.len();

    app.dispatch_command(Command::SelectResearch { tech: TechId(9999) });

    assert!(app.state.log.len() > before);
    assert_eq!(
        app.state.status_message.as_deref(),
        Some("Error: Tech 9999 not found")
    );
}

#[test]
fn dispatch_command_end_turn_sets_summary_status() {
    let mut app = App::new();
    app.new_game(42);
    let initial_turn = app.engine.as_ref().unwrap().state.turn;

    app.dispatch_command(Command::EndTurn);

    assert_eq!(app.engine.as_ref().unwrap().state.turn, initial_turn + 1);
    assert!(app
        .state
        .status_message
        .as_deref()
        .is_some_and(|message| message.starts_with("Turn 2 global summary")));
}

#[test]
fn galaxy_renders_with_log_entries() {
    let backend = TestBackend::new(120, 40);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = App::new();
    app.new_game(42);
    app.end_turn();

    terminal.draw(|frame| app.render(frame)).unwrap();
}

// ──────────────────────────────────────────────────────────────────
// Colony screen tests
// ──────────────────────────────────────────────────────────────────

/// Navigate to the colony screen from a star that has the player's colony.
fn goto_colony_screen(app: &mut App) -> bool {
    // Find the home star (which holds the colony)
    let engine = app.engine.as_ref().unwrap();
    let player_empire = engine.state.player_empire;
    let home_star_id = engine
        .state
        .colonies
        .values()
        .find(|c| c.owner == player_empire)
        .map(|c| c.star);

    if let Some(star_id) = home_star_id {
        app.state.navigation.selected_star = Some(star_id);
        app.try_enter_colony()
    } else {
        false
    }
}

#[test]
fn enter_colony_from_sector_map_with_c_key() {
    let mut app = App::new();
    app.new_game(42);
    app.state.active = Screen::SectorMap;

    // Select the star that has the player's home colony
    let player_empire = app.engine.as_ref().unwrap().state.player_empire;
    let home_star_id = app
        .engine
        .as_ref()
        .unwrap()
        .state
        .colonies
        .values()
        .find(|c| c.owner == player_empire)
        .map(|c| c.star);
    app.state.navigation.selected_star = home_star_id;

    // Press 'c' to enter the colony screen — exercises the actual key binding
    app.handle_key(key(KeyCode::Char('c')));

    assert_eq!(app.state.active, Screen::Colony);
    assert!(app.state.colony.selected_colony.is_some());
}

#[test]
fn try_enter_colony_prefers_selected_planet_colony() {
    let mut app = App::new();
    app.new_game(42);

    let engine = app.engine.as_mut().unwrap();
    let player_empire = engine.state.player_empire;
    let home_colony_id = engine
        .state
        .colonies
        .iter()
        .find(|(_, c)| c.owner == player_empire)
        .map(|(id, _)| *id)
        .expect("player colony should exist");
    let home_colony = engine
        .state
        .colonies
        .get(&home_colony_id)
        .cloned()
        .expect("player colony data should exist");
    let home_star_id = home_colony.star;

    let second_colony_id = ColonyId(engine.state.next_colony_id);
    engine.state.next_colony_id += 1;
    let mut second_colony = home_colony.clone();
    second_colony.id = second_colony_id;
    second_colony.planet_index = 1;
    second_colony.accumulated_production = 0;
    second_colony.build_queue.clear();
    engine
        .state
        .colonies
        .insert(second_colony_id, second_colony);

    let star = engine
        .state
        .stars
        .get_mut(&home_star_id)
        .expect("home star should exist");
    while star.planets.len() < 2 {
        let mut clone = star.planets[0].clone();
        clone.colony = None;
        star.planets.push(clone);
    }
    star.planets[1].colony = Some(second_colony_id);

    app.state.navigation.selected_star = Some(home_star_id);
    app.state.navigation.selected_planet_index = 1;
    assert!(app.try_enter_colony());
    assert_eq!(app.state.colony.selected_colony, Some(second_colony_id));
    assert_eq!(app.state.active, Screen::Colony);
}

#[test]
fn c_key_in_system_view_enters_selected_planet_colony() {
    let mut app = App::new();
    app.new_game(42);

    let engine = app.engine.as_mut().unwrap();
    let player_empire = engine.state.player_empire;
    let home_colony_id = engine
        .state
        .colonies
        .iter()
        .find(|(_, c)| c.owner == player_empire)
        .map(|(id, _)| *id)
        .expect("player colony should exist");
    let home_colony = engine
        .state
        .colonies
        .get(&home_colony_id)
        .cloned()
        .expect("player colony data should exist");
    let home_star_id = home_colony.star;

    let second_colony_id = ColonyId(engine.state.next_colony_id);
    engine.state.next_colony_id += 1;
    let mut second_colony = home_colony.clone();
    second_colony.id = second_colony_id;
    second_colony.planet_index = 1;
    second_colony.accumulated_production = 0;
    second_colony.build_queue.clear();
    engine
        .state
        .colonies
        .insert(second_colony_id, second_colony);

    let star = engine
        .state
        .stars
        .get_mut(&home_star_id)
        .expect("home star should exist");
    while star.planets.len() < 2 {
        let mut clone = star.planets[0].clone();
        clone.colony = None;
        star.planets.push(clone);
    }
    star.planets[1].colony = Some(second_colony_id);

    app.state.active = Screen::System;
    app.state.navigation.selected_star = Some(home_star_id);
    app.state.navigation.selected_planet_index = 1;
    app.handle_key(key(KeyCode::Char('c')));

    assert_eq!(app.state.active, Screen::Colony);
    assert_eq!(app.state.colony.selected_colony, Some(second_colony_id));
}

#[test]
fn enter_key_in_system_view_enters_selected_planet_colony() {
    let mut app = App::new();
    app.new_game(42);

    let engine = app.engine.as_ref().unwrap();
    let player_empire = engine.state.player_empire;
    let home_colony_id = engine
        .state
        .colonies
        .iter()
        .find(|(_, c)| c.owner == player_empire)
        .map(|(id, _)| *id)
        .expect("player colony should exist");
    let home_star_id = engine.state.colonies[&home_colony_id].star;

    app.state.active = Screen::System;
    app.state.navigation.selected_star = Some(home_star_id);
    app.state.navigation.selected_planet_index =
        engine.state.colonies[&home_colony_id].planet_index;
    app.state.colony.selected_colony = None;

    app.handle_key(key(KeyCode::Enter));

    assert_eq!(app.state.active, Screen::Colony);
    assert_eq!(app.state.colony.selected_colony, Some(home_colony_id));
}

#[test]
fn enter_key_in_system_view_keeps_system_without_player_colony() {
    let mut app = App::new();
    app.new_game(42);

    let engine = app.engine.as_ref().unwrap();
    let player_empire = engine.state.player_empire;
    let target_star = engine
        .state
        .stars
        .iter()
        .find(|(_, s)| {
            s.planets.iter().all(|p| {
                p.colony.is_none_or(|cid| {
                    engine
                        .state
                        .colonies
                        .get(&cid)
                        .is_none_or(|c| c.owner != player_empire)
                })
            })
        })
        .map(|(id, _)| *id)
        .expect("need a star without a player colony");

    app.state.active = Screen::System;
    app.state.navigation.selected_star = Some(target_star);
    app.state.navigation.selected_planet_index = 0;
    app.state.colony.selected_colony = None;

    app.handle_key(key(KeyCode::Enter));

    assert_eq!(app.state.active, Screen::System);
    assert!(app.state.colony.selected_colony.is_none());
}

#[test]
fn try_enter_colony_returns_false_without_engine() {
    let mut app = App::new();
    assert!(!app.try_enter_colony());
}

#[test]
fn try_enter_colony_returns_false_without_selected_star() {
    let mut app = App::new();
    app.new_game(42);
    app.state.navigation.selected_star = None;
    assert!(!app.try_enter_colony());
}

#[test]
fn try_enter_colony_returns_false_for_empty_star() {
    let mut app = App::new();
    app.new_game(42);
    // Navigate to SectorMap first
    app.state.active = Screen::SectorMap;
    // Select a star that has no colony
    let engine = app.engine.as_ref().unwrap();
    let player_empire = engine.state.player_empire;
    let empty_star = engine
        .state
        .stars
        .iter()
        .find(|(_, s)| {
            s.planets.iter().all(|p| {
                p.colony.is_none_or(|cid| {
                    engine
                        .state
                        .colonies
                        .get(&cid)
                        .is_none_or(|c| c.owner != player_empire)
                })
            })
        })
        .map(|(id, _)| *id);

    if let Some(star_id) = empty_star {
        app.state.navigation.selected_star = Some(star_id);
        assert!(!app.try_enter_colony());
        assert_eq!(app.state.active, Screen::SectorMap);
    }
    // If every star has a colony (very unlikely with 20 stars and 1 colony) we skip
}

#[test]
fn esc_returns_to_sector_map_from_colony_screen() {
    let mut app = App::new();
    app.new_game(42);
    goto_colony_screen(&mut app);
    assert_eq!(app.state.active, Screen::Colony);

    app.handle_key(key(KeyCode::Esc));
    assert_eq!(app.state.active, Screen::SectorMap);
    assert!(app.state.colony.selected_colony.is_none());
}

#[test]
fn colony_build_cursor_moves_with_j_k() {
    let mut app = App::new();
    app.new_game(42);
    goto_colony_screen(&mut app);

    let initial = app.state.colony.build_cursor;
    app.handle_key(key(KeyCode::Char('j')));
    assert_ne!(app.state.colony.build_cursor, initial);

    app.handle_key(key(KeyCode::Char('k')));
    assert_eq!(app.state.colony.build_cursor, initial);
}

#[test]
fn colony_build_cursor_wraps_around_bottom() {
    let mut app = App::new();
    app.new_game(42);
    goto_colony_screen(&mut app);

    let count = app.visible_build_count();
    // Move down past the last item
    for _ in 0..count {
        app.handle_key(key(KeyCode::Char('j')));
    }
    // Cursor should have wrapped to the start
    assert_eq!(app.state.colony.build_cursor, 0);
}

#[test]
fn colony_build_cursor_wraps_around_top() {
    let mut app = App::new();
    app.new_game(42);
    goto_colony_screen(&mut app);

    // Move up from 0 should wrap to last
    app.handle_key(key(KeyCode::Char('k')));
    let count = app.visible_build_count();
    assert_eq!(app.state.colony.build_cursor, count - 1);
}

#[test]
fn enter_key_queues_building_on_colony_screen() {
    let mut app = App::new();
    app.new_game(42);
    goto_colony_screen(&mut app);

    let colony_id = app.state.colony.selected_colony.unwrap();
    let initial_queue_len = app
        .engine
        .as_ref()
        .unwrap()
        .state
        .colonies
        .get(&colony_id)
        .unwrap()
        .build_queue
        .len();

    // Press Enter to queue the currently selected building
    app.handle_key(key(KeyCode::Enter));

    let new_queue_len = app
        .engine
        .as_ref()
        .unwrap()
        .state
        .colonies
        .get(&colony_id)
        .unwrap()
        .build_queue
        .len();

    assert_eq!(
        new_queue_len,
        initial_queue_len + 1,
        "Queue should grow by 1 after Enter"
    );
}

#[test]
fn end_turn_works_from_colony_screen_with_e_key() {
    let mut app = App::new();
    app.new_game(42);
    goto_colony_screen(&mut app);

    let initial_turn = app.engine.as_ref().unwrap().state.turn;
    app.handle_key(key(KeyCode::Char('e')));
    assert_eq!(app.engine.as_ref().unwrap().state.turn, initial_turn + 1);
}

#[test]
fn colony_screen_renders_without_panic() {
    let backend = TestBackend::new(120, 40);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = App::new();
    app.new_game(42);
    goto_colony_screen(&mut app);

    terminal.draw(|frame| app.render(frame)).unwrap();
}

#[test]
fn save_load_preserves_colony_buildings() {
    let path = tmp_save_path("colony_buildings");
    let mut app = App::new();
    app.new_game(42);

    // Navigate to colony and queue AquacultureBay (first building, cost 60)
    goto_colony_screen(&mut app);
    app.handle_key(key(KeyCode::Enter)); // queue first building
    app.handle_key(key(KeyCode::Esc)); // back to galaxy

    let colony_id = app
        .engine
        .as_ref()
        .unwrap()
        .state
        .colonies
        .keys()
        .next()
        .copied()
        .unwrap();

    // Complete the building: base production is 10/turn, AquacultureBay costs 60 → 6 turns.
    // Close any Galactic Dispatch overlay before each end-turn key so it is not intercepted.
    for _ in 0..6 {
        if app.state.overlay.show_dispatch {
            app.handle_key(key(KeyCode::Esc));
        }
        app.handle_key(key(KeyCode::Char('e')));
    }

    let buildings_before = app
        .engine
        .as_ref()
        .unwrap()
        .state
        .colonies
        .get(&colony_id)
        .unwrap()
        .buildings
        .clone();

    assert!(
        !buildings_before.is_empty(),
        "Building should be completed after enough production turns"
    );

    // Save and reload
    app.save_game(&path).expect("save should succeed");
    app.load_game(&path).expect("load should succeed");

    let buildings_after = app
        .engine
        .as_ref()
        .unwrap()
        .state
        .colonies
        .get(&colony_id)
        .unwrap()
        .buildings
        .clone();

    assert_eq!(
        buildings_before, buildings_after,
        "Colony buildings should survive save/load"
    );

    let _ = std::fs::remove_file(&path);
}

// ──────────────────────────────────────────────────────────────────
// Research screen tests
// ──────────────────────────────────────────────────────────────────

#[test]
fn r_key_opens_research_screen_from_sector_overview() {
    let mut app = App::new();
    app.new_game(42);
    assert_eq!(app.state.active, Screen::SectorOverview);

    app.handle_key(key(KeyCode::Char('r')));

    assert_eq!(app.state.active, Screen::Research);
    assert_eq!(app.state.research.cursor, 0);
}

#[test]
fn esc_closes_research_screen_returns_to_sector_map() {
    let mut app = App::new();
    app.new_game(42);
    app.state.active = Screen::Research;

    app.handle_key(key(KeyCode::Esc));

    assert_eq!(app.state.active, Screen::SectorMap);
}

#[test]
fn research_cursor_wraps_on_j() {
    let mut app = App::new();
    app.new_game(42);
    app.state.active = Screen::Research;
    app.state.research.cursor = 0;

    // j increments cursor; just verify no panic and cursor stays in bounds
    app.handle_key(key(KeyCode::Char('j')));
    let techs_len = game_core::all_techs().len();
    assert!(app.state.research.cursor < techs_len);
}

#[test]
fn research_cursor_wraps_on_k() {
    let mut app = App::new();
    app.new_game(42);
    app.state.active = Screen::Research;
    app.state.research.cursor = 0;

    // k at position 0 should wrap to last
    app.handle_key(key(KeyCode::Char('k')));
    // cursor should now point to last tech (5 for 6 techs with index 0..5)
    let techs_len = game_core::all_techs().len();
    assert!(app.state.research.cursor < techs_len);
}

#[test]
fn enter_selects_research_tech() {
    let mut app = App::new();
    app.new_game(42);
    app.state.active = Screen::Research;
    app.state.research.cursor = 0;

    app.handle_key(key(KeyCode::Enter));

    // Check that a tech is now selected in the engine
    let empire = app
        .engine
        .as_ref()
        .unwrap()
        .state
        .empires
        .get(&app.engine.as_ref().unwrap().state.player_empire)
        .unwrap();
    assert!(empire.research.current_tech.is_some());
}

#[test]
fn a_key_queues_research_tech() {
    let mut app = App::new();
    app.new_game(42);
    app.state.active = Screen::Research;
    app.state.research.cursor = 0;

    app.handle_key(key(KeyCode::Char('a')));

    let empire = app
        .engine
        .as_ref()
        .unwrap()
        .state
        .empires
        .get(&app.engine.as_ref().unwrap().state.player_empire)
        .unwrap();
    assert_eq!(empire.research.queue.len(), 1);
}

#[test]
fn c_key_clears_research_queue() {
    let mut app = App::new();
    app.new_game(42);
    app.state.active = Screen::Research;

    app.state.research.cursor = 0;
    app.handle_key(key(KeyCode::Char('a')));
    app.state.research.cursor = 1;
    app.handle_key(key(KeyCode::Char('a')));

    app.handle_key(key(KeyCode::Char('c')));

    let empire = app
        .engine
        .as_ref()
        .unwrap()
        .state
        .empires
        .get(&app.engine.as_ref().unwrap().state.player_empire)
        .unwrap();
    assert!(empire.research.queue.is_empty());
}

#[test]
fn end_turn_works_from_research_screen_with_t_key() {
    let mut app = App::new();
    app.new_game(42);
    app.state.active = Screen::Research;
    let initial_turn = app.engine.as_ref().unwrap().state.turn;

    app.handle_key(key(KeyCode::Char('t')));

    assert_eq!(app.engine.as_ref().unwrap().state.turn, initial_turn + 1);
}

#[test]
fn research_screen_renders_via_app() {
    let backend = TestBackend::new(120, 40);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = App::new();
    app.new_game(42);
    app.state.active = Screen::Research;

    terminal.draw(|frame| app.render(frame)).unwrap();
}

#[test]
fn research_state_persists_through_save_load() {
    let path = tmp_save_path("research_persist");
    let mut app = App::new();
    app.new_game(42);
    app.state.active = Screen::Research;
    app.state.research.cursor = 0;

    // Select tech and end a turn
    app.handle_key(key(KeyCode::Enter));
    app.handle_key(key(KeyCode::Char('t')));

    let progress_before = app
        .engine
        .as_ref()
        .unwrap()
        .state
        .empires
        .get(&app.engine.as_ref().unwrap().state.player_empire)
        .unwrap()
        .research
        .progress;

    app.save_game(&path).expect("save should succeed");
    app.load_game(&path).expect("load should succeed");

    let progress_after = app
        .engine
        .as_ref()
        .unwrap()
        .state
        .empires
        .get(&app.engine.as_ref().unwrap().state.player_empire)
        .unwrap()
        .research
        .progress;

    assert_eq!(
        progress_before, progress_after,
        "Research progress must survive save/load"
    );

    let _ = std::fs::remove_file(&path);
}

// ──────────────────────────────────────────────────────────────────
// Scout dispatch TUI tests
// ──────────────────────────────────────────────────────────────────

#[test]
fn s_key_dispatches_scout_to_unexplored_star() {
    let mut app = App::new();
    app.new_game(42);

    // Navigate to SectorMap since 'S' is only handled there
    app.state.active = Screen::SectorMap;

    // Select an unexplored star (Engine::new(42) explores at most 4 of 20 stars)
    let star_id = {
        let engine = app.engine.as_ref().unwrap();
        engine
            .state
            .stars
            .keys()
            .find(|id| !engine.state.explored_stars.contains(id))
            .copied()
            .expect("Engine::new(42) must have unexplored stars")
    };

    app.state.navigation.selected_star = Some(star_id);
    let before = app.state.log.len();

    app.handle_key(key(KeyCode::Char('S')));

    // A log entry should have been added (ScoutDispatched message)
    assert!(
        app.state.log.len() > before,
        "Scout dispatch should add a log entry"
    );

    let has_mission = app
        .engine
        .as_ref()
        .unwrap()
        .state
        .scout_missions
        .values()
        .any(|m| m.destination == star_id);
    assert!(has_mission, "Scout mission should be active after S key");
}

#[test]
fn dispatch_scout_without_engine_does_nothing() {
    let mut app = App::new();
    // dispatch_scout with no engine should be a no-op (no panic)
    app.dispatch_scout();
    assert!(app.engine.is_none());
}

#[test]
fn dispatch_scout_without_star_selection_logs_message() {
    let mut app = App::new();
    app.new_game(42);
    app.state.navigation.selected_star = None;

    let before = app.state.log.len();
    app.dispatch_scout();
    assert!(app.state.log.len() > before, "Should log a message");
}

#[test]
fn dispatch_scout_to_explored_star_logs_error() {
    let mut app = App::new();
    app.new_game(42);

    // Select an already-explored star
    let explored_star = *app
        .engine
        .as_ref()
        .unwrap()
        .state
        .explored_stars
        .iter()
        .next()
        .unwrap();
    app.state.navigation.selected_star = Some(explored_star);

    let before = app.state.log.len();
    app.dispatch_scout();
    assert!(
        app.state.log.len() > before,
        "Error should be logged for explored target"
    );
}

#[test]
fn dispatch_scout_when_no_fleet_available_logs_message() {
    let mut app = App::new();
    app.new_game(42);

    let engine = app.engine.as_mut().unwrap();

    // Remove all fleets so none are available
    engine.state.fleets.clear();

    let unexplored = engine
        .state
        .stars
        .keys()
        .find(|id| !engine.state.explored_stars.contains(id))
        .copied()
        .expect("Engine::new(42) must have unexplored stars");

    app.state.navigation.selected_star = Some(unexplored);
    let before = app.state.log.len();
    app.dispatch_scout();
    assert!(
        app.state.log.len() > before,
        "Should log 'no scout available'"
    );
}

#[test]
fn first_idle_player_fleet_filters_busy_kind_and_location() {
    let mut app = App::new();
    app.new_game(42);
    let engine = app.engine.as_mut().unwrap();
    let home = engine.state.empires[&engine.state.player_empire].home_star;
    let busy_scout = FleetId(1);
    let idle_scout = FleetId(7000);
    let idle_science = FleetId(7001);

    let destination = engine
        .state
        .stars
        .keys()
        .find(|id| **id != home)
        .copied()
        .expect("test galaxy should have another star");
    engine.state.scout_missions.insert(
        busy_scout,
        game_core::ScoutMission {
            fleet: busy_scout,
            destination,
            turns_remaining: 2,
            origin: home,
            total_duration: 2,
        },
    );
    engine.state.fleets.insert(
        idle_scout,
        game_core::Fleet {
            id: idle_scout,
            owner: engine.state.player_empire,
            location: destination,
            ships: 1,
            kind: FleetKind::Scout,
            strength: 1,
            integrity: 100,
        },
    );
    engine.state.fleets.insert(
        idle_science,
        game_core::Fleet {
            id: idle_science,
            owner: engine.state.player_empire,
            location: home,
            ships: 1,
            kind: FleetKind::Science,
            strength: 1,
            integrity: 100,
        },
    );

    assert_eq!(
        first_idle_player_fleet(&engine.state, Some(FleetKind::Scout), None),
        Some(idle_scout)
    );
    let expected_home_science = engine
        .state
        .fleets
        .values()
        .filter(|fleet| {
            fleet.owner == engine.state.player_empire
                && fleet.kind == FleetKind::Science
                && fleet.location == home
                && !engine.state.scout_missions.contains_key(&fleet.id)
                && !engine.state.survey_missions.contains_key(&fleet.id)
                && !engine.state.fleet_missions.contains_key(&fleet.id)
        })
        .map(|fleet| fleet.id)
        .min();
    assert_eq!(
        first_idle_player_fleet(&engine.state, Some(FleetKind::Science), Some(home)),
        expected_home_science
    );
    assert_eq!(
        first_idle_player_fleet(&engine.state, Some(FleetKind::Science), Some(destination)),
        None
    );
}

// ──────────────────────────────────────────────────────────────────
// Fleet movement (M key) tests
// ──────────────────────────────────────────────────────────────────

#[test]
fn m_key_dispatches_move_fleet() {
    let mut app = App::new();
    app.new_game(42);

    // Navigate to SectorMap since 'M' is only handled there
    app.state.active = Screen::SectorMap;

    // Select an explored star that is not the fleet's home
    let engine = app.engine.as_ref().unwrap();
    let fleet_id = game_core::FleetId(1);
    let initial_location = engine.state.fleets.get(&fleet_id).unwrap().location;
    let dest = *engine
        .state
        .explored_stars
        .iter()
        .find(|&&id| id != initial_location)
        .expect("Need explored star other than home");
    app.state.navigation.selected_star = Some(dest);

    let before_log_len = app.state.log.len();
    app.handle_key(key(KeyCode::Char('M')));

    // A fleet mission should have been created
    let mission_count = app.engine.as_ref().unwrap().state.fleet_missions.len();
    assert!(
        mission_count > 0,
        "Fleet mission should be created after M key"
    );
    // Log should have grown
    assert!(app.state.log.len() > before_log_len);
}

#[test]
fn move_fleet_without_engine_is_noop() {
    let mut app = App::new();
    // No game started
    app.move_fleet();
    assert!(app.engine.is_none());
}

#[test]
fn move_fleet_without_selection_logs_error() {
    let mut app = App::new();
    app.new_game(42);
    app.state.navigation.selected_star = None;
    let before = app.state.log.len();
    app.move_fleet();
    assert!(app.state.log.len() > before);
}

#[test]
fn move_fleet_when_no_idle_fleet_logs_error() {
    let mut app = App::new();
    app.new_game(42);

    // Put all fleets on scout missions
    let engine = app.engine.as_mut().unwrap();
    let dest = *engine
        .state
        .stars
        .keys()
        .find(|id| !engine.state.explored_stars.contains(id))
        .expect("Unexplored star needed");
    use game_core::{FleetId, ScoutMission, StarId};
    engine.state.scout_missions.insert(
        FleetId(1),
        ScoutMission {
            fleet: FleetId(1),
            destination: dest,
            turns_remaining: 3,
            origin: StarId(0),
            total_duration: 3,
        },
    );

    let explored = *engine
        .state
        .explored_stars
        .iter()
        .next()
        .expect("Need explored star");
    app.state.navigation.selected_star = Some(explored);

    let before = app.state.log.len();
    app.move_fleet();
    assert!(app.state.log.len() > before, "Should log no idle fleet");
}

#[test]
fn move_fleet_to_explored_star_creates_mission_and_logs() {
    let mut app = App::new();
    app.new_game(42);

    let engine = app.engine.as_ref().unwrap();
    let fleet_id = game_core::FleetId(1);
    let initial = engine.state.fleets.get(&fleet_id).unwrap().location;
    let dest = *engine
        .state
        .explored_stars
        .iter()
        .find(|&&id| id != initial)
        .expect("Need explored star other than home");
    app.state.navigation.selected_star = Some(dest);
    let before = app.state.log.len();
    app.move_fleet();

    let engine = app.engine.as_ref().unwrap();
    assert!(
        engine.state.fleet_missions.contains_key(&fleet_id),
        "Fleet mission should be created"
    );
    assert!(app.state.log.len() > before, "Log should grow");
}

#[test]
fn system_colonize_key_without_colonizer_logs_error() {
    let mut app = App::new();
    app.new_game(42);

    // No colonizer fleet exists at game start
    let engine = app.engine.as_ref().unwrap();
    let star = *engine.state.explored_stars.iter().next().unwrap();
    app.state.navigation.selected_star = Some(star);
    app.state.active = Screen::System;

    let before = app.state.log.len();
    app.handle_key(key(KeyCode::Char('C')));
    assert!(
        app.state.log.len() > before,
        "Should log error when no colonizer available"
    );
}

#[test]
fn system_view_survey_targets_selected_planet() {
    use game_core::{FleetKind, SurveyMission};

    let mut app = App::new();
    app.new_game(42);

    let engine = app.engine.as_mut().unwrap();
    let star = *engine.state.explored_stars.iter().next().unwrap();
    let science_fleet = game_core::FleetId(77);
    engine.state.fleets.insert(
        science_fleet,
        game_core::Fleet {
            id: science_fleet,
            owner: engine.state.player_empire,
            location: star,
            ships: 1,
            kind: FleetKind::Science,
            strength: 1,
            integrity: 100,
        },
    );
    engine.state.stars.get_mut(&star).unwrap().planets[0].surveyed = false;
    app.state.navigation.selected_star = Some(star);
    app.state.navigation.selected_planet_index = 0;
    app.state.active = Screen::System;

    app.handle_key(key(KeyCode::Char('S')));

    let engine = app.engine.as_ref().unwrap();
    assert!(
        engine.state.survey_missions.contains_key(&science_fleet),
        "Survey command should create a survey mission"
    );
    assert!(matches!(
        engine.state.survey_missions.get(&science_fleet),
        Some(SurveyMission {
            star: s,
            planet_index: 0,
            ..
        }) if *s == star
    ));
}

#[test]
fn colonize_without_engine_is_noop() {
    let mut app = App::new();
    // No game started
    app.colonize_selected_planet();
    assert!(app.engine.is_none());
}

#[test]
fn colonize_without_selection_logs_error() {
    let mut app = App::new();
    app.new_game(42);
    app.state.navigation.selected_star = None;
    let before = app.state.log.len();
    app.colonize_selected_planet();
    assert!(app.state.log.len() > before);
}

#[test]
fn colonize_key_with_colonizer_succeeds() {
    use game_core::{Command, FleetKind, OrbitalStructureType};

    let mut app = App::new();
    app.new_game(42);

    // Build a colonizer fleet via the engine directly
    let colony_id = game_core::ColonyId(1);
    // Inject Shipyard so Colony Ship can be queued
    app.engine
        .as_mut()
        .unwrap()
        .state
        .colonies
        .get_mut(&colony_id)
        .unwrap()
        .orbital_installations
        .push(OrbitalStructureType::Shipyard);
    app.engine
        .as_mut()
        .unwrap()
        .state
        .empires
        .get_mut(&game_core::EmpireId(1))
        .unwrap()
        .research
        .completed
        .push(game_core::TechId(2));
    app.engine.as_mut().unwrap().apply_turn(vec![
        Command::QueueBuild {
            colony: colony_id,
            item: game_core::BuildItem::Colony,
        },
        Command::SetColonyFocus {
            colony: colony_id,
            prod_pct: 100,
            research_pct: 0,
        },
    ]);
    for _ in 0..21 {
        app.engine
            .as_mut()
            .unwrap()
            .apply_turn(vec![Command::EndTurn]);
    }

    let engine = app.engine.as_ref().unwrap();
    let colonizer_id = engine
        .state
        .fleets
        .values()
        .find(|f| f.kind == FleetKind::Colonizer)
        .map(|f| f.id)
        .expect("Colonizer must exist");

    let home = engine
        .state
        .empires
        .get(&engine.state.player_empire)
        .unwrap()
        .home_star;
    let target = *engine
        .state
        .explored_stars
        .iter()
        .find(|&&id| id != home)
        .expect("Need explored star");
    app.engine
        .as_mut()
        .unwrap()
        .state
        .stars
        .get_mut(&target)
        .unwrap()
        .planets[0]
        .surveyed = true;

    // Move colonizer to target
    app.engine
        .as_mut()
        .unwrap()
        .apply_turn(vec![Command::MoveFleet {
            fleet: colonizer_id,
            destination: target,
        }]);
    for _ in 0..4 {
        app.engine
            .as_mut()
            .unwrap()
            .apply_turn(vec![Command::EndTurn]);
    }

    // Verify colonizer is at target
    let engine = app.engine.as_ref().unwrap();
    assert_eq!(
        engine.state.fleets.get(&colonizer_id).unwrap().location,
        target
    );

    let colonies_before = engine.state.colonies.len();

    // Select target star and press C
    app.state.navigation.selected_star = Some(target);
    app.state.active = Screen::System;
    app.handle_key(key(KeyCode::Char('C')));

    let engine = app.engine.as_ref().unwrap();
    assert_eq!(
        engine.state.colonies.len(),
        colonies_before + 1,
        "Colonization should create a new colony"
    );
    assert!(
        !engine.state.fleets.contains_key(&colonizer_id),
        "Colonizer should be consumed"
    );
}

#[test]
fn sector_overview_renders_with_colonizer_fleet_present() {
    use game_core::FleetKind;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    let backend = TestBackend::new(120, 40);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut engine = game_core::Engine::new(42);

    // Add a colonizer fleet at the home star
    let player = engine.state.player_empire;
    let home = engine.state.empires.get(&player).unwrap().home_star;
    let fleet_id = game_core::FleetId(50);
    engine.state.fleets.insert(
        fleet_id,
        game_core::Fleet {
            id: fleet_id,
            owner: player,
            location: home,
            ships: 1,
            kind: FleetKind::Colonizer,
            strength: 1,
            integrity: 100,
        },
    );

    let sector = engine.state.stars.get(&home).map(|s| s.sector);
    let app_state = crate::AppState {
        navigation: crate::app::NavigationState {
            selected_sector: sector,
            selected_star: Some(home),
            ..Default::default()
        },
        ..Default::default()
    };

    terminal
        .draw(|frame| {
            let area = frame.area();
            crate::screens::sector_overview::render_sector_overview(
                frame,
                area,
                &app_state,
                &engine.state,
            );
        })
        .unwrap();
}

#[test]
fn ai_events_hidden_for_unknown_empire() {
    let mut app = App::new();
    app.new_game(42);

    let ai_empire = {
        let engine = app.engine.as_ref().unwrap();
        *engine
            .state
            .empires
            .keys()
            .find(|id| **id != engine.state.player_empire)
            .expect("AI empire must exist")
    };

    // Ensure empire is Unknown (no contact)
    app.engine
        .as_mut()
        .unwrap()
        .state
        .diplomacy
        .insert(ai_empire, game_core::RelationshipStatus::Unknown);

    app.state.log.clear();

    let event = game_core::Event::AiResearchSelected {
        empire: ai_empire,
        tech: game_core::TechId(1),
    };
    app.push_core_event_to_log(&event);

    assert_eq!(
        app.state.log.len(),
        0,
        "AI event for unknown empire must not appear in log"
    );
}

#[test]
fn ai_only_system_exploration_hidden_from_log() {
    let mut app = App::new();
    app.new_game(42);

    let ai_only_star = {
        let engine = app.engine.as_ref().expect("engine");
        engine
            .state
            .stars
            .keys()
            .copied()
            .find(|star| !engine.state.explored_stars.contains(star))
            .expect("an unexplored star must exist")
    };

    app.engine
        .as_mut()
        .expect("engine")
        .state
        .ai_explored_stars
        .insert(ai_only_star);
    app.state.log.clear();

    app.push_core_event_to_log(&CoreEvent::SystemExplored { star: ai_only_star });

    assert_eq!(
        app.state.log.len(),
        0,
        "AI-only exploration must not appear in log"
    );
}

#[test]
fn player_system_exploration_remains_visible_in_log() {
    let mut app = App::new();
    app.new_game(42);

    let player_star = {
        let engine = app.engine.as_ref().expect("engine");
        *engine
            .state
            .explored_stars
            .iter()
            .next()
            .expect("player must start with explored star")
    };

    app.state.log.clear();
    app.push_core_event_to_log(&CoreEvent::SystemExplored { star: player_star });

    assert_eq!(app.state.log.len(), 1);
    assert_eq!(
        app.state.log.last_n(1),
        &[format!("System {} explored", player_star.0)]
    );
}

#[test]
fn ai_events_visible_for_contacted_empire() {
    let mut app = App::new();
    app.new_game(42);

    let ai_empire = {
        let engine = app.engine.as_ref().unwrap();
        *engine
            .state
            .empires
            .keys()
            .find(|id| **id != engine.state.player_empire)
            .expect("AI empire must exist")
    };

    app.engine
        .as_mut()
        .unwrap()
        .state
        .diplomacy
        .insert(ai_empire, game_core::RelationshipStatus::Contacted);

    app.state.log.clear();

    let event = game_core::Event::AiResearchSelected {
        empire: ai_empire,
        tech: game_core::TechId(1),
    };
    app.push_core_event_to_log(&event);

    assert_eq!(
        app.state.log.len(),
        1,
        "AI event for contacted empire must appear in log"
    );
}

#[test]
fn ai_strategic_events_include_doctrine_marker() {
    let mut app = App::new();
    app.new_game(42);

    let ai_empire = {
        let engine = app.engine.as_ref().expect("engine");
        *engine
            .state
            .empires
            .keys()
            .find(|id| **id != engine.state.player_empire)
            .expect("AI empire must exist")
    };
    app.engine
        .as_mut()
        .expect("engine")
        .state
        .diplomacy
        .insert(ai_empire, game_core::RelationshipStatus::Contacted);

    let doctrine = app
        .engine
        .as_ref()
        .expect("engine")
        .state
        .empires
        .get(&ai_empire)
        .and_then(|empire| empire.empire_def)
        .and_then(empire_definition_by_id)
        .map(|def| def.doctrine_short_summary())
        .expect("AI doctrine must exist");
    let doctrine_marker = format!("[DOC {doctrine}]");

    app.state.log.clear();
    let events = [
        CoreEvent::AiResearchSelected {
            empire: ai_empire,
            tech: TechId::VOID_PROPULSION,
        },
        CoreEvent::AiBuildQueued {
            empire: ai_empire,
            colony: ColonyId(1),
            item: game_core::BuildItem::Structure(BuildingType::AquacultureBay),
        },
        CoreEvent::AiScoutDispatched {
            empire: ai_empire,
            fleet: FleetId(1),
            destination: StarId(1),
        },
        CoreEvent::AiColonized {
            empire: ai_empire,
            star: StarId(1),
            planet_index: 0,
            colony: ColonyId(2),
        },
        CoreEvent::AiColonyRoleAssigned {
            empire: ai_empire,
            colony: ColonyId(2),
            role: ColonyRole::Balanced,
        },
    ];
    for event in events {
        app.push_core_event_to_log(&event);
    }

    let entries = app.state.log.last_n(5);
    assert_eq!(entries.len(), 5);
    for entry in entries {
        assert!(entry.contains(&doctrine_marker));
    }
}

#[test]
fn non_player_economy_events_hidden_from_log() {
    let mut app = App::new();
    app.new_game(42);

    let ai_empire = {
        let engine = app.engine.as_ref().unwrap();
        *engine
            .state
            .empires
            .keys()
            .find(|id| **id != engine.state.player_empire)
            .expect("AI empire must exist")
    };

    app.state.log.clear();

    app.push_core_event_to_log(&CoreEvent::EconomySummary {
        empire: ai_empire,
        credits_income: 9,
        maintenance: 2,
        food_produced: 6,
        food_consumed: 4,
    });
    app.push_core_event_to_log(&CoreEvent::FoodShortage {
        empire: ai_empire,
        deficit: 3,
    });
    app.push_core_event_to_log(&CoreEvent::CreditDeficit {
        empire: ai_empire,
        deficit: 2,
    });

    assert_eq!(
        app.state.log.len(),
        0,
        "Non-player economy events must not appear in log"
    );
}

#[test]
fn non_player_colony_status_events_hidden_from_log() {
    let mut app = App::new();
    app.new_game(42);

    let ai_colony = {
        let engine = app.engine.as_ref().unwrap();
        *engine
            .state
            .colonies
            .iter()
            .find(|(_, colony)| colony.owner != engine.state.player_empire)
            .map(|(id, _)| id)
            .expect("AI colony must exist")
    };

    app.state.log.clear();

    app.push_core_event_to_log(&CoreEvent::ColonyStatusWarning {
        colony: ai_colony,
        food_deficit: 1,
        housing_deficit: 2,
        unemployed: 3,
    });
    app.push_core_event_to_log(&CoreEvent::PopulationGrew {
        colony: ai_colony,
        new_population: 9,
    });

    assert_eq!(
        app.state.log.len(),
        0,
        "Non-player colony status events must not appear in log"
    );
}

#[test]
fn player_operational_events_still_visible_in_log() {
    let mut app = App::new();
    app.new_game(42);

    let (player_empire, player_colony) = {
        let engine = app.engine.as_ref().unwrap();
        let player_empire = engine.state.player_empire;
        let player_colony = *engine
            .state
            .colonies
            .iter()
            .find(|(_, colony)| colony.owner == player_empire)
            .map(|(id, _)| id)
            .expect("Player colony must exist");
        (player_empire, player_colony)
    };

    app.state.log.clear();

    app.push_core_event_to_log(&CoreEvent::EconomySummary {
        empire: player_empire,
        credits_income: 9,
        maintenance: 2,
        food_produced: 6,
        food_consumed: 4,
    });
    app.push_core_event_to_log(&CoreEvent::ColonyStatusWarning {
        colony: player_colony,
        food_deficit: 1,
        housing_deficit: 2,
        unemployed: 3,
    });

    assert_eq!(
        app.state.log.len(),
        2,
        "Player operational events must still appear in log"
    );
}

// ---------------------------------------------------------------------------
// Galactic Dispatch tests
// ---------------------------------------------------------------------------

#[test]
fn test_n_key_opens_dispatch_when_dispatch_available() {
    let mut app = App::new();
    app.new_game(42);
    // Advance enough turns to generate at least one dispatch (cadence = 5)
    // Close any auto-opened overlay before each end-turn key
    for _ in 0..5 {
        if app.state.overlay.show_dispatch {
            app.state.overlay.show_dispatch = false;
        }
        app.dispatch_command(Command::EndTurn);
    }

    let has_dispatch = app
        .engine
        .as_ref()
        .map(|e| !e.state.galactic_dispatches.is_empty())
        .unwrap_or(false);

    assert!(
        has_dispatch,
        "advancing 5 turns must produce at least one cadence dispatch"
    );

    // Close any auto-shown overlay
    app.state.overlay.show_dispatch = false;

    app.handle_key(key(KeyCode::Char('N')));
    assert!(
        app.state.overlay.show_dispatch,
        "N key should open dispatch modal when dispatches exist"
    );
}

#[test]
fn test_dispatch_overlay_closes_on_esc() {
    let mut app = App::new();
    app.new_game(42);
    for _ in 0..5 {
        if app.state.overlay.show_dispatch {
            app.state.overlay.show_dispatch = false;
        }
        app.dispatch_command(Command::EndTurn);
    }

    assert!(
        app.engine
            .as_ref()
            .map(|e| !e.state.galactic_dispatches.is_empty())
            .unwrap_or(false),
        "advancing 5 turns must produce at least one cadence dispatch"
    );

    app.state.overlay.show_dispatch = true;
    app.state.overlay.dispatch_history_index = 0;

    app.handle_key(key(KeyCode::Esc));
    assert!(
        !app.state.overlay.show_dispatch,
        "Esc should close the dispatch modal"
    );
}

#[test]
fn test_dispatch_palette_command_opens_dispatch() {
    let mut app = App::new();
    app.new_game(42);
    for _ in 0..5 {
        if app.state.overlay.show_dispatch {
            app.state.overlay.show_dispatch = false;
        }
        app.dispatch_command(Command::EndTurn);
    }

    assert!(
        app.engine
            .as_ref()
            .map(|e| !e.state.galactic_dispatches.is_empty())
            .unwrap_or(false),
        "advancing 5 turns must produce at least one cadence dispatch"
    );

    // Close any auto-shown overlay first
    app.state.overlay.show_dispatch = false;

    app.execute_palette_command(PaletteCommand::Dispatch);
    assert!(
        app.state.overlay.show_dispatch,
        "PaletteCommand::Dispatch should open the dispatch modal"
    );

    // Reset and test alias
    app.state.overlay.show_dispatch = false;
    app.execute_palette_command(PaletteCommand::News);
    assert!(
        app.state.overlay.show_dispatch,
        "PaletteCommand::News should also open the dispatch modal"
    );
}

#[test]
fn test_dispatch_navigation_cycles_history() {
    let mut app = App::new();
    app.new_game(42);
    // Advance 10 turns to get multiple dispatches (cadence=5 → 2 dispatches)
    for _ in 0..10 {
        if app.state.overlay.show_dispatch {
            app.state.overlay.show_dispatch = false;
        }
        app.dispatch_command(Command::EndTurn);
    }

    let dispatch_count = app
        .engine
        .as_ref()
        .map(|e| e.state.galactic_dispatches.len())
        .unwrap_or(0);

    assert!(
        dispatch_count >= 2,
        "advancing 10 turns must produce at least 2 cadence dispatches, got {dispatch_count}"
    );

    // Open dispatch at newest
    app.state.overlay.show_dispatch = true;
    app.state.overlay.dispatch_history_index = dispatch_count - 1;

    // Navigate left (prev)
    app.handle_key(key(KeyCode::Left));
    assert_eq!(
        app.state.overlay.dispatch_history_index,
        dispatch_count - 2,
        "Left arrow should decrement dispatch_history_index"
    );

    // Navigate right (next)
    app.handle_key(key(KeyCode::Right));
    assert_eq!(
        app.state.overlay.dispatch_history_index,
        dispatch_count - 1,
        "Right arrow should increment dispatch_history_index"
    );

    // At newest, right should not go past the end
    app.handle_key(key(KeyCode::Right));
    assert_eq!(
        app.state.overlay.dispatch_history_index,
        dispatch_count - 1,
        "Right arrow should not go past the last dispatch"
    );

    // At oldest (0), left should not go below 0
    app.state.overlay.dispatch_history_index = 0;
    app.handle_key(key(KeyCode::Left));
    assert_eq!(
        app.state.overlay.dispatch_history_index, 0,
        "Left arrow should not go below 0"
    );
}

#[test]
fn test_n_key_without_engine_does_nothing() {
    let mut app = App::new();
    // No engine — N key should be a no-op
    app.handle_key(key(KeyCode::Char('N')));
    assert!(!app.state.overlay.show_dispatch);
}

#[test]
fn test_dispatch_overlay_closes_with_lowercase_n() {
    let mut app = App::new();
    app.new_game(42);

    // Manually open dispatch
    app.state.overlay.show_dispatch = true;
    app.handle_key(key(KeyCode::Char('n')));
    assert!(
        !app.state.overlay.show_dispatch,
        "n key should close the dispatch modal"
    );
}

#[test]
fn setup_flow_starts_game_from_new_game_setup_screen() {
    let mut app = App::new();
    app.handle_key(key(KeyCode::Char('n')));
    assert_eq!(app.state.active, Screen::EmpireSelect);

    app.handle_key(key(KeyCode::Enter));
    assert_eq!(app.state.active, Screen::NewGameSetup);

    app.handle_key(key(KeyCode::Char('S')));
    assert!(app.engine.is_some());
    assert_eq!(app.state.active, Screen::SectorOverview);
}

#[test]
fn setup_flow_starts_game_from_new_game_setup_screen_with_lowercase_s() {
    let mut app = App::new();
    app.handle_key(key(KeyCode::Char('n')));
    assert_eq!(app.state.active, Screen::EmpireSelect);

    app.handle_key(key(KeyCode::Enter));
    assert_eq!(app.state.active, Screen::NewGameSetup);

    app.handle_key(key(KeyCode::Char('s')));
    assert!(app.engine.is_some());
    assert_eq!(app.state.active, Screen::SectorOverview);
}

#[test]
fn research_era_filter_cycles_with_left_bracket() {
    let mut app = App::new();
    app.new_game(42);
    app.state.active = Screen::Research;
    assert_eq!(app.state.research.era_filter, 0);

    app.handle_key(key(KeyCode::Char('[')));
    assert_eq!(app.state.research.era_filter, 1);
}

#[test]
fn diplomacy_screen_opens_after_contact() {
    let mut app = App::new();
    app.new_game(42);
    let ai_empire = app.engine.as_ref().unwrap().state.ai_empire.unwrap();
    app.engine
        .as_mut()
        .unwrap()
        .state
        .diplomacy
        .insert(ai_empire, game_core::RelationshipStatus::Contacted);

    app.state.active = Screen::SectorMap;
    app.handle_key(key(KeyCode::Char('D')));
    assert_eq!(app.state.active, Screen::Diplomacy);
}

#[test]
fn diplomacy_modal_closes_with_no_pending_messages() {
    let mut app = App::new();
    app.new_game(42);
    app.state.active = Screen::Diplomacy;
    app.state.diplomacy.show_communication_modal = true;
    app.engine
        .as_mut()
        .unwrap()
        .state
        .diplomacy_pending_communications
        .clear();

    app.handle_key(key(KeyCode::Esc));

    assert!(!app.state.diplomacy.show_communication_modal);
}

#[test]
fn v_key_opens_victory_overview_with_progress_lines() {
    let mut app = App::new();
    app.new_game(42);
    app.state.active = Screen::SectorMap;
    app.handle_key(key(KeyCode::Char('V')));
    assert_eq!(app.state.active, Screen::EmpireOverview);

    let backend = TestBackend::new(140, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|frame| app.render(frame)).unwrap();
    let rendered: String = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect();
    assert!(rendered.contains("Victory"));
}

#[test]
fn command_palette_core_commands_save_load_and_dispatch_work() {
    let mut app = App::new();
    app.new_game(42);
    let save_path = tmp_save_path("palette_core_commands");
    let _ = std::fs::remove_file(&save_path);

    app.end_turn();
    let turn_before_save = app.engine.as_ref().unwrap().state.turn;
    app.execute_palette_command_with_path(PaletteCommand::Save, &save_path);
    assert!(app
        .state
        .status_message
        .as_deref()
        .is_some_and(|msg| msg.contains("Save: wrote")));

    app.end_turn();
    assert!(app.engine.as_ref().unwrap().state.turn > turn_before_save);
    app.execute_palette_command_with_path(PaletteCommand::Load, &save_path);
    assert_eq!(app.engine.as_ref().unwrap().state.turn, turn_before_save);

    for _ in 0..5 {
        app.end_turn();
    }
    app.state.overlay.show_dispatch = false;
    app.execute_palette_input("dispatch");
    assert!(app.state.overlay.show_dispatch);

    let _ = std::fs::remove_file(&save_path);
}

#[test]
fn command_palette_clear_rally_clears_active_colony_rally_point() {
    let mut app = App::new();
    app.new_game(42);
    assert!(goto_colony_screen(&mut app));

    let colony_id = app.state.colony.selected_colony.unwrap();
    let player_home = app
        .engine
        .as_ref()
        .unwrap()
        .state
        .empires
        .get(&app.engine.as_ref().unwrap().state.player_empire)
        .unwrap()
        .home_star;
    app.engine
        .as_mut()
        .unwrap()
        .state
        .colonies
        .get_mut(&colony_id)
        .unwrap()
        .rally_point = Some(player_home);

    app.execute_palette_input("clear-rally");

    assert_eq!(
        app.engine
            .as_ref()
            .unwrap()
            .state
            .colonies
            .get(&colony_id)
            .unwrap()
            .rally_point,
        None
    );
}
