# Theme Configuration

Gromaq ships with `gromaq-ghostty` as the built-in default theme preset. It is
a Ghostty-inspired dark palette tuned for native terminal screenshots, long
daily sessions, calm contrast, and expressive ANSI colors. The
`gromaq-dark` preset keeps the original polished dark palette, and
`gromaq-graphite` provides a cooler, crisper graphite palette with brighter
default text.

Built-in presets are guarded by automated contrast tests for foreground text,
cursor color, selection readability, and the readable ANSI color slots. ANSI
black and bright black remain intentionally subdued for terminal UI roles that
need lower emphasis.

Default terminal text is intentionally larger than a compact emulator baseline:
34 px font size, 47 px line height, and 19 px automatic cell width. Users can
override those metrics in the `[font]` section, and the native app supports
browser-style runtime zoom with Control/Super `+`, Control/Super `-`,
Control/Super `0`, Control/Super mouse wheel, and dedicated OS/browser
`ZoomIn` or `ZoomOut` keys when the platform exposes them.
`cargo run -- --runtime-text-zoom-smoke` verifies that default metrics zoom
from 32/18/44 px font/cell/line-height to 37/21/51 px, reduce the visible
grid from 59x15 to 52x13, and reset back to the default metrics without a live
GPU window.
`cargo run -- --theme-legibility-smoke` verifies the shipped default theme from
the CLI, including default font metrics, foreground/background contrast,
selection contrast, cursor contrast, and readable ANSI color contrast gates.
`cargo run -- --theme-preview-snapshot target/gromaq-theme-preview.ppm` writes
a deterministic PPM artifact from the same native glyph-frame preparation path,
covering default text, ANSI colors, selection background, cursor geometry, and
surface padding without requiring a live GPU window. The smoke also rejects
blank or low-value snapshots by counting high-contrast text pixels and exact
selection/cursor-color pixels in the generated RGBA frame before writing the
PPM.
`cargo run -- --theme-preview-config path/to/gromaq.toml target/theme.ppm`
uses the same deterministic prepared-frame path for a user config, including
explicit theme overrides and background opacity.

The native app also shows a default welcome screen with system, terminal,
renderer, and theme stats before shell output. It is enabled by default and can
be disabled with `[welcome] enabled = false` when users prefer a blank
shell-first launch.

The selected preset is the starting point for the theme. Users can keep it
as-is or override any individual field in TOML:

```toml
[theme]
# presets: gromaq-dark, gromaq-graphite, gromaq-ghostty
preset = "gromaq-ghostty"
background = "#101216"
foreground = "#eef4fb"
cursor = "#f6c177"
selection = "#2f3b52"
background_opacity = 1.0
cursor_style = "block"
cursor_blinking = true
ansi = [
  "#242933", "#ff6b7a", "#9ece6a", "#e0af68",
  "#7aa2f7", "#bb9af7", "#7dcfff", "#c8d3e5",
  "#5f667a", "#ff8fa3", "#b9f27c", "#ffd98a",
  "#9dbdff", "#d7afff", "#9ee7ff", "#f7fbff",
]
surface_padding_px = 14
cell_spacing_px = 0
dim_opacity = 0.68
```

## Fields

- `preset`: named built-in theme baseline. Current values: `gromaq-dark`,
  `gromaq-graphite`, and `gromaq-ghostty`.
- `background`: terminal surface color as `#RRGGBB`.
- `foreground`: default text color as `#RRGGBB`.
- `cursor`: cursor color as `#RRGGBB`.
- `selection`: selected cell background color as `#RRGGBB`.
- `background_opacity`: terminal surface opacity from `0.0` to `1.0`. The
  built-in default is `1.0` for a fully opaque window.
- `cursor_style`: one of `block`, `underline`, or `bar`.
- `cursor_blinking`: whether the default cursor requests blinking.
- `ansi`: exactly sixteen normal and bright ANSI colors as `#RRGGBB`.
- `surface_padding_px`: empty space around rendered cells in physical pixels.
  The built-in default is `14` to keep text away from the window edge without
  changing terminal cell semantics.
- `cell_spacing_px`: optional visual gap between adjacent cells, from `0` to
  `32` physical pixels. The default is `0` to preserve dense terminal alignment.
- `dim_opacity`: opacity multiplier for SGR dim text, from `0.1` to `1.0`.

## Presets

Use the default:

```toml
[theme]
preset = "gromaq-ghostty"
```

Use the original polished dark palette:

```toml
[theme]
preset = "gromaq-dark"
```

Use the alternate graphite palette while overriding only the cursor:

```toml
[theme]
preset = "gromaq-graphite"
cursor = "#ffd166"
```

Generate a parseable starter file with:

```bash
gromaq --config-template
```

List built-in presets and their core theme tokens with:

```bash
gromaq --theme-list
```

Export a built-in preset as an importable `[theme]` TOML block with:

```bash
gromaq --theme-export gromaq-ghostty gromaq-theme.toml
```

Render a preview snapshot from a TOML config with:

```bash
gromaq --theme-preview-config gromaq.toml gromaq-theme.ppm
```
