# Theme Configuration

Gromaq ships with `gromaq-dark` as the built-in default theme preset. It is a
dark, high-contrast baseline tuned for native terminal screenshots and long
daily sessions. The alternate built-in `gromaq-graphite` preset provides a
cooler, crisper graphite palette with brighter default text.

Built-in presets are guarded by automated contrast tests for foreground text,
cursor color, selection readability, and the readable ANSI color slots. ANSI
black and bright black remain intentionally subdued for terminal UI roles that
need lower emphasis.

The selected preset is the starting point for the theme. Users can keep it
as-is or override any individual field in TOML:

```toml
[theme]
preset = "gromaq-dark"
background = "#171b24"
foreground = "#edf3fb"
cursor = "#f6c177"
selection = "#33445f"
cursor_style = "block"
cursor_blinking = true
ansi = [
  "#2a2f3a", "#ff6b7a", "#8bdc8b", "#f6c177",
  "#8aadf4", "#c6a0f6", "#8bd5ca", "#cad3e3",
  "#6e7686", "#ff8fa3", "#a6e3a1", "#f9d58a",
  "#a6c8ff", "#f5bde6", "#9ee7dc", "#f7fbff",
]
surface_padding_px = 14
dim_opacity = 0.66
```

## Fields

- `preset`: named built-in theme baseline. Current values: `gromaq-dark` and
  `gromaq-graphite`.
- `background`: terminal surface color as `#RRGGBB`.
- `foreground`: default text color as `#RRGGBB`.
- `cursor`: cursor color as `#RRGGBB`.
- `selection`: selected cell background color as `#RRGGBB`.
- `cursor_style`: one of `block`, `underline`, or `bar`.
- `cursor_blinking`: whether the default cursor requests blinking.
- `ansi`: exactly sixteen normal and bright ANSI colors as `#RRGGBB`.
- `surface_padding_px`: empty space around rendered cells in physical pixels.
- `dim_opacity`: opacity multiplier for SGR dim text, from `0.1` to `1.0`.

## Presets

Use the default:

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
