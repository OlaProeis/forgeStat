# Layout System

The LayoutConfig and PanelLayout system provides resizable panel layouts with persistence to `layout.toml`, including preset configurations for different use cases.

## Overview

The layout system controls the 7-panel TUI grid layout with configurable:
- Row heights (3 rows)
- Column widths within each row
- Three presets: `default`, `compact`, `wide`
- Runtime reset functionality

## Configuration File

Located at `~/.config/forgeStat/layout.toml` (platform-dependent):

```toml
# Select preset: "default", "compact", or "wide"
preset = "default"

# Row 1 configuration (Stars, Issues)
[row1]
width_pct = 100
height_pct = 40

[[row1_columns]]
width_pct = 35  # Stars panel
height_pct = 100

[[row1_columns]]
width_pct = 65  # Issues panel
height_pct = 100

# Row 2 configuration (PRs, Contributors, Releases)
[row2]
width_pct = 100
height_pct = 30

[[row2_columns]]
width_pct = 33  # PRs panel
height_pct = 100

[[row2_columns]]
width_pct = 33  # Contributors panel
height_pct = 100

[[row2_columns]]
width_pct = 34  # Releases panel
height_pct = 100

# Row 3 configuration (Velocity, Security)
[row3]
width_pct = 100
height_pct = 30

[[row3_columns]]
width_pct = 60  # Velocity panel
height_pct = 100

[[row3_columns]]
width_pct = 40  # Security panel
height_pct = 100
```

## Presets

### Default
- Balanced layout for general use
- Row heights: 40% / 30% / 30%
- Stars/Issues: 35% / 65%
- PRs/Contributors/Releases: 33% / 33% / 34%
- Velocity/Security: 60% / 40%

### Compact
- Smaller panels, more content visible
- Row heights: 35% / 35% / 30%
- Stars/Issues: 30% / 70% (smaller stars)
- PRs/Contributors/Releases: equal thirds
- Velocity/Security: 50% / 50%

### Wide
- Emphasizes specific panels
- Row heights: 45% / 30% / 25%
- Stars/Issues: 40% / 60% (larger stars)
- Velocity/Security: 70% / 30% (larger velocity)

## API

### Types

```rust
/// Layout preset enum
pub enum LayoutPreset {
    Default,
    Compact,
    Wide,
}

/// Individual panel configuration
pub struct PanelLayout {
    pub width_pct: u16,   // 0-100
    pub height_pct: u16,  // 0-100
}

/// Full layout configuration
pub struct LayoutConfig {
    pub preset: LayoutPreset,
    pub row1: PanelLayout,
    pub row2: PanelLayout,
    pub row3: PanelLayout,
    pub row1_columns: Vec<PanelLayout>,  // 2 items
    pub row2_columns: Vec<PanelLayout>,    // 3 items
    pub row3_columns: Vec<PanelLayout>,   // 2 items
}
```

### Methods

```rust
// Load from file or return default
let config = LayoutConfig::load();

// Save to file
config.save()?;

// Reset to a preset
config.reset_to_preset(LayoutPreset::Compact);

// Get ratatui constraints
let heights = config.row_heights();
let row1_widths = config.row1_widths();
```

## Keyboard Shortcut

| Key | Action |
|-----|--------|
| `=` | Reset layout to current preset |

## Validation

The layout system enforces:
- Minimum panel size: 10% (prevents collapse)
- Normalization: Row heights always sum to 100%
- Normalization: Column widths always sum to 100%
- Graceful degradation: Invalid configs fall back to preset defaults

## Integration

The layout configuration is loaded at startup in `main.rs`:

```rust
let layout_config = LayoutConfig::load();
let app = App::new(owner, repo, theme, statusbar_config, layout_config);
```

The TUI rendering in `app.rs` uses the configurable constraints:

```rust
fn render_content(&mut self, frame: &mut Frame, area: Rect) {
    let [row1, row2, row3] = Layout::vertical(self.layout_config.row_heights()).areas(area);
    let [stars_area, issues_area] = Layout::horizontal(self.layout_config.row1_widths()).areas(row1);
    // ...
}
```
