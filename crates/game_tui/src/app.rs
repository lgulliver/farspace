//! Application state and main run loop

mod logging;

use crate::components::{
    render_dispatch, render_help, render_palette, EventLog, LogEntryKind, PaletteCommand,
};
use crate::keys::KeyMap;
use crate::screens::empire_overview::{derive_empire_overview, EmpireOverviewData, OverviewSort};
use crate::screens::research::{
    filtered_research_techs, RESEARCH_DOMAIN_FILTER_COUNT, RESEARCH_STATUS_FILTER_COUNT,
};
use crate::screens::Screen;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use game_core::{
    empire_definition_by_id, tech_by_id, BuildingType, ColonyId, ColonyRole, Command, Engine,
    Event as CoreEvent, FleetId, FleetKind, GalaxySize, OrbitalStructureType, ScenarioSetup,
    SectorId, StarId, TechId,
};
use ratatui::{backend::Backend, Frame, Terminal};
use std::io;
use std::time::Duration;

/// Default save file path
const DEFAULT_SAVE_PATH: &str = "farspace.sav";

/// Main application state
pub struct App {
    state: AppState,
    engine: Option<Engine>,
}

/// UI state
#[derive(Debug, Clone, Default)]
pub struct AppState {
    pub(crate) active: Screen,
    pub(crate) overlay: OverlayState,
    pub(crate) navigation: NavigationState,
    pub(crate) sector_overview: SectorOverviewState,
    pub(crate) colony: ColonyScreenState,
    pub(crate) research: ResearchScreenState,
    pub(crate) overview: EmpireOverviewScreenState,
    pub(crate) new_game_setup: NewGameSetupState,
    pub(crate) log: EventLog,
    pub(crate) quit: bool,
    /// Monotonically-increasing frame counter, incremented once per render loop iteration.
    /// Used only for low-frequency UI animations; never affects simulation state.
    pub(crate) tick_count: u64,
    /// When true, all fleet travel animations are suppressed (accessibility / low-motion).
    pub(crate) reduced_motion: bool,
    /// Status line shown in contextual footer hints.
    pub(crate) status_message: Option<String>,
}

/// UI overlay state shared by all screens.
#[derive(Debug, Clone, Default)]
pub(crate) struct OverlayState {
    pub(crate) show_help: bool,
    pub(crate) show_palette: bool,
    pub(crate) palette_input: String,
    /// Whether the Galactic Dispatch modal is open
    pub(crate) show_dispatch: bool,
    /// Index into the dispatch history (0 = oldest shown)
    pub(crate) dispatch_history_index: usize,
}

/// Cross-screen map/system selection state.
#[derive(Debug, Clone, Default)]
pub(crate) struct NavigationState {
    pub(crate) selected_sector: Option<SectorId>,
    pub(crate) selected_star: Option<StarId>,
    /// Selected planet index when inspecting a system.
    pub(crate) selected_planet_index: usize,
}

/// Sector overview screen state.
#[derive(Debug, Clone, Default)]
pub(crate) struct SectorOverviewState {
    /// Toggle for rendering inter-sector hyperspace lanes on galaxy overview.
    pub(crate) show_inter_sector_lanes: bool,
}

/// Colony screen state.
#[derive(Debug, Clone, Default)]
pub(crate) struct ColonyScreenState {
    /// Currently viewed colony (set when entering the colony screen).
    pub(crate) selected_colony: Option<ColonyId>,
    /// Cursor index for the build-picker on the colony screen.
    pub(crate) build_cursor: usize,
    /// Cursor index for the role selector on the colony screen.
    pub(crate) role_cursor: usize,
    /// Whether the role selector is the active panel (true) or build picker (false).
    pub(crate) role_panel_active: bool,
    /// Colony for which a rally point is being picked.
    ///
    /// When `Some`, the sector map shows a prompt and pressing 'R' or Enter
    /// while navigating will confirm the selected star as the rally destination.
    /// Pressing Esc cancels.
    pub(crate) pending_rally_colony: Option<ColonyId>,
}

/// Research screen state.
#[derive(Debug, Clone, Default)]
pub(crate) struct ResearchScreenState {
    /// Cursor index for the tech list on the research screen.
    pub(crate) cursor: usize,
    /// Domain filter index (0 = all, then domain order).
    pub(crate) domain_filter: usize,
    /// Era filter index (0 = all, then Era 1..6).
    pub(crate) era_filter: usize,
    /// Status filter index (0 = all, then available/locked/active/completed).
    pub(crate) status_filter: usize,
    /// Case-insensitive text query for technology name/description/tag.
    pub(crate) query: String,
    /// True while text input mode is active for query editing.
    pub(crate) query_input: bool,
}

/// Empire overview screen state.
#[derive(Debug, Clone, Default)]
pub(crate) struct EmpireOverviewScreenState {
    /// Cursor index for selected row on empire overview.
    pub(crate) cursor: usize,
    /// Current sort mode for empire overview.
    pub(crate) sort: OverviewSort,
    /// Optional overview filter query.
    pub(crate) filter: String,
    /// Whether filter input mode is active on the overview screen.
    pub(crate) filter_input: bool,
}

/// New Game Setup screen state.
#[derive(Debug, Clone)]
pub(crate) struct NewGameSetupState {
    /// Galaxy size selected on the setup screen.
    pub(crate) galaxy_size: GalaxySize,
    /// Number of AI empires selected on the setup screen (1–4).
    pub(crate) ai_count: u8,
    /// Seed string shown on the setup screen (ASCII digits only).
    pub(crate) seed_str: String,
    /// Which field is currently highlighted on the setup screen.
    /// 0 = empire selection, 1 = galaxy size, 2 = AI count, 3 = seed.
    pub(crate) cursor: usize,
    /// Whether the seed text-edit mode is active.
    pub(crate) seed_editing: bool,
    /// Snapshot of `setup_seed_str` taken when seed editing begins, used to
    /// restore the original value if the user presses Esc to discard changes.
    pub(crate) seed_pre_edit: String,
    /// Index into `all_empire_definitions()` for the empire the player has chosen.
    pub(crate) empire_cursor: usize,
}

impl Default for NewGameSetupState {
    fn default() -> Self {
        Self {
            galaxy_size: GalaxySize::Medium,
            ai_count: 1,
            seed_str: "42".to_string(),
            cursor: 0,
            seed_editing: false,
            seed_pre_edit: String::new(),
            empire_cursor: 0,
        }
    }
}

fn fleet_is_idle(state: &game_core::GameState, fleet_id: FleetId) -> bool {
    !state.scout_missions.contains_key(&fleet_id)
        && !state.survey_missions.contains_key(&fleet_id)
        && !state.fleet_missions.contains_key(&fleet_id)
}

fn first_idle_player_fleet(
    state: &game_core::GameState,
    kind: Option<FleetKind>,
    location: Option<StarId>,
) -> Option<FleetId> {
    state
        .fleets
        .values()
        .find(|fleet| {
            fleet.owner == state.player_empire
                && kind.is_none_or(|kind| fleet.kind == kind)
                && location.is_none_or(|location| fleet.location == location)
                && fleet_is_idle(state, fleet.id)
        })
        .map(|fleet| fleet.id)
}

impl App {
    /// Create a new application
    pub fn new() -> Self {
        App {
            state: AppState::default(),
            engine: None,
        }
    }

    /// Start a new game with the given seed (default setup, 1 AI empire).
    pub fn new_game(&mut self, seed: u64) {
        let setup = ScenarioSetup::default_for_seed(seed);
        self.new_game_from_setup(setup);
    }

    /// Start a new game from a `ScenarioSetup`.  Logs an error if the setup is invalid.
    pub fn new_game_from_setup(&mut self, setup: ScenarioSetup) {
        if let Err(e) = setup.validate() {
            self.state.log.push(format!("Error: Invalid setup: {}", e));
            return;
        }
        let engine = Engine::new_from_setup(setup);

        // Select the first sector and first star by default
        self.state.navigation.selected_sector = engine.state.sectors.keys().next().copied();
        self.state.navigation.selected_star = engine.state.stars.keys().next().copied();
        self.state.navigation.selected_planet_index = 0;

        // Add initial log entry with setup summary and playability hints
        let scenario_summary = if let Some(s) = &engine.state.scenario {
            let player_faction = engine
                .state
                .empires
                .get(&engine.state.player_empire)
                .and_then(|empire| empire.empire_def)
                .and_then(empire_definition_by_id)
                .map(|def| def.name)
                .unwrap_or("Unaligned");
            format!(
                "Game started — {} galaxy, {} AI empire(s), seed {}, faction {}",
                s.galaxy_size.label(),
                s.ai_empire_count,
                s.seed,
                player_faction,
            )
        } else {
            "Game started".to_string()
        };
        self.state.log.push(scenario_summary);
        self.state.log.push(
            "What to do next: Enter Sector Map, scout (S), survey (System:S), then colonize (C)."
                .to_string(),
        );
        self.state.status_message = Some(
            "First turn: Enter sector map, scout systems, choose research, queue a build."
                .to_string(),
        );

        self.engine = Some(engine);
        self.state.active = Screen::SectorOverview;
    }

    /// Save the current game to the given path. Returns an error message on failure.
    pub fn save_game(&mut self, path: &std::path::Path) -> Result<(), String> {
        let engine = self
            .engine
            .as_ref()
            .ok_or_else(|| "No game in progress".to_string())?;
        game_save::save_to_file(&engine.state, path)
            .map_err(|e| format!("Error: Save failed ({}): {}", path.display(), e))?;
        Ok(())
    }

    /// Load a game from the given path. Returns an error message on failure.
    pub fn load_game(&mut self, path: &std::path::Path) -> Result<(), String> {
        let state = game_save::load_from_file(path)
            .map_err(|e| format!("Error: Load failed ({}): {}", path.display(), e))?;
        let selected_star = state.stars.keys().next().copied();
        let selected_sector = state.sectors.keys().next().copied();
        self.engine = Some(Engine::from_state(state));
        self.state.navigation.selected_sector = selected_sector;
        self.state.navigation.selected_star = selected_star;
        self.state.navigation.selected_planet_index = 0;
        self.state.active = Screen::SectorOverview;
        Ok(())
    }

    /// Parse and execute palette input (e.g. "save", ":save").
    fn execute_palette_input(&mut self, input: &str) {
        let command = match PaletteCommand::parse(input) {
            Ok(Some(command)) => command,
            Ok(None) => return,
            Err(err) => {
                self.push_error_status(format!("Error: Unknown command: {}", err.command()));
                return;
            }
        };
        self.execute_palette_command(command);
    }

    fn execute_palette_command(&mut self, cmd: PaletteCommand) {
        let path = std::path::PathBuf::from(DEFAULT_SAVE_PATH);
        match cmd {
            PaletteCommand::Save => match self.save_game(&path) {
                Ok(()) => {
                    let msg = format!("Save: wrote {}", path.display());
                    self.push_status(LogEntryKind::SaveLoad, msg);
                }
                Err(e) => {
                    self.push_error_status(e);
                }
            },
            PaletteCommand::Load => match self.load_game(&path) {
                Ok(()) => {
                    let msg = format!("Load: loaded {}", path.display());
                    self.push_status(LogEntryKind::SaveLoad, msg);
                }
                Err(e) => {
                    self.push_error_status(e);
                }
            },
            PaletteCommand::ClearRally => {
                self.clear_rally_point();
            }
            PaletteCommand::Dispatch | PaletteCommand::News => {
                self.open_latest_dispatch();
            }
        }
    }

    /// Run the main event loop
    pub fn run<B: Backend>(mut self, terminal: &mut Terminal<B>) -> io::Result<()> {
        while !self.state.quit {
            terminal.draw(|frame| self.render(frame))?;
            // Increment animation tick counter each render frame (wraps safely at u64::MAX)
            self.state.tick_count = self.state.tick_count.wrapping_add(1);

            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        self.handle_key(key);
                    }
                }
            }
        }

        Ok(())
    }

    /// Render the current state
    fn render(&self, frame: &mut Frame) {
        let area = frame.area();

        // Render the active screen
        let game_state = self.engine.as_ref().map(|e| &e.state);
        self.state
            .active
            .render(frame, area, &self.state, game_state);

        // Render overlays
        if self.state.overlay.show_help {
            render_help(frame, area, &self.state.active);
        }

        if self.state.overlay.show_palette {
            render_palette(frame, area, &self.state.overlay.palette_input);
        }

        if self.state.overlay.show_dispatch {
            if let Some(engine) = &self.engine {
                let dispatches = &engine.state.galactic_dispatches;
                if !dispatches.is_empty() {
                    let idx = self
                        .state
                        .overlay
                        .dispatch_history_index
                        .min(dispatches.len().saturating_sub(1));
                    render_dispatch(frame, area, &dispatches[idx], idx, dispatches.len());
                }
            }
        }
    }

    /// Handle a key event
    fn handle_key(&mut self, key: KeyEvent) {
        // Handle overlays first
        if self.state.overlay.show_dispatch {
            match key.code {
                KeyCode::Esc | KeyCode::Char('N') | KeyCode::Char('n') => {
                    self.state.overlay.show_dispatch = false;
                }
                KeyCode::Left | KeyCode::Char('h') => {
                    if self.state.overlay.dispatch_history_index > 0 {
                        self.state.overlay.dispatch_history_index -= 1;
                    }
                }
                KeyCode::Right | KeyCode::Char('l') => {
                    if let Some(engine) = &self.engine {
                        let max = engine.state.galactic_dispatches.len().saturating_sub(1);
                        if self.state.overlay.dispatch_history_index < max {
                            self.state.overlay.dispatch_history_index += 1;
                        }
                    }
                }
                _ => {}
            }
            return;
        }

        if self.state.overlay.show_help {
            if KeyMap::is_help(key) || KeyMap::is_escape(key) {
                self.state.overlay.show_help = false;
            }
            return;
        }

        if self.state.overlay.show_palette {
            match key.code {
                KeyCode::Esc => {
                    self.state.overlay.show_palette = false;
                    self.state.overlay.palette_input.clear();
                }
                KeyCode::Enter => {
                    let cmd = self.state.overlay.palette_input.clone();
                    self.state.overlay.show_palette = false;
                    self.state.overlay.palette_input.clear();
                    self.execute_palette_input(&cmd);
                }
                KeyCode::Backspace => {
                    self.state.overlay.palette_input.pop();
                }
                KeyCode::Char(c) => {
                    self.state.overlay.palette_input.push(c);
                }
                _ => {}
            }
            return;
        }

        // Global keys
        if KeyMap::is_help(key) {
            self.state.overlay.show_help = true;
            return;
        }

        if KeyMap::is_palette(key) {
            self.state.overlay.show_palette = true;
            return;
        }

        if KeyMap::is_quit(key) {
            self.state.quit = true;
            return;
        }

        if key.code == KeyCode::Char('N') && self.engine.is_some() {
            self.open_latest_dispatch();
            return;
        }

        if matches!(
            key.code,
            KeyCode::Char('O') | KeyCode::Char('o') | KeyCode::Char('V') | KeyCode::Char('v')
        ) && self.engine.is_some()
        {
            self.state.active = Screen::EmpireOverview;
            self.state.overview.filter_input = false;
            return;
        }

        // Screen-specific handling
        match self.state.active {
            Screen::Menu => self.handle_menu_key(key),
            Screen::EmpireSelect => self.handle_empire_select_key(key),
            Screen::NewGameSetup => self.handle_new_game_setup_key(key),
            Screen::SectorOverview => self.handle_sector_overview_key(key),
            Screen::SectorMap => self.handle_sector_map_key(key),
            Screen::System => self.handle_system_key(key),
            Screen::Colony => self.handle_colony_key(key),
            Screen::EmpireOverview => self.handle_empire_overview_key(key),
            Screen::Research => self.handle_research_key(key),
            Screen::Diplomacy => self.handle_diplomacy_key(key),
        }
    }

    fn handle_menu_key(&mut self, key: KeyEvent) {
        if KeyMap::is_new_game(key) {
            // Navigate to empire selection first.
            self.state.active = Screen::EmpireSelect;
        } else if KeyMap::is_load_game(key) {
            let path = std::path::PathBuf::from(DEFAULT_SAVE_PATH);
            match self.load_game(&path) {
                Ok(()) => {
                    let msg = format!("Load: loaded {}", path.display());
                    self.state.log.push(msg.clone());
                    self.state.status_message = Some(msg);
                }
                Err(e) => {
                    self.state.log.push(e.clone());
                    self.state.status_message = Some(e);
                }
            }
        }
    }

    fn handle_empire_select_key(&mut self, key: KeyEvent) {
        let all_defs = game_core::all_empire_definitions();
        match key.code {
            KeyCode::Esc => {
                self.state.active = Screen::Menu;
            }
            KeyCode::Enter => {
                self.state.active = Screen::NewGameSetup;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.state.new_game_setup.empire_cursor = (self.state.new_game_setup.empire_cursor
                    + 1)
                .min(all_defs.len().saturating_sub(1));
            }
            KeyCode::Char('k') | KeyCode::Up if self.state.new_game_setup.empire_cursor > 0 => {
                self.state.new_game_setup.empire_cursor -= 1;
            }
            _ => {}
        }
    }

    /// Handle keyboard input on the New Game Setup screen.
    fn handle_new_game_setup_key(&mut self, key: KeyEvent) {
        use crate::screens::new_game_setup::FIELD_SEED;
        const NUM_FIELDS: usize = 3;

        // Seed editing mode intercepts most keys.
        if self.state.new_game_setup.seed_editing {
            match key.code {
                KeyCode::Esc => {
                    // Cancel editing — restore the value that was active when editing began.
                    self.state.new_game_setup.seed_str =
                        self.state.new_game_setup.seed_pre_edit.clone();
                    self.state.new_game_setup.seed_editing = false;
                }
                KeyCode::Enter => {
                    // Confirm edit.
                    self.state.new_game_setup.seed_editing = false;
                    // Ensure seed string is valid (non-empty).
                    if self.state.new_game_setup.seed_str.is_empty() {
                        self.state.new_game_setup.seed_str = "0".to_string();
                    }
                }
                KeyCode::Backspace => {
                    self.state.new_game_setup.seed_str.pop();
                }
                // Limit to 18 digits to stay within u64::MAX (20 digits).
                KeyCode::Char(c)
                    if c.is_ascii_digit() && self.state.new_game_setup.seed_str.len() < 18 =>
                {
                    self.state.new_game_setup.seed_str.push(c);
                }
                _ => {}
            }
            return;
        }

        match key.code {
            KeyCode::Esc => {
                self.state.active = Screen::EmpireSelect;
            }
            KeyCode::Enter => {
                let cursor = self.state.new_game_setup.cursor;
                if cursor == FIELD_SEED {
                    // Enter edit mode for seed field — snapshot the current value so Esc can restore it.
                    self.state.new_game_setup.seed_pre_edit =
                        self.state.new_game_setup.seed_str.clone();
                    self.state.new_game_setup.seed_editing = true;
                } else {
                    // Start the game from the setup screen.
                    self.start_game_from_setup();
                }
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.state.new_game_setup.cursor =
                    (self.state.new_game_setup.cursor + 1) % NUM_FIELDS;
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.state.new_game_setup.cursor =
                    (self.state.new_game_setup.cursor + NUM_FIELDS - 1) % NUM_FIELDS;
            }
            // Cycle left/decrease
            KeyCode::Char('h') | KeyCode::Left | KeyCode::Char('-') => {
                self.setup_cycle_field(false);
            }
            // Cycle right/increase
            KeyCode::Char('l') | KeyCode::Right | KeyCode::Char('+') => {
                self.setup_cycle_field(true);
            }
            KeyCode::Char('S') => {
                // Start the game from the setup screen (shortcut).
                self.start_game_from_setup();
            }
            _ => {}
        }
    }

    /// Cycle the currently selected setup field forward (true) or backward (false).
    fn setup_cycle_field(&mut self, forward: bool) {
        use crate::screens::new_game_setup::{FIELD_AI_COUNT, FIELD_GALAXY_SIZE};
        use game_core::GalaxySize;
        let all_sizes = GalaxySize::all();
        match self.state.new_game_setup.cursor {
            FIELD_GALAXY_SIZE => {
                let idx = all_sizes
                    .iter()
                    .position(|s| *s == self.state.new_game_setup.galaxy_size)
                    .unwrap_or(0);
                let new_idx = if forward {
                    (idx + 1).min(all_sizes.len() - 1)
                } else {
                    idx.saturating_sub(1)
                };
                self.state.new_game_setup.galaxy_size = all_sizes[new_idx];
            }
            FIELD_AI_COUNT => {
                if forward {
                    if self.state.new_game_setup.ai_count < 4 {
                        self.state.new_game_setup.ai_count += 1;
                    }
                } else if self.state.new_game_setup.ai_count > 1 {
                    self.state.new_game_setup.ai_count -= 1;
                }
            }
            _ => {}
        }
    }

    /// Build a `ScenarioSetup` from the current setup screen state and start the game.
    fn start_game_from_setup(&mut self) {
        let seed: u64 = match self.state.new_game_setup.seed_str.parse() {
            Ok(v) => v,
            Err(_) => {
                self.state.log.push(format!(
                    "Invalid seed '{}' — using 0",
                    self.state.new_game_setup.seed_str
                ));
                self.state.new_game_setup.seed_str = "0".to_string();
                0
            }
        };
        let all_defs = game_core::all_empire_definitions();
        let player_empire_def = all_defs
            .get(self.state.new_game_setup.empire_cursor)
            .map(|d| d.id);
        let setup = ScenarioSetup {
            seed,
            galaxy_size: self.state.new_game_setup.galaxy_size,
            ai_empire_count: self.state.new_game_setup.ai_count,
            sector_count_override: None,
            difficulty: game_core::DifficultyLevel::Standard,
            player_empire_def,
            victory_settings: game_core::VictorySettings::default_v1(),
        };
        self.new_game_from_setup(setup);
    }

    fn handle_sector_overview_key(&mut self, key: KeyEvent) {
        if let Some((dx, dy)) = KeyMap::movement(key) {
            self.move_sector_selection(dx, dy);
            return;
        }

        if key.code == KeyCode::Enter {
            self.state.active = Screen::SectorMap;
            return;
        }

        if key.code == KeyCode::Char('r') {
            self.state.active = Screen::Research;
            self.state.research.cursor = 0;
            return;
        }

        if key.code == KeyCode::Char('D') {
            self.state.active = Screen::Diplomacy;
            return;
        }

        if key.code == KeyCode::Char('L') {
            self.state.sector_overview.show_inter_sector_lanes =
                !self.state.sector_overview.show_inter_sector_lanes;
            return;
        }

        if KeyMap::is_end_turn(key) {
            self.end_turn();
        }
    }

    fn handle_sector_map_key(&mut self, key: KeyEvent) {
        // Rally-point picking mode: 'R' or Enter confirms, Esc cancels
        if self.state.colony.pending_rally_colony.is_some() {
            match key.code {
                KeyCode::Esc => {
                    self.state.colony.pending_rally_colony = None;
                    self.state
                        .log
                        .push("Rally point selection cancelled.".to_string());
                    return;
                }
                KeyCode::Char('R') | KeyCode::Enter => {
                    self.confirm_rally_point();
                    return;
                }
                _ => {}
            }
            // Allow normal navigation in pick mode
            if let Some((dx, dy)) = KeyMap::movement(key) {
                self.move_star_selection_in_sector(dx, dy);
            }
            return;
        }

        if key.code == KeyCode::Esc {
            self.state.active = Screen::SectorOverview;
            return;
        }

        if let Some((dx, dy)) = KeyMap::movement(key) {
            self.move_star_selection_in_sector(dx, dy);
            return;
        }

        if key.code == KeyCode::Enter {
            self.state.active = Screen::System;
            self.state.navigation.selected_planet_index = 0;
            return;
        }

        if key.code == KeyCode::Char('c') {
            if !self.try_enter_colony() {
                let msg = "Unavailable: open colony — no player colony in selected system.";
                self.state.log.push(msg.to_string());
                self.state.status_message = Some(msg.to_string());
            }
            return;
        }

        if key.code == KeyCode::Char('S') {
            self.dispatch_scout();
            return;
        }

        if key.code == KeyCode::Char('M') {
            self.move_fleet();
            return;
        }

        if key.code == KeyCode::Char('r') {
            self.state.active = Screen::Research;
            self.state.research.cursor = 0;
            return;
        }

        if key.code == KeyCode::Char('D') {
            self.state.active = Screen::Diplomacy;
            return;
        }

        if KeyMap::is_end_turn(key) {
            self.end_turn();
        }
    }

    fn handle_system_key(&mut self, key: KeyEvent) {
        let planet_count = self
            .engine
            .as_ref()
            .and_then(|engine| {
                self.state
                    .navigation
                    .selected_star
                    .and_then(|star_id| engine.state.stars.get(&star_id))
            })
            .map(|star| star.planets.len())
            .unwrap_or(0);

        match key.code {
            KeyCode::Esc => {
                self.state.active = Screen::SectorMap;
            }
            KeyCode::Enter => {
                if !self.try_enter_colony() {
                    let msg = "Unavailable: open colony — no player colony in selected system.";
                    self.state.log.push(msg.to_string());
                    self.state.status_message = Some(msg.to_string());
                }
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if planet_count > 0 {
                    self.state.navigation.selected_planet_index =
                        (self.state.navigation.selected_planet_index + 1) % planet_count;
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if planet_count > 0 {
                    self.state.navigation.selected_planet_index =
                        (self.state.navigation.selected_planet_index + planet_count - 1)
                            % planet_count;
                }
            }
            KeyCode::Char('C') => {
                self.colonize_selected_planet();
            }
            KeyCode::Char('S') => {
                self.survey_selected_planet();
            }
            KeyCode::Char('I') => {
                self.invade_selected_planet();
            }
            KeyCode::Char('c') => {
                if !self.try_enter_colony() {
                    let msg = "Unavailable: open colony — no player colony in selected system.";
                    self.state.log.push(msg.to_string());
                    self.state.status_message = Some(msg.to_string());
                }
            }
            _ => {
                if KeyMap::is_end_turn(key) && key.code != KeyCode::Enter {
                    self.end_turn();
                }
            }
        }
    }

    fn handle_colony_key(&mut self, key: KeyEvent) {
        match key.code {
            // Return to sector map
            KeyCode::Esc => {
                self.state.active = Screen::SectorMap;
                self.state.colony.selected_colony = None;
                self.state.colony.role_panel_active = false;
                self.state.colony.role_cursor = 0;
            }
            // Tab: switch active panel (build picker ↔ role selector)
            KeyCode::Tab => {
                self.state.colony.role_panel_active = !self.state.colony.role_panel_active;
            }
            // Navigate the active panel
            KeyCode::Char('j') | KeyCode::Down => {
                if self.state.colony.role_panel_active {
                    let count = ColonyRole::all().len();
                    self.state.colony.role_cursor = (self.state.colony.role_cursor + 1) % count;
                } else {
                    let count = self.visible_build_count();
                    self.state.colony.build_cursor = (self.state.colony.build_cursor + 1) % count;
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if self.state.colony.role_panel_active {
                    let count = ColonyRole::all().len();
                    self.state.colony.role_cursor =
                        (self.state.colony.role_cursor + count.saturating_sub(1)) % count;
                } else {
                    let count = self.visible_build_count();
                    self.state.colony.build_cursor =
                        (self.state.colony.build_cursor + count.saturating_sub(1)) % count;
                }
            }
            // Enter: confirm selection in the active panel
            KeyCode::Enter => {
                if self.state.colony.role_panel_active {
                    self.set_colony_role();
                } else {
                    self.queue_building();
                }
            }
            // R: start rally-point picking — return to Sector Map in pick mode
            KeyCode::Char('R') => {
                if let Some(colony_id) = self.state.colony.selected_colony {
                    self.state.colony.pending_rally_colony = Some(colony_id);
                    self.state.active = Screen::SectorMap;
                    self.state.colony.selected_colony = None;
                    self.state.colony.role_panel_active = false;
                    self.state.log.push(
                        "Select a star and press R to set rally point. Esc to cancel.".to_string(),
                    );
                }
            }
            // X: clear rally point for the active colony
            KeyCode::Char('X') => {
                self.clear_rally_point();
            }
            // End turn from colony screen
            _ => {
                if KeyMap::is_end_turn(key) && key.code != KeyCode::Enter {
                    self.end_turn();
                }
            }
        }
    }

    fn handle_empire_overview_key(&mut self, key: KeyEvent) {
        if self.state.overview.filter_input {
            match key.code {
                KeyCode::Esc => {
                    self.state.overview.filter_input = false;
                }
                KeyCode::Enter => {
                    self.state.overview.filter_input = false;
                }
                KeyCode::Backspace => {
                    self.state.overview.filter.pop();
                    self.state.overview.cursor = 0;
                }
                KeyCode::Char(c) => {
                    self.state.overview.filter.push(c);
                    self.state.overview.cursor = 0;
                }
                _ => {}
            }
            return;
        }

        let overview_data = self.engine.as_ref().map(|engine| {
            derive_empire_overview(
                &engine.state,
                engine.state.player_empire,
                self.state.overview.sort,
                &self.state.overview.filter,
            )
        });
        let visible_count = overview_data.as_ref().map(|d| d.rows.len()).unwrap_or(0);

        match key.code {
            KeyCode::Esc => {
                self.state.active = Screen::SectorMap;
                self.state.overview.filter_input = false;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if visible_count > 0 {
                    self.state.overview.cursor = (self.state.overview.cursor + 1) % visible_count;
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if visible_count > 0 {
                    self.state.overview.cursor =
                        (self.state.overview.cursor + visible_count - 1) % visible_count;
                }
            }
            KeyCode::Enter => {
                self.jump_overview_to_colony(overview_data.as_ref());
            }
            KeyCode::Char('S') => {
                self.jump_overview_to_system(overview_data.as_ref());
            }
            KeyCode::Char('s') => {
                self.state.overview.sort = self.state.overview.sort.next();
                self.state.overview.cursor = 0;
            }
            _ if KeyMap::is_search(key) => {
                self.state.overview.filter_input = true;
            }
            _ => {
                if KeyMap::is_end_turn(key) && key.code != KeyCode::Enter {
                    self.end_turn();
                }
            }
        }
    }

    fn jump_overview_to_colony(&mut self, data: Option<&EmpireOverviewData>) {
        let data = match data {
            Some(d) if !d.rows.is_empty() => d,
            _ => {
                let msg = "Unavailable: open colony — no colonies match current overview filter.";
                self.state.log.push(msg.to_string());
                self.state.status_message = Some(msg.to_string());
                return;
            }
        };
        let selected = self
            .state
            .overview
            .cursor
            .min(data.rows.len().saturating_sub(1));
        let row = &data.rows[selected];
        let (star_id, planet_index, colony_id) = (row.star_id, row.planet_index, row.colony_id);

        self.state.navigation.selected_star = Some(star_id);
        self.state.navigation.selected_planet_index = planet_index;
        self.state.colony.selected_colony = Some(colony_id);
        self.state.colony.build_cursor = 0;
        self.state.active = Screen::Colony;
    }

    fn jump_overview_to_system(&mut self, data: Option<&EmpireOverviewData>) {
        let data = match data {
            Some(d) if !d.rows.is_empty() => d,
            _ => {
                let msg = "Unavailable: open system — no colonies match current overview filter.";
                self.state.log.push(msg.to_string());
                self.state.status_message = Some(msg.to_string());
                return;
            }
        };
        let selected = self
            .state
            .overview
            .cursor
            .min(data.rows.len().saturating_sub(1));
        let row = &data.rows[selected];
        let (star_id, planet_index) = (row.star_id, row.planet_index);

        self.state.navigation.selected_star = Some(star_id);
        self.state.navigation.selected_planet_index = planet_index;
        self.state.active = Screen::System;
    }

    /// Number of build picker items visible to the player (excludes tech-locked items).
    fn visible_build_count(&self) -> usize {
        let completed = self
            .engine
            .as_ref()
            .and_then(|e| e.state.empires.get(&e.state.player_empire))
            .map(|emp| emp.research.completed.as_slice())
            .unwrap_or(&[]);
        let orbital_count = OrbitalStructureType::all()
            .iter()
            .filter(|ot| {
                ot.required_tech()
                    .map(|t| completed.contains(&t))
                    .unwrap_or(true)
            })
            .count();
        let ship_count = game_core::all_ship_designs()
            .iter()
            .filter(|d| {
                d.required_tech
                    .map(|t| completed.contains(&t))
                    .unwrap_or(true)
            })
            .count();
        BuildingType::all().len() + orbital_count + ship_count
    }

    /// Try to enter the colony screen for the selected star.
    /// Returns true if a player colony was found and the screen transitioned.
    fn try_enter_colony(&mut self) -> bool {
        let engine = match &self.engine {
            Some(e) => e,
            None => return false,
        };

        let star_id = match self.state.navigation.selected_star {
            Some(id) => id,
            None => return false,
        };

        let star = match engine.state.stars.get(&star_id) {
            Some(s) => s,
            None => return false,
        };

        if !star.planets.is_empty() {
            let selected_planet = self
                .state
                .navigation
                .selected_planet_index
                .min(star.planets.len().saturating_sub(1));
            if let Some(colony_id) = star.planets[selected_planet].colony {
                if let Some(colony) = engine.state.colonies.get(&colony_id) {
                    if colony.owner == engine.state.player_empire {
                        self.state.colony.selected_colony = Some(colony_id);
                        self.state.colony.build_cursor = 0;
                        self.state.active = Screen::Colony;
                        return true;
                    }
                }
            }
        }

        // Fallback: find the first planet at this star that has a player-owned colony.
        for planet in &star.planets {
            if let Some(colony_id) = planet.colony {
                if let Some(colony) = engine.state.colonies.get(&colony_id) {
                    if colony.owner == engine.state.player_empire {
                        self.state.colony.selected_colony = Some(colony_id);
                        self.state.colony.build_cursor = 0;
                        self.state.active = Screen::Colony;
                        return true;
                    }
                }
            }
        }

        false
    }

    fn handle_research_key(&mut self, key: KeyEvent) {
        if self.state.research.query_input {
            match key.code {
                KeyCode::Esc | KeyCode::Enter => {
                    self.state.research.query_input = false;
                }
                KeyCode::Backspace => {
                    self.state.research.query.pop();
                }
                KeyCode::Char(c) => {
                    self.state.research.query.push(c);
                }
                _ => {}
            }
            self.state.research.cursor = 0;
            return;
        }

        match key.code {
            KeyCode::Esc => {
                self.state.active = Screen::SectorMap;
            }
            KeyCode::Char('/') => {
                self.state.research.query_input = true;
            }
            KeyCode::Tab => {
                self.state.research.domain_filter =
                    (self.state.research.domain_filter + 1) % RESEARCH_DOMAIN_FILTER_COUNT;
                self.state.research.cursor = 0;
            }
            KeyCode::Char('[') => {
                self.state.research.era_filter = (self.state.research.era_filter + 1) % 7;
                self.state.research.cursor = 0;
            }
            KeyCode::Char(']') => {
                self.state.research.status_filter =
                    (self.state.research.status_filter + 1) % RESEARCH_STATUS_FILTER_COUNT;
                self.state.research.cursor = 0;
            }
            // Navigate tech list
            KeyCode::Char('j') | KeyCode::Down => {
                let count = self.tech_tree_count();
                if count > 0 {
                    self.state.research.cursor = (self.state.research.cursor + 1) % count;
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                let count = self.tech_tree_count();
                if count > 0 {
                    self.state.research.cursor =
                        (self.state.research.cursor + count.saturating_sub(1)) % count;
                }
            }
            // Select the highlighted tech for research
            KeyCode::Enter => {
                self.select_research_tech();
            }
            // Queue highlighted tech for automatic follow-up
            KeyCode::Char('a') => {
                self.queue_research_tech();
            }
            // Remove highlighted tech from research queue
            KeyCode::Char('x') => {
                self.remove_queued_research_tech();
            }
            // Move highlighted queued tech one slot earlier
            KeyCode::Char('u') => {
                self.move_queued_research_up();
            }
            // Move highlighted queued tech one slot later
            KeyCode::Char('d') => {
                self.move_queued_research_down();
            }
            // Clear entire research queue
            KeyCode::Char('c') => {
                self.dispatch_command(Command::ClearResearchQueue);
            }
            // End turn from research screen (excluding Enter, which selects tech)
            _ => {
                if KeyMap::is_end_turn(key) && key.code != KeyCode::Enter {
                    self.end_turn();
                }
            }
        }
    }

    fn handle_diplomacy_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.state.active = Screen::SectorMap;
            }
            // End turn from diplomacy screen
            _ => {
                if KeyMap::is_end_turn(key) {
                    self.end_turn();
                }
            }
        }
    }

    /// Returns the number of technologies in the tree.
    fn tech_tree_count(&self) -> usize {
        let Some(engine) = &self.engine else {
            return 0;
        };
        filtered_research_techs(&self.state, &engine.state).len()
    }

    /// Dispatch one game-core command and centralize event logging/status updates.
    fn dispatch_command(&mut self, command: Command) {
        let is_end_turn = matches!(command, Command::EndTurn);
        let (events, end_turn_report) = {
            let engine = match &mut self.engine {
                Some(engine) => engine,
                None => return,
            };
            let events = engine.apply_turn(vec![command]);
            let report =
                is_end_turn.then(|| Self::build_end_turn_report(engine.state.turn, &events));
            (events, report)
        };

        for event in &events {
            self.push_core_event_to_log(event);
        }

        if let Some(report) = end_turn_report {
            self.push_status(LogEntryKind::TurnReport, report);
            // Auto-show dispatch if a new one was generated this turn
            if let Some(engine) = &self.engine {
                if !engine.state.galactic_dispatches.is_empty() {
                    let last_idx = engine.state.galactic_dispatches.len().saturating_sub(1);
                    let last_dispatch = &engine.state.galactic_dispatches[last_idx];
                    // last_dispatch.turn is the completed_turn; state.turn has been incremented
                    if last_dispatch.turn + 1 == engine.state.turn {
                        self.state.overlay.dispatch_history_index = last_idx;
                        self.state.overlay.show_dispatch = true;
                    }
                }
            }
            return;
        }

        if let Some(CoreEvent::Error { message }) = events
            .iter()
            .rev()
            .find(|event| matches!(event, CoreEvent::Error { .. }))
        {
            self.state.status_message = Some(format!("Error: {}", message));
        }
    }

    /// Select the highlighted technology for research
    fn select_research_tech(&mut self) {
        let Some(tech_id) = self.highlighted_research_tech("select research") else {
            return;
        };

        self.dispatch_command(Command::SelectResearch { tech: tech_id });
    }

    fn queue_research_tech(&mut self) {
        let Some(tech_id) = self.highlighted_research_tech("queue research") else {
            return;
        };

        self.dispatch_command(Command::QueueResearch { tech: tech_id });
    }

    fn remove_queued_research_tech(&mut self) {
        let Some(tech_id) = self.highlighted_research_tech("remove queued research") else {
            return;
        };

        self.dispatch_command(Command::RemoveQueuedResearch { tech: tech_id });
    }

    fn move_queued_research_up(&mut self) {
        let Some(tech_id) = self.highlighted_research_tech("reorder queued research") else {
            return;
        };

        self.dispatch_command(Command::MoveQueuedResearchUp { tech: tech_id });
    }

    fn move_queued_research_down(&mut self) {
        let Some(tech_id) = self.highlighted_research_tech("reorder queued research") else {
            return;
        };

        self.dispatch_command(Command::MoveQueuedResearchDown { tech: tech_id });
    }

    fn highlighted_research_tech(&mut self, action: &str) -> Option<TechId> {
        let Some(engine) = &self.engine else {
            let msg = format!("Unavailable: {} — no game in progress.", action);
            self.state.log.push(msg.clone());
            self.state.status_message = Some(msg);
            return None;
        };

        let visible = filtered_research_techs(&self.state, &engine.state);
        if visible.is_empty() {
            let msg = format!(
                "Unavailable: {} — no technologies match current filters.",
                action
            );
            self.state.log.push(msg.clone());
            self.state.status_message = Some(msg);
            return None;
        }

        let cursor = self.state.research.cursor % visible.len();
        Some(visible[cursor].id)
    }

    /// Queue the currently selected build item at the active colony
    fn queue_building(&mut self) {
        let colony_id = match self.state.colony.selected_colony {
            Some(id) => id,
            None => {
                let msg = "Unavailable: queue build — no colony selected.";
                self.state.log.push(msg.to_string());
                self.state.status_message = Some(msg.to_string());
                return;
            }
        };

        let completed = self
            .engine
            .as_ref()
            .and_then(|e| e.state.empires.get(&e.state.player_empire))
            .map(|emp| emp.research.completed.clone())
            .unwrap_or_default();
        let surface_buildings = BuildingType::all();
        let visible_orbitals: Vec<_> = OrbitalStructureType::all()
            .iter()
            .filter(|ot| {
                ot.required_tech()
                    .map(|t| completed.contains(&t))
                    .unwrap_or(true)
            })
            .collect();
        let visible_ships: Vec<_> = game_core::all_ship_designs()
            .iter()
            .filter(|d| {
                d.required_tech
                    .map(|t| completed.contains(&t))
                    .unwrap_or(true)
            })
            .collect();

        let total = surface_buildings.len() + visible_orbitals.len() + visible_ships.len();
        if total == 0 {
            let msg = "Unavailable: queue build — no build items available.";
            self.state.log.push(msg.to_string());
            self.state.status_message = Some(msg.to_string());
            return;
        }
        let cursor = self.state.colony.build_cursor % total;

        let item = if cursor < surface_buildings.len() {
            game_core::BuildItem::SurfaceStructure(surface_buildings[cursor])
        } else if cursor < surface_buildings.len() + visible_orbitals.len() {
            game_core::BuildItem::OrbitalStructure(
                *visible_orbitals[cursor - surface_buildings.len()],
            )
        } else {
            game_core::BuildItem::Ship(
                visible_ships[cursor - surface_buildings.len() - visible_orbitals.len()].id,
            )
        };

        self.dispatch_command(Command::QueueBuild {
            colony: colony_id,
            item,
        });
    }

    /// Assign the currently highlighted role to the active colony.
    fn set_colony_role(&mut self) {
        let colony_id = match self.state.colony.selected_colony {
            Some(id) => id,
            None => {
                let msg = "Unavailable: set role — no colony selected.";
                self.state.log.push(msg.to_string());
                self.state.status_message = Some(msg.to_string());
                return;
            }
        };

        let roles = ColonyRole::all();
        if roles.is_empty() {
            let msg = "Unavailable: set role — no colony roles are defined.";
            self.state.log.push(msg.to_string());
            self.state.status_message = Some(msg.to_string());
            return;
        }
        let cursor = self.state.colony.role_cursor % roles.len();
        let role = roles[cursor];

        self.dispatch_command(Command::SetColonyRole {
            colony: colony_id,
            role,
        });
    }

    /// Confirm the selected star as the rally point for `pending_rally_colony`.
    fn confirm_rally_point(&mut self) {
        let colony_id = match self.state.colony.pending_rally_colony.take() {
            Some(id) => id,
            None => return,
        };
        let star_id = match self.state.navigation.selected_star {
            Some(id) => id,
            None => {
                self.state
                    .log
                    .push("No star selected for rally point.".to_string());
                return;
            }
        };

        self.dispatch_command(Command::SetRallyPoint {
            colony: colony_id,
            star: star_id,
        });
    }

    /// Clear the rally point for the currently selected colony (or the pending rally colony).
    fn clear_rally_point(&mut self) {
        let colony_id = self
            .state
            .colony
            .selected_colony
            .or(self.state.colony.pending_rally_colony);
        let colony_id = match colony_id {
            Some(id) => id,
            None => {
                self.state
                    .log
                    .push("No colony selected for rally clear.".to_string());
                return;
            }
        };
        self.state.colony.pending_rally_colony = None;

        self.dispatch_command(Command::ClearRallyPoint { colony: colony_id });
    }

    #[allow(dead_code)]
    fn move_star_selection(&mut self, dx: i32, dy: i32) {
        let engine = match &self.engine {
            Some(e) => e,
            None => return,
        };

        let current = match self.state.navigation.selected_star {
            Some(id) => id,
            None => {
                // Select first star if none selected
                self.state.navigation.selected_star = engine.state.stars.keys().next().copied();
                return;
            }
        };

        let current_star = match engine.state.stars.get(&current) {
            Some(s) => s,
            None => return,
        };

        // Find the nearest star in the given direction
        let mut best: Option<(StarId, i32)> = None;

        for star in engine.state.stars.values() {
            if star.id == current {
                continue;
            }

            let rel_x = star.x - current_star.x;
            let rel_y = star.y - current_star.y;

            // Check if star is in the right direction
            let in_direction = match (dx, dy) {
                (1, 0) => rel_x > 0,
                (-1, 0) => rel_x < 0,
                (0, 1) => rel_y > 0,
                (0, -1) => rel_y < 0,
                _ => false,
            };

            if !in_direction {
                continue;
            }

            let distance = rel_x * rel_x + rel_y * rel_y;

            match &best {
                None => best = Some((star.id, distance)),
                Some((_, best_dist)) if distance < *best_dist => {
                    best = Some((star.id, distance));
                }
                _ => {}
            }
        }

        if let Some((id, _)) = best {
            self.state.navigation.selected_star = Some(id);
        }
    }

    fn move_sector_selection(&mut self, dx: i32, dy: i32) {
        let engine = match &self.engine {
            Some(e) => e,
            None => return,
        };

        let current = match self.state.navigation.selected_sector {
            Some(id) => id,
            None => {
                self.state.navigation.selected_sector = engine.state.sectors.keys().next().copied();
                return;
            }
        };

        let current_sector = match engine.state.sectors.get(&current) {
            Some(s) => s,
            None => return,
        };

        let mut best: Option<(SectorId, i32)> = None;

        for sector in engine.state.sectors.values() {
            if sector.id == current {
                continue;
            }

            let rel_x = sector.x - current_sector.x;
            let rel_y = sector.y - current_sector.y;

            let in_direction = match (dx, dy) {
                (1, 0) => rel_x > 0,
                (-1, 0) => rel_x < 0,
                (0, 1) => rel_y > 0,
                (0, -1) => rel_y < 0,
                _ => false,
            };

            if !in_direction {
                continue;
            }

            let distance = rel_x * rel_x + rel_y * rel_y;

            match &best {
                None => best = Some((sector.id, distance)),
                Some((_, best_dist)) if distance < *best_dist => {
                    best = Some((sector.id, distance));
                }
                _ => {}
            }
        }

        if let Some((id, _)) = best {
            self.state.navigation.selected_sector = Some(id);
            self.state.navigation.selected_star = engine
                .state
                .stars
                .values()
                .find(|s| s.sector == id)
                .map(|s| s.id);
        }
    }

    fn move_star_selection_in_sector(&mut self, dx: i32, dy: i32) {
        let engine = match &self.engine {
            Some(e) => e,
            None => return,
        };

        let sector_id = match self.state.navigation.selected_sector {
            Some(id) => id,
            None => return,
        };

        let current = match self.state.navigation.selected_star {
            Some(id) => id,
            None => {
                self.state.navigation.selected_star = engine
                    .state
                    .stars
                    .values()
                    .find(|s| s.sector == sector_id)
                    .map(|s| s.id);
                return;
            }
        };

        let current_star = match engine.state.stars.get(&current) {
            Some(s) if s.sector == sector_id => s,
            _ => {
                self.state.navigation.selected_star = engine
                    .state
                    .stars
                    .values()
                    .find(|s| s.sector == sector_id)
                    .map(|s| s.id);
                return;
            }
        };

        let mut best: Option<(StarId, i32)> = None;

        for star in engine.state.stars.values() {
            if star.id == current || star.sector != sector_id {
                continue;
            }

            let rel_x = star.x - current_star.x;
            let rel_y = star.y - current_star.y;

            let in_direction = match (dx, dy) {
                (1, 0) => rel_x > 0,
                (-1, 0) => rel_x < 0,
                (0, 1) => rel_y > 0,
                (0, -1) => rel_y < 0,
                _ => false,
            };

            if !in_direction {
                continue;
            }

            let distance = rel_x * rel_x + rel_y * rel_y;

            match &best {
                None => best = Some((star.id, distance)),
                Some((_, best_dist)) if distance < *best_dist => {
                    best = Some((star.id, distance));
                }
                _ => {}
            }
        }

        if let Some((id, _)) = best {
            self.state.navigation.selected_star = Some(id);
        }
    }

    fn end_turn(&mut self) {
        self.dispatch_command(Command::EndTurn);
    }

    /// Open the Galactic Dispatch modal showing the latest dispatch.
    fn open_latest_dispatch(&mut self) {
        if let Some(engine) = &self.engine {
            if !engine.state.galactic_dispatches.is_empty() {
                self.state.overlay.dispatch_history_index =
                    engine.state.galactic_dispatches.len().saturating_sub(1);
                self.state.overlay.show_dispatch = true;
            } else {
                let msg = "No dispatches available yet.";
                self.state.status_message = Some(msg.to_string());
            }
        }
    }

    /// Dispatch an available scout fleet to the currently selected star system.
    /// Logs an error if no fleet is available or the destination is already explored.
    fn dispatch_scout(&mut self) {
        let star_id = match self.state.navigation.selected_star {
            Some(id) => id,
            None => {
                let msg = "Unavailable: dispatch scout — no star selected.";
                self.state.log.push(msg.to_string());
                self.state.status_message = Some(msg.to_string());
                return;
            }
        };

        let fleet_id: Option<FleetId> = {
            let engine = match &self.engine {
                Some(e) => e,
                None => return,
            };
            first_idle_player_fleet(&engine.state, Some(FleetKind::Scout), None)
        };

        let fleet_id = match fleet_id {
            Some(id) => id,
            None => {
                let msg = "Unavailable: dispatch scout — no idle scout is available.";
                self.state.log.push(msg.to_string());
                self.state.status_message = Some(msg.to_string());
                return;
            }
        };

        self.dispatch_command(Command::SendScout {
            fleet: fleet_id,
            destination: star_id,
        });
    }

    /// Move the first idle player fleet to the currently selected (explored) star system.
    /// Logs an error if no idle fleet is available or the destination is not explored.
    fn move_fleet(&mut self) {
        let star_id = match self.state.navigation.selected_star {
            Some(id) => id,
            None => {
                let msg = "Unavailable: move fleet — no star selected.";
                self.state.log.push(msg.to_string());
                self.state.status_message = Some(msg.to_string());
                return;
            }
        };

        let fleet_id: Option<FleetId> = {
            let engine = match &self.engine {
                Some(e) => e,
                None => return,
            };
            first_idle_player_fleet(&engine.state, None, None)
        };

        let fleet_id = match fleet_id {
            Some(id) => id,
            None => {
                let msg = "Unavailable: move fleet — no idle fleet is available.";
                self.state.log.push(msg.to_string());
                self.state.status_message = Some(msg.to_string());
                return;
            }
        };

        self.dispatch_command(Command::MoveFleet {
            fleet: fleet_id,
            destination: star_id,
        });
    }

    /// Colonize the currently selected planet at the selected star system
    /// using an idle colonizer fleet present at that system.
    fn colonize_selected_planet(&mut self) {
        let star_id = match self.state.navigation.selected_star {
            Some(id) => id,
            None => {
                let msg = "Unavailable: colonize — no star selected.";
                self.state.log.push(msg.to_string());
                self.state.status_message = Some(msg.to_string());
                return;
            }
        };

        let fleet_id: Option<FleetId> = {
            let engine = match &self.engine {
                Some(e) => e,
                None => return,
            };
            first_idle_player_fleet(&engine.state, Some(FleetKind::Colonizer), Some(star_id))
        };

        let fleet_id = match fleet_id {
            Some(id) => id,
            None => {
                let msg = "Unavailable: colonize — no idle colonizer at selected system.";
                self.state.log.push(msg.to_string());
                self.state.status_message = Some(msg.to_string());
                return;
            }
        };

        let planet_count = {
            let engine = match &self.engine {
                Some(e) => e,
                None => return,
            };
            engine
                .state
                .stars
                .get(&star_id)
                .map(|s| s.planets.len())
                .unwrap_or(0)
        };

        if planet_count == 0 {
            let msg = "Unavailable: colonize — selected system has no planets.";
            self.state.log.push(msg.to_string());
            self.state.status_message = Some(msg.to_string());
            return;
        }
        let planet_index = self
            .state
            .navigation
            .selected_planet_index
            .min(planet_count - 1);

        self.dispatch_command(Command::Colonize {
            fleet: fleet_id,
            star: star_id,
            planet_index,
        });
    }

    /// Survey the currently selected planet using an idle science ship at the selected star.
    fn survey_selected_planet(&mut self) {
        let star_id = match self.state.navigation.selected_star {
            Some(id) => id,
            None => {
                let msg = "Unavailable: survey — no star selected.";
                self.state.log.push(msg.to_string());
                self.state.status_message = Some(msg.to_string());
                return;
            }
        };

        let fleet_id: Option<FleetId> = {
            let engine = match &self.engine {
                Some(e) => e,
                None => return,
            };
            first_idle_player_fleet(&engine.state, Some(FleetKind::Science), Some(star_id))
        };

        let fleet_id = match fleet_id {
            Some(id) => id,
            None => {
                let msg = "Unavailable: survey — no idle science ship at selected system.";
                self.state.log.push(msg.to_string());
                self.state.status_message = Some(msg.to_string());
                return;
            }
        };

        let planet_count = {
            let engine = match &self.engine {
                Some(e) => e,
                None => return,
            };
            engine
                .state
                .stars
                .get(&star_id)
                .map(|s| s.planets.len())
                .unwrap_or(0)
        };

        if planet_count == 0 {
            let msg = "Unavailable: survey — selected system has no planets.";
            self.state.log.push(msg.to_string());
            self.state.status_message = Some(msg.to_string());
            return;
        }

        let planet_index = self
            .state
            .navigation
            .selected_planet_index
            .min(planet_count - 1);

        self.dispatch_command(Command::SurveyPlanet {
            fleet: fleet_id,
            star: star_id,
            planet_index,
        });
    }

    /// Invade the currently selected enemy colony using an idle troop transport at this system.
    fn invade_selected_planet(&mut self) {
        let star_id = match self.state.navigation.selected_star {
            Some(id) => id,
            None => {
                let msg = "Unavailable: invade — no star selected.";
                self.state.log.push(msg.to_string());
                self.state.status_message = Some(msg.to_string());
                return;
            }
        };

        let fleet_id: Option<FleetId> = {
            let engine = match &self.engine {
                Some(e) => e,
                None => return,
            };
            first_idle_player_fleet(
                &engine.state,
                Some(FleetKind::TroopTransport),
                Some(star_id),
            )
        };

        let fleet_id = match fleet_id {
            Some(id) => id,
            None => {
                let msg = "Unavailable: invade — no idle troop transport at selected system.";
                self.state.log.push(msg.to_string());
                self.state.status_message = Some(msg.to_string());
                return;
            }
        };

        let planet_count = {
            let engine = match &self.engine {
                Some(e) => e,
                None => return,
            };
            engine
                .state
                .stars
                .get(&star_id)
                .map(|s| s.planets.len())
                .unwrap_or(0)
        };
        if planet_count == 0 {
            let msg = "Unavailable: invade — selected system has no planets.";
            self.state.log.push(msg.to_string());
            self.state.status_message = Some(msg.to_string());
            return;
        }

        let planet_index = self
            .state
            .navigation
            .selected_planet_index
            .min(planet_count - 1);
        self.dispatch_command(Command::Invade {
            fleet: fleet_id,
            star: star_id,
            planet_index,
        });
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests;
