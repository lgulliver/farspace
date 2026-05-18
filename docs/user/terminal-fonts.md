# FARSPACE Terminal Visual Modes and Fonts

FARSPACE supports three visual modes:

- **ASCII**: strict ASCII-only glyphs for maximal compatibility.
- **Unicode**: broadly supported Unicode symbols.
- **NerdFont**: richer iconography using Nerd Font / Powerline-style glyphs.

## Recommended Nerd Fonts

Use one of these when enabling NerdFont mode:

- JetBrainsMono Nerd Font
- FiraCode Nerd Font
- MesloLGS NF
- Hack Nerd Font

## How to switch visual mode

- Main menu: press **`V`** to cycle `ASCII -> Unicode -> NerdFont`.
- Command palette: press **`:`** and run **`visual-mode`** (aliases: `mode`, `visual`).

Selected mode is persisted in:

- `$XDG_CONFIG_HOME/farspace/ui.conf`, or
- `~/.config/farspace/ui.conf`

with:

```ini
visual_mode=ascii|unicode|nerdfont
```

## Fallback behavior

- ASCII mode uses plain characters for stars, planets, fleets, lanes, warnings, borders, and status markers.
- Unicode mode uses non-private Unicode symbols and avoids NerdFont private-use icons.
- NerdFont glyphs are only emitted in NerdFont mode.

## Terminal limitations and portability notes

- FARSPACE does **not** bundle fonts.
- FARSPACE does **not** force terminal font selection.
- FARSPACE does **not** use terminal-emulator-specific runtime APIs to change fonts.
- If glyphs look wrong in your terminal or over SSH, switch to Unicode or ASCII mode.

## Practical terminal examples

- **Windows Terminal**: set profile font to a Nerd Font, then use NerdFont mode.
- **iTerm2**: set profile font to Nerd Font and enable Unicode width defaults.
- **Alacritty**: configure `font.normal.family` to Nerd Font in `alacritty.toml`.
- **WezTerm**: set `font = wezterm.font("... Nerd Font")`.
- **Ghostty**: set UI font family to Nerd Font in Ghostty config.

If any environment still renders poorly, use ASCII mode for reliable readability.
