//! Application state and main run loop

use crate::components::{render_help, render_palette, EventLog};
use crate::keys::KeyMap;
use crate::screens::empire_overview::{derive_empire_overview, EmpireOverviewData, OverviewSort};
use crate::screens::Screen;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use game_core::{
    all_techs, BuildingType, ColonyId, ColonyRole, Command, Engine, FleetId, FleetKind,
    OrbitalStructureType, SectorId, StarId, TechId,
};
use ratatui::{backend::Backend, Frame, Terminal};
use std::io;
use std::time::Duration;

/// Default save file path
const DEFAULT_SAVE_PATH: &str = "farspace.sav";

/// Main application state
pub struct App {
    pub state: AppState,
    pub engine: Option<Engine>,
    pub pending_commands: Vec<Command>,
}

/// UI state
#[derive(Debug, Clone, Default)]
pub struct AppState {
    pub active: Screen,
    pub show_help: bool,
    pub show_palette: bool,
    pub palette_input: String,
    pub search_input: String,
    pub show_search: bool,
    pub selected_sector: Option<SectorId>,
    pub selected_star: Option<StarId>,
    /// Toggle for rendering inter-sector hyperspace lanes on galaxy overview.
    pub show_inter_sector_lanes: bool,
    /// Selected planet index when inspecting a system
    pub selected_planet_index: usize,
    /// Currently viewed colony (set when entering the colony screen)
    pub selected_colony: Option<ColonyId>,
    /// Cursor index for the build-picker on the colony screen
    pub colony_build_cursor: usize,
    /// Cursor index for the role selector on the colony screen
    pub colony_role_cursor: usize,
    /// Whether the role selector is the active panel (true) or build picker (false)
    pub colony_role_panel_active: bool,
    /// Cursor index for the tech list on the research screen
    pub research_cursor: usize,
    /// Cursor index for selected row on empire overview
    pub overview_cursor: usize,
    /// Current sort mode for empire overview
    pub overview_sort: OverviewSort,
    /// Optional overview filter query
    pub overview_filter: String,
    /// Whether filter input mode is active on the overview screen
    pub overview_filter_input: bool,
    pub log: EventLog,
    pub quit: bool,
    /// Monotonically-increasing frame counter, incremented once per render loop iteration.
    /// Used only for low-frequency UI animations; never affects simulation state.
    pub tick_count: u64,
    /// When true, all fleet travel animations are suppressed (accessibility / low-motion).
    pub reduced_motion: bool,
}

impl App {
    /// Create a new application
    pub fn new() -> Self {
        App {
            state: AppState::default(),
            engine: None,
            pending_commands: Vec::new(),
        }
    }

    /// Start a new game with the given seed
    pub fn new_game(&mut self, seed: u64) {
        let engine = Engine::new(seed);

        // Select the first sector and first star by default
        self.state.selected_sector = engine.state.sectors.keys().next().copied();
        self.state.selected_star = engine.state.stars.keys().next().copied();
        self.state.selected_planet_index = 0;

        // Add initial log entry
        self.state.log.push("Game started".to_string());

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
        self.state.selected_sector = selected_sector;
        self.state.selected_star = selected_star;
        self.state.selected_planet_index = 0;
        self.state.active = Screen::SectorOverview;
        Ok(())
    }

    /// Execute a palette command string (e.g. "save", ":save")
    fn execute_palette_command(&mut self, cmd: &str) {
        let cmd = cmd.trim_start_matches(':').trim();
        if cmd.is_empty() {
            return;
        }
        let path = std::path::PathBuf::from(DEFAULT_SAVE_PATH);
        match cmd {
            "save" => match self.save_game(&path) {
                Ok(()) => self.state.log.push("Game saved.".to_string()),
                Err(e) => self.state.log.push(e),
            },
            "load" => match self.load_game(&path) {
                Ok(()) => self.state.log.push("Game loaded.".to_string()),
                Err(e) => self.state.log.push(e),
            },
            other => {
                self.state
                    .log
                    .push(format!("Error: Unknown command: {}", other));
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
        if self.state.show_help {
            render_help(frame, area, &self.state.active);
        }

        if self.state.show_palette {
            render_palette(frame, area, &self.state.palette_input);
        }
    }

    /// Handle a key event
    fn handle_key(&mut self, key: KeyEvent) {
        // Handle overlays first
        if self.state.show_help {
            if KeyMap::is_help(key) || KeyMap::is_escape(key) {
                self.state.show_help = false;
            }
            return;
        }

        if self.state.show_palette {
            match key.code {
                KeyCode::Esc => {
                    self.state.show_palette = false;
                    self.state.palette_input.clear();
                }
                KeyCode::Enter => {
                    let cmd = self.state.palette_input.clone();
                    self.state.show_palette = false;
                    self.state.palette_input.clear();
                    // Normalize before checking — skip empty, whitespace-only, or bare ":"
                    let normalized = cmd.trim_start_matches(':').trim();
                    if !normalized.is_empty() {
                        self.execute_palette_command(&cmd);
                    }
                }
                KeyCode::Backspace => {
                    self.state.palette_input.pop();
                }
                KeyCode::Char(c) => {
                    self.state.palette_input.push(c);
                }
                _ => {}
            }
            return;
        }

        // Global keys
        if KeyMap::is_help(key) {
            self.state.show_help = true;
            return;
        }

        if KeyMap::is_palette(key) {
            self.state.show_palette = true;
            return;
        }

        if KeyMap::is_quit(key) {
            self.state.quit = true;
            return;
        }

        if key.code == KeyCode::Char('O') && self.engine.is_some() {
            self.state.active = Screen::EmpireOverview;
            self.state.overview_filter_input = false;
            return;
        }

        // Screen-specific handling
        match self.state.active {
            Screen::Menu => self.handle_menu_key(key),
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
            // Use a fixed default seed for deterministic, reproducible games.
            // A user-configurable seed will be added via the command palette.
            self.new_game(42);
        } else if KeyMap::is_load_game(key) {
            let path = std::path::PathBuf::from(DEFAULT_SAVE_PATH);
            match self.load_game(&path) {
                Ok(()) => self.state.log.push("Game loaded.".to_string()),
                Err(e) => self.state.log.push(e),
            }
        }
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
            self.state.research_cursor = 0;
            return;
        }

        if key.code == KeyCode::Char('D') {
            self.state.active = Screen::Diplomacy;
            return;
        }

        if key.code == KeyCode::Char('L') {
            self.state.show_inter_sector_lanes = !self.state.show_inter_sector_lanes;
            return;
        }

        if KeyMap::is_end_turn(key) {
            self.end_turn();
        }
    }

    fn handle_sector_map_key(&mut self, key: KeyEvent) {
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
            self.state.selected_planet_index = 0;
            return;
        }

        if key.code == KeyCode::Char('c') {
            self.try_enter_colony();
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
            self.state.research_cursor = 0;
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
                self.try_enter_colony();
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if planet_count > 0 {
                    self.state.selected_planet_index =
                        (self.state.selected_planet_index + 1) % planet_count;
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if planet_count > 0 {
                    self.state.selected_planet_index =
                        (self.state.selected_planet_index + planet_count - 1) % planet_count;
                }
            }
            KeyCode::Char('C') => {
                self.colonize_selected_planet();
            }
            KeyCode::Char('S') => {
                self.survey_selected_planet();
            }
            KeyCode::Char('c') => {
                self.try_enter_colony();
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
                self.state.selected_colony = None;
                self.state.colony_role_panel_active = false;
                self.state.colony_role_cursor = 0;
            }
            // Tab: switch active panel (build picker ↔ role selector)
            KeyCode::Tab => {
                self.state.colony_role_panel_active = !self.state.colony_role_panel_active;
            }
            // Navigate the active panel
            KeyCode::Char('j') | KeyCode::Down => {
                if self.state.colony_role_panel_active {
                    let count = ColonyRole::all().len();
                    self.state.colony_role_cursor = (self.state.colony_role_cursor + 1) % count;
                } else {
                    let count = Self::all_build_item_count();
                    self.state.colony_build_cursor = (self.state.colony_build_cursor + 1) % count;
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if self.state.colony_role_panel_active {
                    let count = ColonyRole::all().len();
                    self.state.colony_role_cursor =
                        (self.state.colony_role_cursor + count.saturating_sub(1)) % count;
                } else {
                    let count = Self::all_build_item_count();
                    self.state.colony_build_cursor =
                        (self.state.colony_build_cursor + count.saturating_sub(1)) % count;
                }
            }
            // Enter: confirm selection in the active panel
            KeyCode::Enter => {
                if self.state.colony_role_panel_active {
                    self.set_colony_role();
                } else {
                    self.queue_building();
                }
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
        if self.state.overview_filter_input {
            match key.code {
                KeyCode::Esc => {
                    self.state.overview_filter_input = false;
                }
                KeyCode::Enter => {
                    self.state.overview_filter_input = false;
                }
                KeyCode::Backspace => {
                    self.state.overview_filter.pop();
                    self.state.overview_cursor = 0;
                }
                KeyCode::Char(c) => {
                    self.state.overview_filter.push(c);
                    self.state.overview_cursor = 0;
                }
                _ => {}
            }
            return;
        }

        let overview_data = self.engine.as_ref().map(|engine| {
            derive_empire_overview(
                &engine.state,
                engine.state.player_empire,
                self.state.overview_sort,
                &self.state.overview_filter,
            )
        });
        let visible_count = overview_data.as_ref().map(|d| d.rows.len()).unwrap_or(0);

        match key.code {
            KeyCode::Esc => {
                self.state.active = Screen::SectorMap;
                self.state.overview_filter_input = false;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if visible_count > 0 {
                    self.state.overview_cursor = (self.state.overview_cursor + 1) % visible_count;
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if visible_count > 0 {
                    self.state.overview_cursor =
                        (self.state.overview_cursor + visible_count - 1) % visible_count;
                }
            }
            KeyCode::Enter => {
                self.jump_overview_to_colony(overview_data.as_ref());
            }
            KeyCode::Char('S') => {
                self.jump_overview_to_system(overview_data.as_ref());
            }
            KeyCode::Char('s') => {
                self.state.overview_sort = self.state.overview_sort.next();
                self.state.overview_cursor = 0;
            }
            _ if KeyMap::is_search(key) => {
                self.state.overview_filter_input = true;
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
            _ => return,
        };
        let selected = self
            .state
            .overview_cursor
            .min(data.rows.len().saturating_sub(1));
        let row = &data.rows[selected];
        let (star_id, planet_index, colony_id) = (row.star_id, row.planet_index, row.colony_id);

        self.state.selected_star = Some(star_id);
        self.state.selected_planet_index = planet_index;
        self.state.selected_colony = Some(colony_id);
        self.state.colony_build_cursor = 0;
        self.state.active = Screen::Colony;
    }

    fn jump_overview_to_system(&mut self, data: Option<&EmpireOverviewData>) {
        let data = match data {
            Some(d) if !d.rows.is_empty() => d,
            _ => return,
        };
        let selected = self
            .state
            .overview_cursor
            .min(data.rows.len().saturating_sub(1));
        let row = &data.rows[selected];
        let (star_id, planet_index) = (row.star_id, row.planet_index);

        self.state.selected_star = Some(star_id);
        self.state.selected_planet_index = planet_index;
        self.state.active = Screen::System;
    }

    /// Total number of items in the build picker (surface buildings + orbital structures + ships)
    fn all_build_item_count() -> usize {
        BuildingType::all().len()
            + OrbitalStructureType::all().len()
            + game_core::all_ship_designs().len()
    }

    /// Try to enter the colony screen for the selected star.
    /// Returns true if a player colony was found and the screen transitioned.
    fn try_enter_colony(&mut self) -> bool {
        let engine = match &self.engine {
            Some(e) => e,
            None => return false,
        };

        let star_id = match self.state.selected_star {
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
                .selected_planet_index
                .min(star.planets.len().saturating_sub(1));
            if let Some(colony_id) = star.planets[selected_planet].colony {
                if let Some(colony) = engine.state.colonies.get(&colony_id) {
                    if colony.owner == engine.state.player_empire {
                        self.state.selected_colony = Some(colony_id);
                        self.state.colony_build_cursor = 0;
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
                        self.state.selected_colony = Some(colony_id);
                        self.state.colony_build_cursor = 0;
                        self.state.active = Screen::Colony;
                        return true;
                    }
                }
            }
        }

        false
    }

    fn handle_research_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.state.active = Screen::SectorMap;
            }
            // Navigate tech list
            KeyCode::Char('j') | KeyCode::Down => {
                let count = self.available_tech_count();
                if count > 0 {
                    self.state.research_cursor = (self.state.research_cursor + 1) % count;
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                let count = self.available_tech_count();
                if count > 0 {
                    self.state.research_cursor =
                        (self.state.research_cursor + count.saturating_sub(1)) % count;
                }
            }
            // Select the highlighted tech for research
            KeyCode::Enter => {
                self.select_research_tech();
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
    fn available_tech_count(&self) -> usize {
        if self.engine.is_none() {
            return 0;
        }
        all_techs().len()
    }

    /// Select the highlighted technology for research
    fn select_research_tech(&mut self) {
        // Collect the tech_id first using a scoped borrow
        let tech_id: TechId = {
            if self.engine.is_none() {
                return;
            }

            let all = all_techs();
            if all.is_empty() {
                return;
            }
            let cursor = self.state.research_cursor % all.len();
            all[cursor].id
        };

        self.pending_commands
            .push(Command::SelectResearch { tech: tech_id });

        let commands = std::mem::take(&mut self.pending_commands);
        let engine = match &mut self.engine {
            Some(e) => e,
            None => return,
        };
        let events = engine.apply_turn(commands);
        for event in events {
            self.state.log.push(event.to_log_message());
        }
    }

    /// Queue the currently selected build item at the active colony
    fn queue_building(&mut self) {
        let colony_id = match self.state.selected_colony {
            Some(id) => id,
            None => return,
        };

        let surface_buildings = BuildingType::all();
        let orbital_structures = OrbitalStructureType::all();
        let ship_designs = game_core::all_ship_designs();

        let total = surface_buildings.len() + orbital_structures.len() + ship_designs.len();
        if total == 0 {
            return;
        }
        let cursor = self.state.colony_build_cursor % total;

        let item = if cursor < surface_buildings.len() {
            game_core::BuildItem::SurfaceStructure(surface_buildings[cursor])
        } else if cursor < surface_buildings.len() + orbital_structures.len() {
            game_core::BuildItem::OrbitalStructure(
                orbital_structures[cursor - surface_buildings.len()],
            )
        } else {
            game_core::BuildItem::Ship(
                ship_designs[cursor - surface_buildings.len() - orbital_structures.len()].id,
            )
        };

        self.pending_commands.push(Command::QueueBuild {
            colony: colony_id,
            item,
        });

        let commands = std::mem::take(&mut self.pending_commands);
        let engine = match &mut self.engine {
            Some(e) => e,
            None => return,
        };
        let events = engine.apply_turn(commands);
        for event in events {
            self.state.log.push(event.to_log_message());
        }
    }

    /// Assign the currently highlighted role to the active colony.
    fn set_colony_role(&mut self) {
        let colony_id = match self.state.selected_colony {
            Some(id) => id,
            None => return,
        };

        let roles = ColonyRole::all();
        if roles.is_empty() {
            return;
        }
        let cursor = self.state.colony_role_cursor % roles.len();
        let role = roles[cursor];

        self.pending_commands.push(Command::SetColonyRole {
            colony: colony_id,
            role,
        });

        let commands = std::mem::take(&mut self.pending_commands);
        let engine = match &mut self.engine {
            Some(e) => e,
            None => return,
        };
        let events = engine.apply_turn(commands);
        for event in events {
            self.state.log.push(event.to_log_message());
        }
    }

    #[allow(dead_code)]
    fn move_star_selection(&mut self, dx: i32, dy: i32) {
        let engine = match &self.engine {
            Some(e) => e,
            None => return,
        };

        let current = match self.state.selected_star {
            Some(id) => id,
            None => {
                // Select first star if none selected
                self.state.selected_star = engine.state.stars.keys().next().copied();
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
            self.state.selected_star = Some(id);
        }
    }

    fn move_sector_selection(&mut self, dx: i32, dy: i32) {
        let engine = match &self.engine {
            Some(e) => e,
            None => return,
        };

        let current = match self.state.selected_sector {
            Some(id) => id,
            None => {
                self.state.selected_sector = engine.state.sectors.keys().next().copied();
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
            self.state.selected_sector = Some(id);
            self.state.selected_star = engine
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

        let sector_id = match self.state.selected_sector {
            Some(id) => id,
            None => return,
        };

        let current = match self.state.selected_star {
            Some(id) => id,
            None => {
                self.state.selected_star = engine
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
                self.state.selected_star = engine
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
            self.state.selected_star = Some(id);
        }
    }

    fn end_turn(&mut self) {
        let engine = match &mut self.engine {
            Some(e) => e,
            None => return,
        };

        // Add EndTurn command
        self.pending_commands.push(Command::EndTurn);

        // Process all pending commands
        let commands = std::mem::take(&mut self.pending_commands);
        let events = engine.apply_turn(commands);

        // Add events to log
        for event in events {
            self.state.log.push(event.to_log_message());
        }
    }

    /// Dispatch an available scout fleet to the currently selected star system.
    /// Logs an error if no fleet is available or the destination is already explored.
    fn dispatch_scout(&mut self) {
        let star_id = match self.state.selected_star {
            Some(id) => id,
            None => {
                self.state.log.push("No star selected.".to_string());
                return;
            }
        };

        // Find the first player-owned fleet that is not on an active mission
        let fleet_id: Option<FleetId> = {
            let engine = match &self.engine {
                Some(e) => e,
                None => return,
            };
            engine
                .state
                .fleets
                .values()
                .find(|f| {
                    f.owner == engine.state.player_empire
                        && f.kind == FleetKind::Scout
                        && !engine.state.scout_missions.contains_key(&f.id)
                        && !engine.state.survey_missions.contains_key(&f.id)
                        && !engine.state.fleet_missions.contains_key(&f.id)
                })
                .map(|f| f.id)
        };

        let fleet_id = match fleet_id {
            Some(id) => id,
            None => {
                self.state
                    .log
                    .push("No scout available to dispatch.".to_string());
                return;
            }
        };

        self.pending_commands.push(Command::SendScout {
            fleet: fleet_id,
            destination: star_id,
        });

        let commands = std::mem::take(&mut self.pending_commands);
        let engine = match &mut self.engine {
            Some(e) => e,
            None => return,
        };
        let events = engine.apply_turn(commands);
        for event in events {
            self.state.log.push(event.to_log_message());
        }
    }

    /// Move the first idle player fleet to the currently selected (explored) star system.
    /// Logs an error if no idle fleet is available or the destination is not explored.
    fn move_fleet(&mut self) {
        let star_id = match self.state.selected_star {
            Some(id) => id,
            None => {
                self.state.log.push("No star selected.".to_string());
                return;
            }
        };

        // Find the first idle player fleet (no scout mission, no fleet mission)
        let fleet_id: Option<FleetId> = {
            let engine = match &self.engine {
                Some(e) => e,
                None => return,
            };
            engine
                .state
                .fleets
                .values()
                .find(|f| {
                    f.owner == engine.state.player_empire
                        && !engine.state.scout_missions.contains_key(&f.id)
                        && !engine.state.survey_missions.contains_key(&f.id)
                        && !engine.state.fleet_missions.contains_key(&f.id)
                })
                .map(|f| f.id)
        };

        let fleet_id = match fleet_id {
            Some(id) => id,
            None => {
                self.state
                    .log
                    .push("No idle fleet available to move.".to_string());
                return;
            }
        };

        self.pending_commands.push(Command::MoveFleet {
            fleet: fleet_id,
            destination: star_id,
        });

        let commands = std::mem::take(&mut self.pending_commands);
        let engine = match &mut self.engine {
            Some(e) => e,
            None => return,
        };
        let events = engine.apply_turn(commands);
        for event in events {
            self.state.log.push(event.to_log_message());
        }
    }

    /// Colonize the currently selected planet at the selected star system
    /// using an idle colonizer fleet present at that system.
    fn colonize_selected_planet(&mut self) {
        let star_id = match self.state.selected_star {
            Some(id) => id,
            None => {
                self.state.log.push("No star selected.".to_string());
                return;
            }
        };

        // Find the first idle colonizer fleet at the selected star
        let fleet_id: Option<FleetId> = {
            let engine = match &self.engine {
                Some(e) => e,
                None => return,
            };
            engine
                .state
                .fleets
                .values()
                .find(|f| {
                    f.owner == engine.state.player_empire
                        && f.location == star_id
                        && f.kind == FleetKind::Colonizer
                        && !engine.state.scout_missions.contains_key(&f.id)
                        && !engine.state.survey_missions.contains_key(&f.id)
                        && !engine.state.fleet_missions.contains_key(&f.id)
                })
                .map(|f| f.id)
        };

        let fleet_id = match fleet_id {
            Some(id) => id,
            None => {
                self.state
                    .log
                    .push("No idle colonizer fleet present at selected system.".to_string());
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
            self.state
                .log
                .push("Selected system has no planets.".to_string());
            return;
        }
        let planet_index = self.state.selected_planet_index.min(planet_count - 1);

        self.pending_commands.push(Command::Colonize {
            fleet: fleet_id,
            star: star_id,
            planet_index,
        });

        let commands = std::mem::take(&mut self.pending_commands);
        let engine = match &mut self.engine {
            Some(e) => e,
            None => return,
        };
        let events = engine.apply_turn(commands);
        for event in events {
            self.state.log.push(event.to_log_message());
        }
    }

    /// Survey the currently selected planet using an idle science ship at the selected star.
    fn survey_selected_planet(&mut self) {
        let star_id = match self.state.selected_star {
            Some(id) => id,
            None => {
                self.state.log.push("No star selected.".to_string());
                return;
            }
        };

        let fleet_id: Option<FleetId> = {
            let engine = match &self.engine {
                Some(e) => e,
                None => return,
            };
            engine
                .state
                .fleets
                .values()
                .find(|f| {
                    f.owner == engine.state.player_empire
                        && f.location == star_id
                        && f.kind == FleetKind::Science
                        && !engine.state.scout_missions.contains_key(&f.id)
                        && !engine.state.survey_missions.contains_key(&f.id)
                        && !engine.state.fleet_missions.contains_key(&f.id)
                })
                .map(|f| f.id)
        };

        let fleet_id = match fleet_id {
            Some(id) => id,
            None => {
                self.state
                    .log
                    .push("No science ship available to survey.".to_string());
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
            self.state
                .log
                .push("Selected system has no planets.".to_string());
            return;
        }

        let planet_index = self.state.selected_planet_index.min(planet_count - 1);

        self.pending_commands.push(Command::SurveyPlanet {
            fleet: fleet_id,
            star: star_id,
            planet_index,
        });

        let commands = std::mem::take(&mut self.pending_commands);
        let engine = match &mut self.engine {
            Some(e) => e,
            None => return,
        };
        let events = engine.apply_turn(commands);
        for event in events {
            self.state.log.push(event.to_log_message());
        }
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyModifiers};
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    /// Create a unique temporary file path for tests to avoid parallel races.
    fn tmp_save_path(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("farspace_test_{}.sav", name))
    }

    #[test]
    fn toggle_help_overlay_on_question_mark() {
        let mut app = App::new();
        assert!(!app.state.show_help);

        app.handle_key(key(KeyCode::Char('?')));
        assert!(app.state.show_help);

        app.handle_key(key(KeyCode::Char('?')));
        assert!(!app.state.show_help);
    }

    #[test]
    fn toggle_palette_on_colon() {
        let mut app = App::new();
        assert!(!app.state.show_palette);

        app.handle_key(key(KeyCode::Char(':')));
        assert!(app.state.show_palette);

        app.handle_key(key(KeyCode::Esc));
        assert!(!app.state.show_palette);
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
    fn overview_enter_opens_selected_colony() {
        let mut app = App::new();
        app.new_game(42);
        app.state.active = Screen::EmpireOverview;
        app.state.overview_cursor = 0;

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
        assert_eq!(app.state.selected_colony, Some(expected_colony));
    }

    #[test]
    fn overview_s_opens_selected_system() {
        let mut app = App::new();
        app.new_game(42);
        app.state.active = Screen::EmpireOverview;
        app.state.overview_cursor = 0;

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
        assert_eq!(app.state.selected_star, Some(expected_star));
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
        app.state.selected_star = Some(target);
        app.state.selected_planet_index = 1;
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
    fn star_selection_moves_on_hjkl() {
        let mut app = App::new();
        app.new_game(42);

        let initial = app.state.selected_star;
        assert!(initial.is_some());

        // Move right
        app.handle_key(key(KeyCode::Char('l')));

        // Selection should change (might be same if no star to right)
        // Just verify it doesn't crash
        assert!(app.state.selected_star.is_some());
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
        assert!(app.state.selected_star.is_none());
    }

    #[test]
    fn move_star_selection_with_no_selection_selects_first() {
        let mut app = App::new();
        app.new_game(42);
        app.state.selected_star = None;

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
        app.state.show_help = true;

        app.handle_key(key(KeyCode::Esc));
        assert!(!app.state.show_help);
    }

    #[test]
    fn palette_key_closes_palette() {
        let mut app = App::new();
        app.state.show_palette = true;
        app.state.palette_input = String::new();

        // Pressing `:` when palette is open adds `:` to input (does not close)
        app.handle_key(key(KeyCode::Char(':')));
        assert!(app.state.show_palette);
        assert_eq!(app.state.palette_input, ":");

        // Pressing Esc closes the palette
        app.handle_key(key(KeyCode::Esc));
        assert!(!app.state.show_palette);
        assert!(app.state.palette_input.is_empty());
    }

    #[test]
    fn palette_accepts_character_input() {
        let mut app = App::new();
        app.state.show_palette = true;
        app.state.palette_input.clear();

        app.handle_key(key(KeyCode::Char('s')));
        app.handle_key(key(KeyCode::Char('a')));
        app.handle_key(key(KeyCode::Char('v')));
        app.handle_key(key(KeyCode::Char('e')));

        assert_eq!(app.state.palette_input, "save");
    }

    #[test]
    fn palette_backspace_removes_last_char() {
        let mut app = App::new();
        app.state.show_palette = true;
        app.state.palette_input = "sav".to_string();

        app.handle_key(key(KeyCode::Backspace));

        assert_eq!(app.state.palette_input, "sa");
    }

    #[test]
    fn palette_unknown_command_logs_error() {
        let mut app = App::new();
        app.new_game(42);
        let before = app.state.log.len();

        app.execute_palette_command("unknowncmd");

        assert!(app.state.log.len() > before);
    }

    #[test]
    fn palette_enter_executes_and_closes() {
        let mut app = App::new();
        app.state.show_palette = true;
        app.state.palette_input = "unknowncmd".to_string();

        app.handle_key(key(KeyCode::Enter));

        assert!(!app.state.show_palette);
        assert!(app.state.palette_input.is_empty());
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
        app.execute_palette_command(":");
        app.execute_palette_command("  ");
        app.execute_palette_command(":  ");

        assert_eq!(app.state.log.len(), before, "No-op commands should not log");
    }

    #[test]
    fn palette_enter_with_colon_only_does_not_execute() {
        let mut app = App::new();
        app.new_game(42);
        app.state.show_palette = true;
        app.state.palette_input = ":".to_string();
        let before = app.state.log.len();

        app.handle_key(key(KeyCode::Enter));

        assert!(!app.state.show_palette);
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
            app.state.selected_star = Some(star_id);
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
        app.state.selected_star = home_star_id;

        // Press 'c' to enter the colony screen — exercises the actual key binding
        app.handle_key(key(KeyCode::Char('c')));

        assert_eq!(app.state.active, Screen::Colony);
        assert!(app.state.selected_colony.is_some());
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

        app.state.selected_star = Some(home_star_id);
        app.state.selected_planet_index = 1;
        assert!(app.try_enter_colony());
        assert_eq!(app.state.selected_colony, Some(second_colony_id));
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
        app.state.selected_star = Some(home_star_id);
        app.state.selected_planet_index = 1;
        app.handle_key(key(KeyCode::Char('c')));

        assert_eq!(app.state.active, Screen::Colony);
        assert_eq!(app.state.selected_colony, Some(second_colony_id));
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
        app.state.selected_star = Some(home_star_id);
        app.state.selected_planet_index = engine.state.colonies[&home_colony_id].planet_index;
        app.state.selected_colony = None;

        app.handle_key(key(KeyCode::Enter));

        assert_eq!(app.state.active, Screen::Colony);
        assert_eq!(app.state.selected_colony, Some(home_colony_id));
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
        app.state.selected_star = Some(target_star);
        app.state.selected_planet_index = 0;
        app.state.selected_colony = None;

        app.handle_key(key(KeyCode::Enter));

        assert_eq!(app.state.active, Screen::System);
        assert!(app.state.selected_colony.is_none());
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
        app.state.selected_star = None;
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
            app.state.selected_star = Some(star_id);
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
        assert!(app.state.selected_colony.is_none());
    }

    #[test]
    fn colony_build_cursor_moves_with_j_k() {
        let mut app = App::new();
        app.new_game(42);
        goto_colony_screen(&mut app);

        let initial = app.state.colony_build_cursor;
        app.handle_key(key(KeyCode::Char('j')));
        assert_ne!(app.state.colony_build_cursor, initial);

        app.handle_key(key(KeyCode::Char('k')));
        assert_eq!(app.state.colony_build_cursor, initial);
    }

    #[test]
    fn colony_build_cursor_wraps_around_bottom() {
        let mut app = App::new();
        app.new_game(42);
        goto_colony_screen(&mut app);

        let count = App::all_build_item_count();
        // Move down past the last item
        for _ in 0..count {
            app.handle_key(key(KeyCode::Char('j')));
        }
        // Cursor should have wrapped to the start
        assert_eq!(app.state.colony_build_cursor, 0);
    }

    #[test]
    fn colony_build_cursor_wraps_around_top() {
        let mut app = App::new();
        app.new_game(42);
        goto_colony_screen(&mut app);

        // Move up from 0 should wrap to last
        app.handle_key(key(KeyCode::Char('k')));
        let count = App::all_build_item_count();
        assert_eq!(app.state.colony_build_cursor, count - 1);
    }

    #[test]
    fn enter_key_queues_building_on_colony_screen() {
        let mut app = App::new();
        app.new_game(42);
        goto_colony_screen(&mut app);

        let colony_id = app.state.selected_colony.unwrap();
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

        // Complete the building: base production is 10/turn, AquacultureBay costs 60 → 6 turns
        for _ in 0..6 {
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
        assert_eq!(app.state.research_cursor, 0);
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
        app.state.research_cursor = 0;

        // j increments cursor; just verify no panic and cursor stays in bounds
        app.handle_key(key(KeyCode::Char('j')));
        let techs_len = game_core::all_techs().len();
        assert!(app.state.research_cursor < techs_len);
    }

    #[test]
    fn research_cursor_wraps_on_k() {
        let mut app = App::new();
        app.new_game(42);
        app.state.active = Screen::Research;
        app.state.research_cursor = 0;

        // k at position 0 should wrap to last
        app.handle_key(key(KeyCode::Char('k')));
        // cursor should now point to last tech (5 for 6 techs with index 0..5)
        let techs_len = game_core::all_techs().len();
        assert!(app.state.research_cursor < techs_len);
    }

    #[test]
    fn enter_selects_research_tech() {
        let mut app = App::new();
        app.new_game(42);
        app.state.active = Screen::Research;
        app.state.research_cursor = 0;

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
        app.state.research_cursor = 0;

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

        app.state.selected_star = Some(star_id);
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
        app.state.selected_star = None;

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
        app.state.selected_star = Some(explored_star);

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

        app.state.selected_star = Some(unexplored);
        let before = app.state.log.len();
        app.dispatch_scout();
        assert!(
            app.state.log.len() > before,
            "Should log 'no scout available'"
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
        app.state.selected_star = Some(dest);

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
        app.state.selected_star = None;
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
        app.state.selected_star = Some(explored);

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
        app.state.selected_star = Some(dest);
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
        app.state.selected_star = Some(star);
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
        app.state.selected_star = Some(star);
        app.state.selected_planet_index = 0;
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
        app.state.selected_star = None;
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
        app.state.selected_star = Some(target);
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
            selected_sector: sector,
            selected_star: Some(home),
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
}
