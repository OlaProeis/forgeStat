# Braille Sparklines

## Overview

Braille sparklines provide 2x vertical resolution compared to traditional block-based sparklines by using Unicode Braille patterns (⣿⣷⣦⣄). This allows for more detailed visualization of star history trends in the TUI.

## Key Files

- `src/tui/widgets/braille_sparkline.rs` - Custom BrailleSparkline widget implementation
- `src/core/theme.rs` - ThemeConfig with `braille_mode` toggle
- `src/tui/app/panels.rs` — Stars panel rendering with Braille/classic fallback

## Implementation Details

### Braille Unicode Patterns

Each Braille character (U+2800 to U+28FF) represents 8 dots in a 2×4 grid:

```
1 4
2 5
3 6
7 8
```

The widget uses:
- Left column (dots 1,2,3,7) for the first data point
- Right column (dots 4,5,6,8) for the second data point

### 4-Level Height Encoding

| Level | Left Pattern | Right Pattern | Visual |
|-------|-------------|---------------|--------|
| 0 | Empty | Empty | ⠀ |
| 1 | Bottom dot only (3/6) | ⢀/⠠ | ⠄ |
| 2 | Lower half (2,3/5,6) | ⠒/⠢ | ⠒ |
| 3 | Upper half (1,2,3/4,5,6) | ⠇/⠷ | ⠗ |
| 4 | Full column (1,2,3,7/4,5,6,8) | ⡇/⣷ | ⡷ |

### Horizontal Resolution

Each Braille character represents 2 data points, doubling horizontal resolution:
- Same width displays 2× more data points than classic bar sparklines
- Data is resampled using linear interpolation to fit the available width

### Configuration

Theme file location:
- **Windows:** `%APPDATA%\forgeStat\theme.toml`
- **Linux:** `~/.config/forgeStat/theme.toml`
- **macOS:** `~/Library/Application Support/forgeStat/theme.toml`

Enable Braille via `theme.toml`:

```toml
# Global override (applies to all themes)
braille_mode = true

# Or set per-theme in a custom theme
theme = "my-custom"

[custom_themes.my-custom]
name = "my-custom"
braille_mode = true
sparkline = "#fbbf24"
# ... other colors
```

**To use classic sparklines again (disable Braille):** set `braille_mode = false` or remove the `braille_mode` line; the default is `false`.

## Usage

The stars panel automatically uses Braille rendering when `braille_mode` is enabled. No user interaction required—just set the config and restart the app.

## Fallback Behavior

- When `braille_mode = false` (default): Uses ratatui's classic `Sparkline` widget
- When `braille_mode = true`: Uses custom `BrailleSparkline` widget
- Non-Unicode terminals: Braille characters render as fallback characters (typically boxes or spaces)

## Testing

Run widget tests:

```bash
cargo test tui::widgets::braille_sparkline
```

Tests cover:
- Value-to-level conversion (0.0-1.0 → 0-4)
- Left/right Braille pattern generation
- Data resampling (upsample/downsample)
- Unicode character validity
