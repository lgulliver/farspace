# FARSPACE — Splash Screen UX Standard

This document defines the visual design, layout system, and interaction model for
the FARSPACE splash / main menu screen. It is the canonical reference for the project's
terminal UX identity.

---

## Design Intent

The splash screen should feel like:

- a premium strategy game launcher
- a futuristic command terminal
- calm and cinematic
- spacious but intentional
- readable from across the room

Not:

- a dashboard prototype
- a telemetry panel
- a sci-fi UI experiment
- a collection of widgets

---

## Composition Model

```
┌──────────────────────────────────────────────────────────────┐
│                                                              │
│                        FARSPACE                              │
│                                                              │
│                  CHART • EXPAND • ENDURE                     │
│                                                              │
│                                                              │
│                      ▶ New Game                              │
│                        Load Game                             │
│                        Options                               │
│                        Quit                                  │
│                                                              │
│                                                              │
│ ▲ Update available: v0.2.0  [U] Download          (notice)  │
├──────────────────────────────────────────────────────────────┤
│ Enter Select  ↑↓ Move  ? Help  Esc Quit         v0.1.0-α   │
└──────────────────────────────────────────────────────────────┘
```

Key zones (top → bottom):

| Zone | Content | Weight |
|---|---|---|
| Upper space | Background starfield bleeds through | `Fill(1)` |
| Title block | FARSPACE gradient title | Fixed height (1–5 rows by terminal width) |
| Tagline | `CHART • EXPAND • ENDURE` | 1–2 rows |
| Gap | Breathing room | `Fill(1)` |
| Menu | 4 items with cursor indicator | 4–6 rows |
| Lower space | Background | `Fill(1)` |
| Update notice | Optional 1-row banner | 1 row (above footer) |
| Footer | Keybind hints + version | 2 rows (separator + bar) |

---

## Title Tiers

The title renderer selects the appropriate title style based on terminal width:

| Tier | Min width | Style |
|---|---|---|
| `WIDE` | ≥ 80 | Large ASCII art (`FARSPACE` in stylized block letters) |
| `MEDIUM` | ≥ 50 | Spaced caps: `F A R S P A C E` |
| `COMPACT` | < 50 | Plain: `FARSPACE` |

Gradient is rendered left→right across the title using `SplashPalette`:

- Left: deep blue (`#0e1a2b`)
- Mid: steel blue (`#4a9aba`)
- Right: cyan accent (`#5de0e0`)

---

## Background Atmosphere

The background is a generated starfield rendered once per frame over the entire terminal area (`render_menu_starfield`). It uses the tick counter for a subtle, non-distracting drift.

Rules:

- Stars are sparse (approximately 1 in 2 600 cells on average)
- Deep navy/black base (`Color::Black`)
- Subtle violet/cyan nebula haze in large terminals (elliptical region, `% 2 600` threshold)
- No dense pixel rain, no obvious vertical streak repetition
- Background fills the full viewport — no header/footer bars are carved out

The frame renders:

1. Starfield (full area)
2. Nebula haze (full area, if terminal is large enough)
3. Content layout on top

---

## Palette

Defined in `Theme::splash_palette()` (`crates/game_tui/src/theme.rs`):

| Role | Color | Usage |
|---|---|---|
| `bg` | `Color::Black` | Frame background |
| `accent` | `#5de0e0` (cyan) | Selection indicator, title gradient right edge, keybinds |
| `title_left` | `#0e1a2b` (deep blue) | Title gradient left edge |
| `title_mid` | `#4a9aba` (steel blue) | Title gradient midpoint |
| `text` | `Color::White` | Menu items, tagline |
| `text_muted` | `Color::DarkGray` | Footer hints |
| `warning` | `Color::Yellow` | Update notices (available/error) |
| `menu_selected_bg` | `#0a1520` (deep teal) | Selected menu item background tint |

---

## Menu Presentation

Four items in order:

1. New Game
2. Load Game
3. Options
4. Quit

Selection state:

- Selected item prefixed with `▶` (Unicode mode) or `>` (ASCII)
- Selected item rendered in `accent` colour + bold
- Unselected items in `text` colour
- The menu block has a subtle nebula-tinted background (`menu_selected_bg`)
- No noisy borders, no flashing

Navigation: `j`/`k` or `↑`/`↓`. `Enter` activates. First letter shortcuts work for Options (`o`/`s`/`O`/`S`) and Load (`l`/`L`).

---

## Footer Strip

Two rows:

```
──────────────────────────────────────────────────────────────
 Enter Select  ↑↓ Move  ? Help  Esc Quit               v0.1.0
```

- Separator: `─` character in `text_muted` colour
- Hint text: muted (`DarkGray`)
- Version right-aligned: `v{CARGO_PKG_VERSION}[-{BUILD_TAG}]`
- Background tint: subtle `menu_selected_bg` wash

---

## Update Notice

When `update_state.is_notifiable()` is true, a 1-row notice is rendered immediately
above the footer separator:

| State | Text | Colour |
|---|---|---|
| `Available` | `▲ Update available: vX.Y.Z  [U] Download` | `warning` (amber) |
| `Downloading` | `⬇ Downloading update…` | `text_muted` |
| `Staged` | `✓ vX.Y.Z ready  [U] Apply & Restart` | `accent` (cyan) |
| `Error` | `⚠ Update check failed: <message>` | `warning` (amber) |

Pressing `[U]` opens the **Update Confirm Dialog**.

---

## Update Confirm Dialog

A centred modal overlay (`render_update_confirm` in `app.rs`):

```
╭──────────────────────────────────────────╮
│           Download Update?               │
│                                          │
│  Version v0.2.0 is available.           │
│  Download and stage for installation?   │
│                                          │
│       [Y] Yes      [N] No / Esc          │
╰──────────────────────────────────────────╯
```

Two variants:

- **Download**: `UpdateConfirmKind::Download(UpdateInfo)` — pressing Y sends the `UpdateInfo` to the download channel.
- **Apply & Restart**: `UpdateConfirmKind::ApplyAndRestart { version }` — pressing Y sets `restart_requested = true` and quits the TUI. `main.rs` then calls `update::check_and_apply_staged()` and re-execs the binary.

The overlay has highest input priority — all other keys are blocked while it is open.

---

## Overlay Pattern (General Standard)

All non-game-state dialogs (settings, dispatch, battle reports, update confirm) follow this pattern:

1. Add `show_X: bool` (or `x: Option<T>`) to `OverlayState` in `app.rs`.
2. Check it at the top of `handle_key` — return early after handling.
3. Render last in `App::render()` so the overlay appears above all screens.
4. Use `Clear` widget before the modal block to wipe the background.
5. Use `BorderType::Rounded` + `Color::Cyan` border for modals.
6. Close on `Esc`.

This pattern is used by: Settings, Galactic Dispatch, Battle Reports, Update Confirm.

---

## Resize Behaviour

| Terminal size | Mode |
|---|---|
| < 40 cols or < 12 rows | `render_compact_menu` — plain centred list, no starfield decorations |
| ≥ `MIN_SPLASH_WIDTH` × `MIN_SPLASH_HEIGHT` (40 × 12) | Full `render_dashboard` path |
| Preferred | 120 × 36 |
| Ultrawide | Margins scale via `Fill(1)` constraints — content stays centred |

All `Rect` arithmetic uses `saturating_sub` / `saturating_add` to prevent underflow panics.

---

## Implementation Reference

| Concern | Location |
|---|---|
| Full render entry | `crates/game_tui/src/screens/menu.rs::render_menu` |
| Starfield | `render_menu_starfield` |
| Layout | `build_layout` |
| Title rendering | `render_title` |
| Tagline | `build_tagline_line` |
| Menu block | `render_menu_items` |
| Footer | `render_footer` |
| Update notice | `render_update_notice` |
| Palette definition | `crates/game_tui/src/theme.rs::Theme::splash_palette` |
| Update confirm overlay | `crates/game_tui/src/app.rs::render_update_confirm` |
| Overlay state | `crates/game_tui/src/app.rs::OverlayState` |
| Restart logic | `crates/farspace/src/main.rs::restart_process` |
