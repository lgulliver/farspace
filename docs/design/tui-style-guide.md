# FARSPACE TUI Style Guide

Practical rules for keeping every screen part of one coherent product. When in
doubt, reuse a shared helper instead of writing one-off style code.

Source of cinematic intent: `ux-splash-screen.md`. This document covers the
working screens.

---

## Color philosophy

- **Use semantic `Theme` roles, never raw `Color::Rgb` in screens.** Screens
  call roles like `Theme::title_style()`, `accent_style()`, `warning_style()`,
  `faction_color()`, `bright_star()`. RGB literals live only in `theme/` and
  `components/`.
- Roles carry meaning, not just a hue: `error`/`danger` = blocking or hostile,
  `warning` = deficit or caution, `success` = surplus or friendly, `accent` =
  interactive/selected, `muted` = secondary or hidden information.
- The palette degrades through three modes (`theme/capability.rs`):
  `TrueColor → Ansi256 → Mono`. RGB-specific roles branch on
  `Theme::color_mode()`; named-ANSI roles map naturally in all modes.
- Faction identity comes from `Theme::faction_color` (foreground) and
  `faction_territory_color` (map fill). Do not invent per-screen faction hues.

## Border usage

- `page_block(title)` — the focused outer frame of a full screen.
- `panel_block(title, focused)` — a sub-panel; pass `focused` to drive the
  border between `focused_border_style` and `dim_border_style`.
- `quiet_panel_block(title)` — a non-interactive companion panel.
- All blocks use the `ROUNDED` border set. Do not hand-roll `Block::default()`
  with ad-hoc borders except for genuine error/modal surfaces (e.g. the
  unavailable-screen fallback uses an error border deliberately).
- One focused border per screen at a time. The focused panel signals where
  keyboard input goes.

## Footer rules

- Footers come from `components/footer.rs` — one arm per `Screen`.
- Keep hints **restrained: ~4–7 core entries.** Prefer the primary action,
  navigation, end-turn, `?` Help, and back/quit. Secondary commands live in the
  `?` help overlay, not the footer.
- Format is `Key Label │ Key Label …`; the component wraps cleanly on narrow
  terminals. Avoid command dumps.
- Every interactive screen exposes `?` Help in its footer.
- Do not duplicate a control in both a panel body and the footer. If body copy
  already teaches the keys, drop them from the footer.

## Selection / focus state rules

- **Selection must never be color-only.** Every selectable row carries a glyph
  marker plus a style change: list rows use `▶`/`>` prefixes, map/planet
  cursors use a distinct glyph (`•` vs `·`). The marker keeps selection legible
  in Mono mode and for color-blind users.
- Status that matters carries a symbol too, not just a hue — e.g. diplomacy
  relationships use `⚠` Hostile / `⚔` War / `●` Contacted alongside color.
- Selected rows use `Theme::highlight_style()` (bold + inverted) on top of the
  marker, never instead of it.

## Resize expectations

- Validate at `80x24`, `100x30`, `120x36`, `160x44`. `80x24` is the floor.
- Use `Constraint`-based layouts from `layout.rs` (`compose_layout`,
  `split_sidebar_main`, `split_main_detail`). Never compute fixed offsets that
  assume a width.
- At small sizes: keep the footer visible, never clip titles, never overlap
  panels. Collapse secondary sections (long descriptions, emblem columns)
  first; preserve the list + selected-detail and the functional controls.
- Wide sizes earn richer detail (sidebar + detail, emblem art), not bigger fonts
  of the same content.

## Accessibility expectations

- Important state is conveyed by **glyph/shape + text**, not color alone.
- Warnings and errors include a symbol or word, never a bare colored number.
- Muted/disabled text must stay readable (use `muted_style`, not near-bg hues).
- Monochrome fallback must remain usable: prefer named ANSI roles for text, and
  keep `SplashPalette::for_mode(Mono)` grayscale-only.

## When to use emblems

- Emblems (`components/emblem.rs`) are for **identity moments**: empire select,
  the diplomacy detail panel, setup. They answer "who is this faction?"
- Resolve via `EmpireEmblem::from_empire_index` / `resolve_empire_emblem*`.
  Render with `render_empire_emblem`, which self-sizes and drops its border at
  small areas.
- Do not scatter emblems into dense data screens (research, colony build lists)
  where they compete with information.

## When *not* to use decorative panels

- Don't wrap every value in its own bordered box. Borders cost two rows/columns
  and fragment scanning. Group related data under a `section_heading` inside one
  panel instead.
- No telemetry-style widgets or fake status meters. A meter (`meter_line`) must
  reflect real game state.
- Keep composition calm and spacious. Decoration serves hierarchy; if it does
  not help the player read the screen, leave it out.
