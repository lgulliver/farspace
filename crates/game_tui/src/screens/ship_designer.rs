//! Ship Designer screen — hull selection, slot configuration, design management.

use crate::components::{derive_header_data, render_footer, render_header};
use crate::layout::compose_layout;
use crate::screens::Screen;
use crate::theme::Theme;
use crate::AppState;
use game_core::{
    all_hull_templates, components_for_slot, is_component_unlocked, ComponentDef, ComponentId,
    CustomShipDesign, DerivedShipStats, FleetKind, GameState, HullTemplate, SlotCategory, TechId,
};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

// ── State types ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DesignerMode {
    #[default]
    Browse,
    NewDesign,
    EditSlots,
    ConfirmSave,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DesignerPanel {
    #[default]
    DesignList,
    SlotConfig,
    Stats,
}

#[derive(Debug, Clone, Default)]
pub struct ShipDesignerState {
    pub mode: DesignerMode,
    pub selected_hull_idx: usize,
    pub selected_design_idx: usize,
    pub active_slot_idx: usize,
    pub component_cursor: usize,
    pub name_input: Option<String>,
    pub panel: DesignerPanel,
    pub current_components: Vec<ComponentId>,
}

impl ShipDesignerState {
    pub fn reset_to_browse(&mut self) {
        self.mode = DesignerMode::Browse;
        self.panel = DesignerPanel::DesignList;
        self.current_components.clear();
        self.name_input = None;
        self.active_slot_idx = 0;
        self.component_cursor = 0;
    }

    pub fn begin_new_design(&mut self) {
        self.mode = DesignerMode::NewDesign;
        self.panel = DesignerPanel::SlotConfig;
        self.selected_design_idx = 0;
        self.current_components.clear();
        self.name_input = None;
        self.active_slot_idx = 0;
        self.component_cursor = 0;
    }

    pub fn begin_edit_slots(&mut self, hull: &HullTemplate) {
        self.mode = DesignerMode::EditSlots;
        self.active_slot_idx = 0;
        self.component_cursor = 0;
        // Pre-fill with first available component per slot (ComponentId(0) = no component yet)
        self.current_components = hull
            .slots
            .iter()
            .map(|&cat| {
                game_core::components_for_slot(cat)
                    .into_iter()
                    .find(|c| c.required_tech.is_none())
                    .map(|c| c.component_id)
                    .unwrap_or(ComponentId(0))
            })
            .collect();
    }
}

// ── Render ────────────────────────────────────────────────────────────────────

pub fn render_ship_designer(
    frame: &mut Frame,
    area: Rect,
    app_state: &AppState,
    game_state: &GameState,
) {
    let (header_area, main_area, footer_area) = compose_layout(area);

    let header_data = derive_header_data(game_state);
    render_header(frame, header_area, &header_data);

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(28),
            Constraint::Percentage(44),
            Constraint::Percentage(28),
        ])
        .split(main_area);

    render_design_list(frame, cols[0], app_state, game_state);
    render_slot_config(frame, cols[1], app_state, game_state);
    render_stats_panel(frame, cols[2], app_state, game_state);

    render_footer(frame, footer_area, &Screen::ShipDesigner, None);
}

// ── Left panel: Design List ───────────────────────────────────────────────────

fn render_design_list(frame: &mut Frame, area: Rect, app_state: &AppState, game_state: &GameState) {
    let ds = &app_state.ship_designer;
    let focused = ds.panel == DesignerPanel::DesignList;
    let border_style = if focused {
        Theme::focused_border_style()
    } else {
        Theme::dim_border_style()
    };
    let block = Block::default()
        .title(" Ship Designs ")
        .borders(Borders::ALL)
        .border_style(border_style)
        .style(Theme::default_style());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let player = game_state.player_empire;
    let existing: Vec<_> = game_state
        .custom_designs
        .values()
        .filter(|d| d.owner == player && !d.obsolete)
        .collect();

    let mut lines: Vec<Line> = Vec::new();

    // "New Design" entry (index 0)
    let new_selected = ds.selected_design_idx == 0;
    let prefix = if new_selected && focused { ">" } else { " " };
    let style = if new_selected && focused {
        Theme::highlight_style()
    } else {
        Theme::default_style()
    };
    lines.push(Line::from(vec![Span::styled(
        format!(" {prefix} New Design [---]"),
        style,
    )]));

    if !existing.is_empty() {
        lines.push(Line::from(vec![Span::styled(
            " Saved Designs",
            Theme::muted_style(),
        )]));
        for (i, design) in existing.iter().enumerate() {
            let idx = i + 1;
            let is_sel = ds.selected_design_idx == idx;
            let pfx = if is_sel && focused { ">" } else { " " };
            let sty = if is_sel && focused {
                Theme::highlight_style()
            } else {
                Theme::default_style()
            };
            let tag = design
                .hull_id
                .template()
                .map(|h| fleet_kind_tag(h.fleet_kind))
                .unwrap_or("???");
            lines.push(Line::from(vec![Span::styled(
                format!(" {pfx} {} [{tag}]", design.name),
                sty,
            )]));
        }
    }

    let paragraph = Paragraph::new(lines).style(Theme::default_style());
    frame.render_widget(paragraph, inner);
}

// ── Center panel: Slot Config ─────────────────────────────────────────────────

fn render_slot_config(frame: &mut Frame, area: Rect, app_state: &AppState, game_state: &GameState) {
    let ds = &app_state.ship_designer;
    let focused = ds.panel == DesignerPanel::SlotConfig;
    let border_style = if focused {
        Theme::focused_border_style()
    } else {
        Theme::dim_border_style()
    };
    let block = Block::default()
        .title(" Slot Configuration ")
        .borders(Borders::ALL)
        .border_style(border_style)
        .style(Theme::default_style());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let completed: Vec<TechId> = game_state
        .empires
        .get(&game_state.player_empire)
        .map(|e| e.research.completed.clone())
        .unwrap_or_default();

    let mut lines: Vec<Line> = Vec::new();

    match ds.mode {
        DesignerMode::Browse => {
            // Show read-only info for selected design if any
            if ds.selected_design_idx == 0 {
                lines.push(Line::from(Span::styled(
                    " Select a design or press [n] to create one.",
                    Theme::muted_style(),
                )));
            } else {
                let player = game_state.player_empire;
                let existing: Vec<_> = game_state
                    .custom_designs
                    .values()
                    .filter(|d| d.owner == player && !d.obsolete)
                    .collect();
                let design_idx = (ds.selected_design_idx - 1).min(existing.len().saturating_sub(1));
                if let Some(design) = existing.get(design_idx) {
                    if let Some(hull) = design.hull_id.template() {
                        lines.push(Line::from(Span::styled(
                            format!(" Hull: {}", hull.name),
                            Theme::title_style(),
                        )));
                        lines.push(Line::from(Span::raw("")));
                        for (slot_idx, cat) in hull.slots.iter().enumerate() {
                            let comp_id = design.components.get(slot_idx).copied();
                            let comp_name = comp_id
                                .and_then(|c| c.def())
                                .map(|d| d.name)
                                .unwrap_or("(empty)");
                            lines.push(Line::from(vec![Span::styled(
                                format!(" [{}] {}", slot_category_label(*cat), comp_name),
                                Theme::default_style(),
                            )]));
                        }
                    }
                }
            }
        }
        DesignerMode::NewDesign => {
            lines.push(Line::from(Span::styled(
                " Select a hull (j/k, Enter to confirm):",
                Theme::muted_style(),
            )));
            lines.push(Line::from(Span::raw("")));
            let hulls = all_hull_templates();
            for (i, hull) in hulls.iter().enumerate() {
                let is_sel = ds.selected_hull_idx == i;
                let locked = hull
                    .required_tech
                    .map(|t| !completed.contains(&t))
                    .unwrap_or(false);
                let suffix = if locked { " (locked)" } else { "" };
                let pfx = if is_sel { ">" } else { " " };
                let sty = if locked {
                    Theme::muted_style()
                } else if is_sel {
                    Theme::highlight_style()
                } else {
                    Theme::default_style()
                };
                lines.push(Line::from(vec![Span::styled(
                    format!(
                        " {pfx} {} [{}]{}",
                        hull.name,
                        fleet_kind_tag(hull.fleet_kind),
                        suffix
                    ),
                    sty,
                )]));
            }
        }
        DesignerMode::EditSlots | DesignerMode::ConfirmSave => {
            let hull_opt = all_hull_templates().get(ds.selected_hull_idx);
            if let Some(hull) = hull_opt {
                lines.push(Line::from(Span::styled(
                    format!(" Hull: {} — configure slots (h/l cycle)", hull.name),
                    Theme::muted_style(),
                )));
                lines.push(Line::from(Span::raw("")));
                for (slot_idx, cat) in hull.slots.iter().enumerate() {
                    let is_active_slot = ds.active_slot_idx == slot_idx;
                    let slot_label = slot_category_label(*cat);
                    let slot_style = if is_active_slot {
                        Theme::accent_style()
                    } else {
                        Theme::title_style()
                    };
                    lines.push(Line::from(vec![Span::styled(
                        format!(" [{slot_label}]"),
                        slot_style,
                    )]));

                    let comps = components_for_slot(*cat);
                    for (ci, comp) in comps.iter().enumerate() {
                        let chosen_id = ds
                            .current_components
                            .get(slot_idx)
                            .copied()
                            .unwrap_or(ComponentId(0));
                        let is_chosen = chosen_id == comp.component_id;
                        let is_cursor = is_active_slot && ds.component_cursor == ci;
                        let locked = !is_component_unlocked(comp.component_id, &completed);
                        let bullet = if is_chosen { "●" } else { "○" };
                        let lock_suffix = if locked { " [locked]" } else { "" };
                        let stat_tag = build_stat_tag(comp);
                        let sty = if locked {
                            Theme::muted_style()
                        } else if is_cursor {
                            Theme::highlight_style()
                        } else if is_chosen {
                            Theme::accent_style()
                        } else {
                            Theme::default_style()
                        };
                        lines.push(Line::from(vec![Span::styled(
                            format!("   {bullet} {}{}{}", comp.name, stat_tag, lock_suffix),
                            sty,
                        )]));
                    }
                    lines.push(Line::from(Span::raw("")));
                }
            } else {
                lines.push(Line::from(Span::styled(
                    " No hull selected.",
                    Theme::muted_style(),
                )));
            }
        }
    }

    let paragraph = Paragraph::new(lines).style(Theme::default_style());
    frame.render_widget(paragraph, inner);
}

// ── Right panel: Stats ────────────────────────────────────────────────────────

fn render_stats_panel(frame: &mut Frame, area: Rect, app_state: &AppState, game_state: &GameState) {
    let ds = &app_state.ship_designer;
    let focused = ds.panel == DesignerPanel::Stats;
    let border_style = if focused {
        Theme::focused_border_style()
    } else {
        Theme::dim_border_style()
    };
    let block = Block::default()
        .title(" Design Stats ")
        .borders(Borders::ALL)
        .border_style(border_style)
        .style(Theme::default_style());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines: Vec<Line> = Vec::new();

    let player = game_state.player_empire;

    match ds.mode {
        DesignerMode::Browse => {
            if ds.selected_design_idx == 0 {
                lines.push(Line::from(Span::styled(" —", Theme::muted_style())));
            } else {
                let existing: Vec<_> = game_state
                    .custom_designs
                    .values()
                    .filter(|d| d.owner == player && !d.obsolete)
                    .collect();
                let design_idx = (ds.selected_design_idx - 1).min(existing.len().saturating_sub(1));
                if let Some(design) = existing.get(design_idx) {
                    push_design_stats(&mut lines, design, game_state);
                }
            }
        }
        DesignerMode::NewDesign | DesignerMode::EditSlots | DesignerMode::ConfirmSave => {
            if let Some(hull) = all_hull_templates().get(ds.selected_hull_idx) {
                let components: Vec<ComponentId> = ds
                    .current_components
                    .iter()
                    .filter(|c| c.0 != 0)
                    .copied()
                    .collect();
                let scratch = CustomShipDesign {
                    design_id: game_core::CustomDesignId(0),
                    hull_id: hull.hull_id,
                    components,
                    owner: player,
                    name: ds
                        .name_input
                        .clone()
                        .unwrap_or_else(|| format!("{} Design", hull.name)),
                    obsolete: false,
                };
                let stats = scratch.derived_stats();
                push_derived_stats(&mut lines, hull.name, hull.role, &stats);
            } else {
                lines.push(Line::from(Span::styled(
                    " No hull selected.",
                    Theme::muted_style(),
                )));
            }
        }
    }

    lines.push(Line::from(Span::raw("")));
    lines.push(Line::from(vec![Span::styled(
        " [s] Save  [d] Delete  [Enter] Slots",
        Theme::muted_style(),
    )]));

    let paragraph = Paragraph::new(lines).style(Theme::default_style());
    frame.render_widget(paragraph, inner);
}

fn push_design_stats(lines: &mut Vec<Line>, design: &CustomShipDesign, _game_state: &GameState) {
    let hull = design.hull_id.template();
    let hull_name = hull.map(|h| h.name).unwrap_or("Unknown");
    let hull_role = hull.map(|h| h.role).unwrap_or("—");
    let stats = design.derived_stats();
    push_derived_stats(lines, hull_name, hull_role, &stats);
}

fn push_derived_stats(
    lines: &mut Vec<Line>,
    hull_name: &str,
    role: &str,
    stats: &DerivedShipStats,
) {
    lines.push(Line::from(vec![Span::styled(
        format!(" Hull:  {hull_name}"),
        Theme::title_style(),
    )]));
    lines.push(Line::from(vec![Span::styled(
        format!(" Role:  {role}"),
        Theme::default_style(),
    )]));
    lines.push(Line::from(Span::raw("")));
    lines.push(Line::from(vec![Span::styled(
        format!(" ATK:   {}", stats.attack),
        Theme::default_style(),
    )]));
    lines.push(Line::from(vec![Span::styled(
        format!(" DEF:   {}", stats.defense),
        Theme::default_style(),
    )]));
    lines.push(Line::from(vec![Span::styled(
        format!(" HP:    {}", stats.hp),
        Theme::default_style(),
    )]));
    lines.push(Line::from(vec![Span::styled(
        format!(" Cost:  {}pp", stats.production_cost),
        Theme::default_style(),
    )]));
    lines.push(Line::from(vec![Span::styled(
        format!(" Maint: {}/turn", stats.maintenance),
        Theme::default_style(),
    )]));
}

// ── Helper functions ──────────────────────────────────────────────────────────

#[allow(dead_code)]
fn current_hull(ds: &ShipDesignerState, _game_state: &GameState) -> Option<&'static HullTemplate> {
    all_hull_templates().get(ds.selected_hull_idx)
}

fn slot_category_label(cat: SlotCategory) -> &'static str {
    match cat {
        SlotCategory::Weapon => "WPN",
        SlotCategory::Defense => "DEF",
        SlotCategory::Engine => "ENG",
        SlotCategory::MissionModule => "MIS",
        SlotCategory::Utility => "UTL",
    }
}

fn fleet_kind_tag(kind: FleetKind) -> &'static str {
    match kind {
        FleetKind::Scout | FleetKind::FastScout | FleetKind::SurveyCutter => "EXP",
        FleetKind::Colonizer | FleetKind::ColonyArk => "COL",
        FleetKind::Science => "SCI",
        FleetKind::TroopTransport => "TRP",
        FleetKind::EscortFrigate
        | FleetKind::MissileFrigate
        | FleetKind::Destroyer
        | FleetKind::PatrolCorvette => "MIL",
    }
}

fn build_stat_tag(comp: &ComponentDef) -> String {
    let mut parts: Vec<String> = Vec::new();
    if comp.attack_modifier != 0 {
        parts.push(format!("{:+}ATK", comp.attack_modifier));
    }
    if comp.defense_modifier != 0 {
        parts.push(format!("{:+}DEF", comp.defense_modifier));
    }
    if comp.hp_modifier != 0 {
        parts.push(format!("{:+}HP", comp.hp_modifier));
    }
    if comp.movement_modifier != 0 {
        parts.push(format!("{:+}MOV", comp.movement_modifier));
    }
    if parts.is_empty() {
        String::new()
    } else {
        format!(" ({})", parts.join(" "))
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use game_core::HullId;

    #[test]
    fn initial_state_is_browse_mode() {
        let state = ShipDesignerState::default();
        assert_eq!(state.mode, DesignerMode::Browse);
    }

    #[test]
    fn begin_new_design_sets_mode() {
        let mut state = ShipDesignerState::default();
        state.begin_new_design();
        assert_eq!(state.mode, DesignerMode::NewDesign);
        assert_eq!(state.panel, DesignerPanel::SlotConfig);
    }

    #[test]
    fn reset_clears_scratch_state() {
        let mut state = ShipDesignerState::default();
        state.begin_new_design();
        state.current_components.push(ComponentId(1));
        state.reset_to_browse();
        assert_eq!(state.mode, DesignerMode::Browse);
        assert!(state.current_components.is_empty());
        assert!(state.name_input.is_none());
    }

    #[test]
    fn begin_edit_slots_sizes_components() {
        let mut state = ShipDesignerState::default();
        let hulls = all_hull_templates();
        let scout = hulls
            .iter()
            .find(|h| h.hull_id == HullId::SCOUT)
            .expect("scout hull must exist");
        state.begin_edit_slots(scout);
        assert_eq!(state.mode, DesignerMode::EditSlots);
        assert_eq!(state.current_components.len(), scout.slots.len());
    }

    #[test]
    fn slot_category_label_covers_all() {
        let cats = [
            SlotCategory::Weapon,
            SlotCategory::Defense,
            SlotCategory::Engine,
            SlotCategory::MissionModule,
            SlotCategory::Utility,
        ];
        for cat in cats {
            assert!(!slot_category_label(cat).is_empty());
        }
    }
}
