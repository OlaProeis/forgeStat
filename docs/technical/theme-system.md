# Theme System

## Overview

The theme system provides customizable color schemes for the forgeStat TUI. It supports 6 built-in themes plus user-defined custom themes via TOML configuration.

## Key Files

- `src/core/theme.rs` - ThemeConfig struct, color parsing, theme registry
- `~/.config/forgeStat/theme.toml` - User theme configuration file

## Built-in Themes

| Theme | Description | Best For |
|-------|-------------|----------|
| `default` | Modern cyan/amber colors | General use |
| `monochrome` | Grayscale only | Minimalist / accessibility |
| `high-contrast` | Bold maximum visibility | Low vision / presentations |
| `solarized-dark` | Classic Solarized palette | Familiar terminal users |
| `dracula` | Dracula theme colors | Popular dark theme fans |
| `gruvbox` | Gruvbox Dark palette | Warm color preference |

## Configuration

### Theme File Location

```
~/.config/forgeStat/theme.toml
```

### Basic Usage

Select a built-in theme:

```toml
theme = "dracula"
```

### Custom Themes

Define custom colors via `[custom_themes]` sections:

```toml
theme = "mytheme"

[custom_themes.mytheme]
name = "mytheme"
border_selected = "#ff00ff"
border_unselected = "#666666"
sparkline = "#00ff00"
status_live = "#00ff00"
status_stale = "#ffff00"
status_offline = "#ff0000"
text_primary = "#ffffff"
text_secondary = "#888888"
text_highlight = "#ffffff"
indicator_success = "#00ff00"
indicator_warning = "#ffff00"
indicator_error = "#ff0000"
indicator_info = "#00ffff"
indicator_muted = "#666666"
severity_critical = "#ff0000"
severity_high = "#ff6666"
severity_medium = "#ffff00"
severity_low = "#666666"
help_border = "#00ffff"
help_title = "#00ffff"
header_border = "#ffffff"
```

### Color Reference

| Field | UI Element | Default |
|-------|-----------|---------|
| `border_selected` | Selected panel border | `#22d3ee` (cyan) |
| `border_unselected` | Unselected panel border | `#6b7280` (gray) |
| `header_border` | Header block border | `#ffffff` (white) |
| `sparkline` | Star history sparkline | `#fbbf24` (amber) |
| `status_live` | LIVE indicator | `#4ade80` (green) |
| `status_stale` | STALE indicator | `#facc15` (yellow) |
| `status_offline` | OFFLINE indicator | `#f87171` (red) |
| `text_primary` | Primary text | `#ffffff` (white) |
| `text_secondary` | Muted text | `#9ca3af` (gray) |
| `text_highlight` | Bold/important text | `#ffffff` (white) |
| `indicator_success` | Success/open counts | `#4ade80` (green) |
| `indicator_warning` | Warnings/drafts | `#facc15` (yellow) |
| `indicator_error` | Errors/closed counts | `#f87171` (red) |
| `indicator_info` | Info/ready state | `#22d3ee` (cyan) |
| `indicator_muted` | Muted indicators | `#6b7280` (gray) |
| `severity_critical` | Critical alerts | `#ef4444` (red) |
| `severity_high` | High severity | `#f87171` (light red) |
| `severity_medium` | Medium severity | `#facc15` (yellow) |
| `severity_low` | Low severity | `#6b7280` (gray) |
| `help_border` | Help overlay border | `#22d3ee` (cyan) |
| `help_title` | Help overlay title | `#22d3ee` (cyan) |

All colors use hex format: `#RRGGBB`

## Programmatic API

### Loading Themes

```rust
use forgeStat::core::theme;

// Load from config file (or default if missing)
let theme = theme::load_theme();
```

### Accessing Colors

```rust
// Get ratatui Color directly
let border = theme.border_selected_color();
let sparkline = theme.sparkline_color();
```

### Built-in Theme Access

```rust
use forgeStat::core::theme::ThemeConfig;

let dracula = ThemeConfig::get_builtin("dracula");
```

### Managing Themes

```rust
// Set active theme in config file
theme::set_active_theme("dracula")?;

// Save custom theme
theme::save_custom_theme("mytheme", &theme_config)?;

// List available themes
let (builtin, custom) = theme::list_available_themes()?;
```

## Fallback Behavior

If the theme file is missing, invalid, or references a non-existent theme name, the system automatically falls back to the `default` theme.

## Runtime Theme Switching (Future)

The Command Palette (Task 13) will add `:theme <name>` for runtime theme switching without restarting.
