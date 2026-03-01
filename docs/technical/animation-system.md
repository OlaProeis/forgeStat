# Animation System

The animation system provides visual polish for the TUI with configurable animations that enhance the user experience while remaining lightweight and optional.

## Overview

Animations are controlled via `AnimationConfig` stored in `~/.config/forgeStat/animation.toml`. All animations are enabled by default but can be individually disabled or globally turned off.

## Features

### 1. Panel Highlight Flash

When new data is received, the currently selected panel briefly flashes with a bright border effect to draw attention.

- **Duration**: 300ms (configurable via `flash_duration_ms`)
- **Trigger**: New snapshot received via `set_snapshot()`
- **Disable**: Set `enabled = false` or `low_power_mode = true`

### 2. Count-Up Numbers

Numeric metrics animate by counting up from zero to their actual value when new data arrives.

- **Duration**: 800ms (configurable via `count_up_duration_ms`)
- **Applies to**: Star count, issue count, PR count, contributor count, release count
- **Disable**: Set `enabled = false` or `low_power_mode = true`

### 3. Live Indicator Spinner

A rotating Braille spinner (⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏) appears next to the "LIVE" status when data is being fetched.

- **Speed**: 80ms per frame (~12fps)
- **Location**: Status bar sync state indicator
- **Disable**: Set `spinner_enabled = false`

### 4. Sync Pulse

A visual pulse effect that triggers when transitioning to LIVE state from another state.

- **Duration**: 500ms
- **Effect**: Subtle intensity variation
- **Disable**: Set `sync_pulse_enabled = false`

## Configuration

Create or edit `~/.config/forgeStat/animation.toml`:

```toml
# Master switch for all animations
enabled = true

# Low-power mode for basic terminals (disables advanced effects)
low_power_mode = false

# Panel flash duration in milliseconds
flash_duration_ms = 300

# Count-up animation duration in milliseconds
count_up_duration_ms = 800

# Individual animation toggles
sync_pulse_enabled = true
spinner_enabled = true
sparkline_draw_enabled = true
```

## Implementation Details

### Widgets

The animation system includes two reusable widgets in `src/tui/widgets/spinner.rs`:

- **BrailleSpinner**: Rotating Braille character spinner with 10 frames
- **AnimatedCounter**: Numeric counter with smooth count-up animation

### State Management

Animation state is stored in the `App` struct:

```rust
animation_config: AnimationConfig,      // Configuration
live_spinner: BrailleSpinner,           // Live indicator spinner
panel_flash: Option<(Panel, Instant)>,  // Active panel flash
animated_counters: HashMap<String, AnimatedCounter>, // Count-up state
sync_pulse_active: bool,                // Sync pulse state
```

### Event Loop Integration

The event loop adjusts its polling rate based on animation activity:

- **Active animations**: 16ms poll interval (~60fps)
- **Idle**: 250ms poll interval

The `update_animations()` method is called each frame to advance animation states and return whether a redraw is needed.

### Low-Power Terminal Fallback

When `low_power_mode` is enabled:
- Panel flash is disabled (uses static colors)
- Count-up animations show final values immediately
- Sparkline draw animation is disabled
- Only the live spinner remains (minimal CPU impact)

## Architecture

```
┌─────────────────┐
│  Event Loop     │
│  (16ms/250ms)   │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ update_animations│
│  - Advance frames│
│  - Update counters│
│  - Return redraw │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ Render Panels   │
│  - Apply flash  │
│  - Show counts  │
│  - Draw spinner │
└─────────────────┘
```

## Testing

The spinner widget includes comprehensive tests:

- Frame rotation and wrapping
- Counter stepping and completion
- Progress setting and reset behavior

Run tests: `cargo test spinner::`
