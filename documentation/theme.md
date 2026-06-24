# Theme Configuration

Gromaq ships with `gromaq-dark` as the built-in default theme preset. It is a
dark, high-contrast baseline tuned for native terminal screenshots and long
daily sessions.

The preset is the starting point for the theme. Users can keep it as-is or
override any individual field in TOML:

```toml
[theme]
preset = "gromaq-dark"
background = "#0b0f14"
foreground = "#f2f4f8"
cursor = "#f6c177"
selection = "#26364f"
cursor_style = "block"
cursor_blinking = true
ansi = [
  "#151922", "#ff6b7a", "#7ee787", "#f6c177",
  "#82aaff", "#c792ea", "#7dcfff", "#d7dde8",
  "#6b7280", "#ff8fa3", "#a6e3a1", "#f9e2af",
  "#89b4fa", "#f5c2e7", "#94e2d5", "#ffffff",
]
surface_padding_px = 16
```

## Fields

- `preset`: named built-in theme baseline. Current value: `gromaq-dark`.
- `background`: terminal surface color as `#RRGGBB`.
- `foreground`: default text color as `#RRGGBB`.
- `cursor`: cursor color as `#RRGGBB`.
- `selection`: selected cell background color as `#RRGGBB`.
- `cursor_style`: one of `block`, `underline`, or `bar`.
- `cursor_blinking`: whether the default cursor requests blinking.
- `ansi`: exactly sixteen normal and bright ANSI colors as `#RRGGBB`.
- `surface_padding_px`: empty space around rendered cells in physical pixels.

Generate a parseable starter file with:

```bash
gromaq --config-template
```
