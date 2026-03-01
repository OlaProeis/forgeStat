# Mouse-Resizable Panels

## Overview

Mouse-resizable panels allow users to drag panel borders to resize the TUI layout interactively. Changes are persisted to `layout.toml` and persist across application restarts.

## Key Files

- `src/tui/app/mouse.rs` — Drag state tracking, border detection, resize logic, mouse event handling
- `src/tui/app/event_loop.rs` — Mouse event dispatch
- `src/main.rs` - Mouse capture enable/disable for terminal
- `src/core/config.rs` - `LayoutConfig` and `PanelLayout` structs with persistence

## Implementation Details

### Border Detection

During render, border hit areas are calculated with a 2-character grab width:
- **Vertical borders**: 4 total (Row 1: 1, Row 2: 2, Row 3: 1)
- **Horizontal borders**: 2 total (between rows 1-2 and 2-3)

### Drag State

```rust
struct DragState {
    border_index: usize,      // Which border is being dragged
    border_type: BorderType,  // Vertical (column) or Horizontal (row)
    last_mouse_pos: (u16, u16), // For calculating delta
}
```

### Resize Constraints

- **Minimum width**: 20% per panel (enforced during resize)
- **Minimum height**: 20% per panel (enforced during resize)
- **Normalization**: Percentages automatically normalized to sum to 100%

### Mouse Event Flow

1. **MouseDown on border**: `start_border_drag()` initializes drag state
2. **MouseDrag**: `handle_drag()` → `resize_vertical_border()` / `resize_horizontal_border()`
3. **MouseUp**: `end_drag()` saves layout to `layout.toml`

### Terminal Setup

Mouse capture must be enabled in `main.rs`:

```rust
use ratatui::crossterm::event::EnableMouseCapture;
use ratatui::crossterm::ExecutableCommand;

// After ratatui::init()
stdout().execute(EnableMouseCapture)?;

// Cleanup before exit
let _ = stdout().execute(DisableMouseCapture);
```

## Dependencies Used

- `ratatui::crossterm` - Mouse event capture and terminal control

## Usage

### Mouse Resize

1. Move mouse to panel border (cursor changes depending on terminal)
2. Click and drag to resize
3. Release to persist changes

### Keyboard Fallback

If mouse isn't available:
- `=` key resets layout to current preset
- Presets: Default, Compact, Wide (configured in `layout.toml`)

## Persistence

Layout is saved to `~/.config/forgeStat/layout.toml` (platform-specific):
- Windows: `%APPDATA%/forgeStat/layout.toml`
- macOS: `~/Library/Application Support/forgeStat/layout.toml`
- Linux: `~/.config/forgeStat/layout.toml`

## Terminal Compatibility

| Terminal | Mouse Support |
|----------|---------------|
| Windows Terminal | Full |
| Cursor IDE Terminal | Limited/None |
| PowerShell Console | Limited |
| iTerm2/macOS | Full |
| GNOME Terminal | Full |
