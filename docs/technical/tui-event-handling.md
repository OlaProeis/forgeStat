# TUI Event Handling and Navigation

## Overview

Keyboard-driven event handling system for the Repowatch TUI. Implements navigation between 7 metric panels, data refresh, help overlay, and application quit functionality.

## Key Files

- `src/tui/app/event_loop.rs` — Event loop implementation and key binding dispatch
- `src/tui/app/mod.rs` — App struct and state management
- `src/tui/app/mouse.rs` — Mouse event handlers (click, drag, resize)
- `src/main.rs` — Handles `AppAction` results from TUI event loop

See [TUI Module Architecture](./tui-module-architecture.md) for the full module breakdown.

## Implementation Details

### Event Loop Architecture

The `run_event_loop()` function uses a polling-based approach:

```rust
pub fn run_event_loop(terminal: &mut DefaultTerminal, app: &mut App) -> Result<AppAction>
```

- Polls with 250ms timeout to keep UI responsive
- Only processes `KeyEventKind::Press` to avoid key repeat issues
- Returns `AppAction` enum for main loop to handle refresh/quit

### Key Bindings

| Key | Action | Implementation |
|-----|--------|----------------|
| `Tab` | Next panel | `next_panel()` - cycles 0-6 with wraparound |
| `Shift+Tab` | Previous panel | `prev_panel()` - cycles backwards with wraparound |
| `r` | Refresh data | Returns `AppAction::Refresh` to main |
| `q` | Quit | Returns `AppAction::Quit` to main |
| `?` | Toggle help | `toggle_help()` - shows/hides overlay |

### Panel Navigation

Panel selection tracked via `selected_panel: usize` (0-6):

- 0: Stars
- 1: Issues
- 2: Pull Requests
- 3: Contributors
- 4: Releases
- 5: Velocity
- 6: Security

Selected panel highlighted with cyan border via `panel_block()` helper.

### Help Overlay

Centered popup (50% x 50%) showing all keyboard shortcuts:
- Rendered with `Clear` widget to blank background
- Bordered block with cyan styling
- Toggled with `?` key

## Dependencies Used

- `crossterm` - Cross-platform terminal event handling
- `ratatui` - TUI framework with event loop integration

## Usage

```bash
# Launch TUI
cargo run -- ratatui-org/ratatui

# Navigation
Tab         # Next panel
Shift+Tab   # Previous panel
?           # Show/hide help
r           # Force refresh
q           # Quit
```

## Integration with Main Loop

The event loop returns `AppAction` which main.rs handles:

```rust
match forgeStat::tui::app::run_event_loop(&mut terminal, &mut app) {
    Ok(AppAction::Quit) => break Ok(()),
    Ok(AppAction::Refresh) => { /* trigger snapshot fetch */ },
    Err(e) => break Err(e),
}
```
