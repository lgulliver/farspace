//! Main menu screen

use crate::components::render_brand_header;
use crate::map_render::visual_hash;
use crate::renderer::{
    starfield::{detail_star_glyph, should_render_star, starfield_detail},
    Canvas,
};
use crate::theme::{gradient, lerp_rgb, SplashPalette, Theme};
use crate::update::UpdateState;
use crate::AppState;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

const MENU_STARFIELD_SALT: u64 = 0x4D45_4E55;
const MENU_STARFIELD_TWINKLE_SALT_XOR: u64 = 0x51;
const MENU_STARFIELD_HAZE_SALT_XOR: u64 = 0x3A;
const MIN_SPLASH_WIDTH: u16 = 80;
const MIN_SPLASH_HEIGHT: u16 = 24;
const WIDE_TITLE_WIDTH: u16 = 110;
const MEDIUM_TITLE_WIDTH: u16 = 70;
const MENU_ACTION_COUNT: usize = 5;

const TAGLINE: &str = "CHART • EXPAND • ENDURE";
const FOOTER_HINT: &str = "Enter Select   ↑↓ Move   ? Help   Esc Quit";
const FOOTER_HINT_COMPACT: &str = "Enter Select   ↑↓ Move   Esc Quit";

const TITLE_LINES_WIDE: &[&str] = &[
    "  ███████╗ █████╗ ██████╗ ███████╗██████╗  █████╗  ██████╗███████╗",
    "  ██╔════╝██╔══██╗██╔══██╗██╔════╝██╔══██╗██╔══██╗██╔════╝██╔════╝",
    "  █████╗  ███████║██████╔╝███████╗██████╔╝███████║██║     █████╗  ",
    "  ██╔══╝  ██╔══██║██╔══██╗╚════██║██╔═══╝ ██╔══██║██║     ██╔══╝  ",
    "  ██║     ██║  ██║██║  ██║███████║██║     ██║  ██║╚██████╗███████╗",
    "  ╚═╝     ╚═╝  ╚═╝╚═╝  ╚═╝╚══════╝╚═╝     ╚═╝  ╚═╝ ╚═════╝╚══════╝",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuAction {
    Continue,
    NewGame,
    LoadGame,
    Options,
    Quit,
}

impl MenuAction {
    pub const fn from_cursor(cursor: usize) -> Self {
        match cursor % MENU_ACTION_COUNT {
            0 => Self::Continue,
            1 => Self::NewGame,
            2 => Self::LoadGame,
            3 => Self::Options,
            _ => Self::Quit,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Continue => "Continue",
            Self::NewGame => "New Game",
            Self::LoadGame => "Load Game",
            Self::Options => "Options",
            Self::Quit => "Quit",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SplashLayout {
    frame: Rect,
    title: Rect,
    menu: Rect,
    footer: Rect,
}

pub const fn menu_action_count() -> usize {
    MENU_ACTION_COUNT
}

fn build_layout(main_area: Rect) -> SplashLayout {
    // Frame fills almost the full viewport with small margin.
    let frame_width = main_area.width.saturating_sub(4).max(64);
    let frame_height = main_area.height.saturating_sub(2).max(18);
    let frame_x = main_area.x + (main_area.width.saturating_sub(frame_width)) / 2;
    let frame_y = main_area.y + (main_area.height.saturating_sub(frame_height)) / 2;
    let frame = Rect::new(frame_x, frame_y, frame_width, frame_height);
    let inner = Rect::new(
        frame.x.saturating_add(2),
        frame.y.saturating_add(1),
        frame.width.saturating_sub(4),
        frame.height.saturating_sub(2),
    );
    let title_height = if frame.width >= WIDE_TITLE_WIDTH {
        8
    } else if frame.width >= MEDIUM_TITLE_WIDTH {
        6
    } else {
        4
    };
    let title_gap = if frame.height >= 22 { 2 } else { 1 };
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(title_height),
            Constraint::Length(title_gap),
            Constraint::Length(6),
            Constraint::Fill(1),
            Constraint::Length(2),
        ])
        .split(inner);

    SplashLayout {
        frame,
        title: sections[1],
        menu: sections[3],
        footer: sections[5],
    }
}

fn build_title_lines(area_width: u16, palette: SplashPalette) -> Vec<Line<'static>> {
    if area_width >= WIDE_TITLE_WIDTH {
        let colors = gradient(
            palette.title_primary,
            palette.title_secondary,
            TITLE_LINES_WIDE.len(),
        );
        let mut lines = TITLE_LINES_WIDE
            .iter()
            .enumerate()
            .map(|(index, line)| {
                Line::from(Span::styled(
                    (*line).to_string(),
                    Style::default()
                        .fg(colors[index])
                        .add_modifier(Modifier::BOLD),
                ))
            })
            .collect::<Vec<_>>();
        lines.push(Line::from(""));
        lines.push(build_tagline_line(area_width, palette));
        lines
    } else if area_width >= MEDIUM_TITLE_WIDTH {
        vec![
            Line::from(Span::styled(
                "F A R S P A C E",
                Style::default()
                    .fg(palette.title_primary)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            build_tagline_line(area_width, palette),
        ]
    } else {
        vec![
            Line::from(Span::styled(
                "F A R S P A C E",
                Style::default()
                    .fg(palette.title_primary)
                    .add_modifier(Modifier::BOLD),
            )),
            build_tagline_line(area_width, palette),
        ]
    }
}

fn build_tagline_line(area_width: u16, palette: SplashPalette) -> Line<'static> {
    if area_width >= 88 {
        Line::from(vec![
            Span::styled("── ", Style::default().fg(palette.border_cold)),
            Span::styled(
                TAGLINE,
                Style::default()
                    .fg(palette.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" ──", Style::default().fg(palette.border_cold)),
        ])
    } else {
        Line::from(Span::styled(
            TAGLINE,
            Style::default()
                .fg(palette.accent)
                .add_modifier(Modifier::BOLD),
        ))
    }
}

fn build_menu_lines(app_state: &AppState, palette: SplashPalette) -> Vec<Line<'static>> {
    (0..MENU_ACTION_COUNT)
        .map(|index| {
            let action = MenuAction::from_cursor(index);
            let selected = app_state.menu_cursor % MENU_ACTION_COUNT == index;
            // Continue is disabled when there is no campaign to resume; it stays
            // visible but muted rather than vanishing, so the layout is stable.
            let disabled = action == MenuAction::Continue && !app_state.can_continue;
            let marker = if selected { "▶ " } else { "  " };
            let marker_style = if selected {
                Style::default()
                    .fg(palette.accent)
                    .bg(lerp_rgb(palette.void_bg, palette.nebula_b, 0.18))
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(palette.text_muted)
            };
            let label_style = if disabled {
                Style::default()
                    .fg(palette.text_muted)
                    .add_modifier(Modifier::DIM)
            } else if selected {
                Style::default()
                    .fg(palette.accent)
                    .bg(lerp_rgb(palette.void_bg, palette.nebula_b, 0.18))
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Theme::fg())
            };

            Line::from(vec![
                Span::styled(marker.to_string(), marker_style),
                Span::styled(action.label().to_string(), label_style),
            ])
        })
        .collect()
}

fn render_gradient_frame(frame: &mut Frame, area: Rect, palette: SplashPalette) {
    if area.width < 2 || area.height < 2 {
        return;
    }

    let edge = gradient(palette.border_cold, palette.border_hot, area.width as usize);
    let buffer = frame.buffer_mut();

    for x in 0..area.width {
        let top_glyph = match x {
            0 => '╭',
            value if value == area.width.saturating_sub(1) => '╮',
            _ => '─',
        };
        let bottom_glyph = match x {
            0 => '╰',
            value if value == area.width.saturating_sub(1) => '╯',
            _ => '─',
        };
        let top_style = Style::default()
            .fg(edge[x as usize])
            .bg(palette.void_bg)
            .add_modifier(Modifier::BOLD);
        let bottom_style = Style::default()
            .fg(edge[area.width.saturating_sub(1) as usize - x as usize])
            .bg(palette.void_bg)
            .add_modifier(Modifier::BOLD);

        if let Some(cell) = buffer.cell_mut((area.x + x, area.y)) {
            cell.set_char(top_glyph);
            cell.set_style(top_style);
        }
        if let Some(cell) = buffer.cell_mut((area.x + x, area.bottom().saturating_sub(1))) {
            cell.set_char(bottom_glyph);
            cell.set_style(bottom_style);
        }
    }

    for y in 1..area.height.saturating_sub(1) {
        let color = lerp_rgb(
            palette.border_cold,
            palette.border_hot,
            y as f32 / area.height.saturating_sub(1) as f32,
        );
        let style = Style::default()
            .fg(color)
            .bg(palette.void_bg)
            .add_modifier(Modifier::BOLD);

        if let Some(cell) = buffer.cell_mut((area.x, area.y + y)) {
            cell.set_char('│');
            cell.set_style(style);
        }
        if let Some(cell) = buffer.cell_mut((area.right().saturating_sub(1), area.y + y)) {
            cell.set_char('│');
            cell.set_style(style);
        }
    }
}

fn render_menu_starfield(
    frame: &mut Frame,
    area: Rect,
    app_state: &AppState,
    palette: SplashPalette,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    let mut canvas = Canvas::new(area.width, area.height);
    let detail = starfield_detail(area);
    let frame_group = if app_state.reduced_motion {
        0
    } else {
        app_state.tick_count / 10
    };

    for y in 0..area.height {
        for x in 0..area.width {
            let base_hash = visual_hash(0, x, y, 0, MENU_STARFIELD_SALT);
            let drift_hash = visual_hash(0, x, y, frame_group, MENU_STARFIELD_SALT ^ 0xA5);
            let haze_hash = visual_hash(
                0,
                x,
                y,
                0,
                MENU_STARFIELD_SALT ^ MENU_STARFIELD_HAZE_SALT_XOR,
            );
            let drift = (drift_hash % 100) as f32 / 100.0;
            let depth = (base_hash % 100) as f32 / 100.0;
            let bg_color = lerp_rgb(
                lerp_rgb(palette.void_bg, palette.nebula_a, depth * 0.12),
                palette.nebula_b,
                drift * 0.05,
            );

            let mut symbol = ' ';
            let mut style = Style::default().bg(bg_color);
            let haze = haze_hash % 2600;

            if haze == 0 {
                symbol = '·';
                style = style.fg(lerp_rgb(palette.nebula_a, palette.text_muted, 0.14));
            } else if haze == 1 {
                symbol = '.';
                style = style.fg(lerp_rgb(palette.nebula_a, palette.text_muted, 0.12));
            }

            if should_render_star(base_hash, detail) && base_hash.is_multiple_of(9) {
                let twinkle_hash = visual_hash(
                    0,
                    x,
                    y,
                    frame_group,
                    MENU_STARFIELD_SALT ^ MENU_STARFIELD_TWINKLE_SALT_XOR,
                );
                symbol = if twinkle_hash.is_multiple_of(9) {
                    '✦'
                } else {
                    detail_star_glyph(base_hash, detail)
                };
                let sparkle = 0.5 + (twinkle_hash % 20) as f32 / 100.0;
                style = Style::default().bg(bg_color).fg(lerp_rgb(
                    palette.title_primary,
                    palette.star_core,
                    sparkle,
                ));
            }

            canvas.set_cell(x, y, symbol, style, 0);
        }
    }

    canvas.render_to_buffer(area, frame.buffer_mut());
}

fn render_footer_line(
    frame: &mut Frame,
    area: Rect,
    palette: SplashPalette,
    hint: &str,
    show_version: bool,
) {
    let footer_bg = lerp_rgb(palette.void_bg, palette.nebula_a, 0.08);
    frame.render_widget(
        Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(palette.border_cold))
            .style(Style::default().bg(footer_bg)),
        area,
    );

    let inner = Rect::new(
        area.x.saturating_add(1),
        area.y.saturating_add(1),
        area.width.saturating_sub(2),
        area.height.saturating_sub(1),
    );
    if show_version && inner.width > 28 {
        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Fill(1), Constraint::Length(16)])
            .split(inner);

        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                hint,
                Style::default().fg(palette.text_muted),
            )))
            .alignment(Alignment::Left)
            .style(Style::default().bg(footer_bg)),
            columns[0],
        );
        frame.render_widget(
            Paragraph::new(Span::styled(
                build_info(),
                Style::default().fg(palette.text_muted),
            ))
            .alignment(Alignment::Right)
            .style(Style::default().bg(footer_bg)),
            columns[1],
        );
    } else {
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                hint,
                Style::default().fg(palette.text_muted),
            )))
            .alignment(Alignment::Center)
            .style(Style::default().bg(footer_bg)),
            inner,
        );
    }
}

fn render_compact_menu(
    frame: &mut Frame,
    area: Rect,
    app_state: &AppState,
    palette: SplashPalette,
) {
    let content = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(1),
            Constraint::Length(5),
            Constraint::Length(1),
        ])
        .split(Rect::new(
            area.x + area.width.saturating_sub(34) / 2,
            area.y + area.height.saturating_sub(11) / 2,
            area.width.min(34),
            area.height.min(11),
        ));

    frame.render_widget(
        Clear,
        Rect::new(content[0].x, content[0].y, content[0].width, 11),
    );
    render_brand_header(frame, content[0], false);
    frame.render_widget(
        Paragraph::new(build_menu_lines(app_state, palette))
            .alignment(Alignment::Center)
            .style(Style::default().bg(palette.void_bg)),
        content[2],
    );
    render_footer_line(frame, content[3], palette, FOOTER_HINT_COMPACT, false);
}

fn render_dashboard(frame: &mut Frame, area: Rect, app_state: &AppState, palette: SplashPalette) {
    let layout = build_layout(area);
    frame.render_widget(
        Block::default().style(Style::default().bg(palette.void_bg)),
        layout.frame,
    );
    render_gradient_frame(frame, layout.frame, palette);

    let title = Paragraph::new(build_title_lines(layout.frame.width, palette))
        .alignment(Alignment::Center)
        .style(Style::default().bg(palette.void_bg))
        .wrap(Wrap { trim: false });
    frame.render_widget(title, layout.title);

    let menu_width = layout.menu.width.min(30);
    let menu_area = Rect::new(
        layout.menu.x + layout.menu.width.saturating_sub(menu_width) / 2,
        layout.menu.y,
        menu_width,
        layout.menu.height,
    );
    frame.render_widget(
        Block::default().style(Style::default().bg(lerp_rgb(
            palette.void_bg,
            palette.nebula_a,
            0.10,
        ))),
        menu_area,
    );
    let menu = Paragraph::new(build_menu_lines(app_state, palette))
        .alignment(Alignment::Left)
        .style(Style::default().bg(lerp_rgb(palette.void_bg, palette.nebula_a, 0.10)));
    frame.render_widget(menu, menu_area);

    render_footer_line(frame, layout.footer, palette, FOOTER_HINT, true);

    // Update notification — rendered above footer when available
    if app_state.update_state.is_notifiable() {
        render_update_notice(frame, layout.footer, app_state, palette);
    }
}

fn build_info() -> String {
    match option_env!("FARSPACE_BUILD_TAG") {
        Some(tag) if !tag.is_empty() => format!("v{}-{tag}", env!("CARGO_PKG_VERSION")),
        _ => format!("v{}", env!("CARGO_PKG_VERSION")),
    }
}

fn render_update_notice(
    frame: &mut Frame,
    footer_area: Rect,
    app_state: &AppState,
    palette: SplashPalette,
) {
    let text = match &app_state.update_state {
        UpdateState::Available(info) => {
            format!("▲ Update available: {}  [U] Download", info.version)
        }
        UpdateState::Downloading => "⬇ Downloading update…".to_string(),
        UpdateState::Staged { version } => {
            format!("✓ {version} ready  [U] Apply & Restart")
        }
        UpdateState::Error(e) => format!("⚠ Update check failed: {e}"),
        _ => return,
    };
    // Render as a 1-row notice just above the footer separator.
    if footer_area.y == 0 {
        return;
    }
    let notice_area = Rect::new(
        footer_area.x.saturating_add(2),
        footer_area.y.saturating_sub(1),
        footer_area.width.saturating_sub(4),
        1,
    );
    let notice_style = match &app_state.update_state {
        UpdateState::Available(_) => Style::default().fg(palette.warning),
        UpdateState::Staged { .. } => Style::default().fg(palette.accent),
        UpdateState::Error(_) => Style::default().fg(palette.warning),
        _ => Style::default().fg(palette.text_muted),
    };
    frame.render_widget(
        Paragraph::new(Span::styled(text, notice_style)).alignment(Alignment::Center),
        notice_area,
    );
}

/// Render the main menu
pub fn render_menu(frame: &mut Frame, area: Rect, app_state: &AppState) {
    let palette = Theme::splash_palette();

    // Starfield fills the entire terminal area — no header/footer strip carved out.
    render_menu_starfield(frame, area, app_state, palette);

    if area.width < MIN_SPLASH_WIDTH || area.height < MIN_SPLASH_HEIGHT {
        render_compact_menu(frame, area, app_state, palette);
    } else {
        render_dashboard(frame, area, app_state, palette);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::buffer::Buffer;
    use ratatui::Terminal;

    fn cell_char(buf: &Buffer, x: u16, y: u16) -> char {
        buf.cell((x, y))
            .and_then(|c| c.symbol().chars().next())
            .unwrap_or(' ')
    }

    fn render_to_buffer(width: u16, height: u16) -> Buffer {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| render_menu(frame, frame.area(), &AppState::default()))
            .unwrap();
        terminal.backend().buffer().clone()
    }

    fn buffer_text(buf: &Buffer, width: u16, height: u16) -> String {
        (0..height)
            .flat_map(|y| (0..width).map(move |x| cell_char(buf, x, y)))
            .collect()
    }

    #[test]
    fn menu_screen_renders_without_panic() {
        render_to_buffer(100, 30);
    }

    #[test]
    fn menu_screen_renders_in_small_terminal() {
        render_to_buffer(60, 18);
    }

    #[test]
    fn production_copy_is_visible() {
        let buf = render_to_buffer(100, 30);
        let full = buffer_text(&buf, 100, 30);
        assert!(
            full.contains("FARSPACE")
                || full.contains("F A R S P A C E")
                || full.contains("███████")
        );
        assert!(full.contains("CHART • EXPAND • ENDURE"));
    }

    #[test]
    fn compact_fallback_contains_core_menu() {
        let buf = render_to_buffer(60, 18);
        let full = buffer_text(&buf, 60, 18);
        assert!(full.contains("FARSPACE"));
        assert!(full.contains("New Game"));
        assert!(full.contains("Quit"));
    }

    #[test]
    fn prototype_copy_is_removed() {
        let buf = render_to_buffer(100, 30);
        let full = buffer_text(&buf, 100, 30);
        assert!(!full.contains("Prototype"));
        assert!(!full.contains("TELEMETRY"));
        assert!(!full.contains("PREVIEW"));
        assert!(!full.contains("Update"));
    }

    #[test]
    fn selected_menu_item_is_rendered() {
        let app_state = AppState {
            menu_cursor: 3,
            ..AppState::default()
        };
        let backend = TestBackend::new(100, 30);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| render_menu(frame, frame.area(), &app_state))
            .unwrap();
        let full = buffer_text(terminal.backend().buffer(), 100, 30);
        assert!(full.contains("▶ Options"));
    }

    #[test]
    fn continue_action_is_shown() {
        let buf = render_to_buffer(100, 30);
        let full = buffer_text(&buf, 100, 30);
        assert!(full.contains("Continue"));
    }
}
