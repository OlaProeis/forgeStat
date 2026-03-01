# TUI Module Architecture

## Overview

The TUI layer is organized as a directory module (`src/tui/app/`) with focused, single-responsibility submodules. The `App` struct orchestrates state and rendering dispatch while each submodule owns a specific domain of functionality. Rust's child module pattern provides private field access without exposing internal state to the wider crate.

## Key Files

- `src/tui/app/mod.rs` — `App` struct, enums, state management, render dispatch, status bar
- `src/tui/app/utils.rs` — Pure utility functions (formatting, resampling, layout helpers)
- `src/tui/app/panels.rs` — 7 metric panel renderers + panel block styling
- `src/tui/app/zoom.rs` — Zoom overlay with expanded panel views
- `src/tui/app/diff.rs` — Split-screen diff mode comparing snapshots
- `src/tui/app/command_palette.rs` — Vim-style command palette with autocomplete
- `src/tui/app/event_loop.rs` — Keyboard/mouse event handling loop
- `src/tui/app/mini_map.rs` — Condensed overview overlay for all metrics
- `src/tui/app/fuzzy_finder.rs` — Repository switcher with fuzzy search
- `src/tui/app/help.rs` — Help overlay with keyboard shortcut reference
- `src/tui/app/mouse.rs` — Mouse click, drag, and panel border resize handlers

## Module Structure

```
src/tui/app/
├── mod.rs              # App struct, enums, state, render(), status bar
├── utils.rs            # format_count, format_age, truncate, resample_to_width, centered_rect
├── panels.rs           # render_stars/issues/prs/contributors/releases/velocity/security
├── zoom.rs             # render_zoom_overlay + 7 zoomed panel renderers + severity_style
├── diff.rs             # render_diff_overlay + split-screen comparison renderers
├── command_palette.rs  # toggle/execute/render command palette, suggestions, history
├── event_loop.rs       # run_event_loop — polls crossterm events, dispatches to App methods
├── mini_map.rs         # render_mini_map_overlay — condensed 7-panel overview
├── fuzzy_finder.rs     # fuzzy repo search, filtering, render_fuzzy_overlay
├── help.rs             # render_help_overlay — keyboard shortcut reference
└── mouse.rs            # click detection, border drag, resize, layout persistence
```

## Design Pattern

Each submodule adds an `impl App` block with methods scoped to its domain. Because these are child modules of `app/`, they have access to `App`'s private fields without requiring `pub(crate)` visibility on struct fields.

```rust
// In app/panels.rs
use super::{App, Panel};

impl App {
    pub(super) fn render_stars(&self, frame: &mut Frame, area: Rect) {
        // Can access self.snapshot, self.theme, etc. directly
    }
}
```

**Visibility conventions:**
- `pub(super)` — methods called from `mod.rs` or `event_loop.rs` (cross-module within `app/`)
- Private — helper methods used only within their own module
- `pub(crate)` — utility functions in `utils.rs` (may be used by other `tui` modules)
- `pub` — only on types and functions that `main.rs` needs (`App`, `run_event_loop`, `AppAction`)

## Module Responsibilities

| Module | Responsibility | Key Methods |
|--------|---------------|-------------|
| `mod.rs` | App struct, enums, state transitions, render dispatch, status bar, search/filter state | `new()`, `render()`, `render_header()`, `render_content()`, `render_status_bar()` |
| `utils.rs` | Pure functions with no `App` dependency | `format_count()`, `format_age()`, `truncate()`, `resample_to_width()`, `centered_rect()` |
| `panels.rs` | 7 metric panel renderers + block styling | `render_stars()` through `render_security()`, `panel_block()` |
| `zoom.rs` | Expanded single-panel views with additional detail | `render_zoom_overlay()`, `render_zoom_stars()` through `render_zoom_security()` |
| `diff.rs` | Split-screen snapshot comparison | `render_diff_overlay()`, `render_diff_header()`, `render_diff_side()`, per-panel diff renderers |
| `command_palette.rs` | Command input, suggestion matching, execution | `toggle_command_palette()`, `execute_command()`, `render_command_palette()` |
| `event_loop.rs` | Event polling, key binding dispatch, mouse event routing | `run_event_loop()` |
| `mini_map.rs` | Condensed overview with all 7 panels summarized | `render_mini_map_overlay()` |
| `fuzzy_finder.rs` | Cached repo listing, fuzzy filter, selection | `toggle_fuzzy_mode()`, `render_fuzzy_overlay()` |
| `help.rs` | Keyboard shortcut reference overlay | `render_help_overlay()` |
| `mouse.rs` | Panel click selection, border drag-resize | `handle_mouse_click()`, `handle_drag()`, `resize_vertical_border()` |

## Render Pipeline

`mod.rs` orchestrates the render order via `App::render()`:

```
render()
├── render_header()          # mod.rs — repo name, description, language
├── render_content()         # mod.rs — layout grid, delegates to panels.rs
│   ├── render_stars()       # panels.rs
│   ├── render_issues()      # panels.rs
│   ├── render_prs()         # panels.rs
│   ├── render_contributors()# panels.rs
│   ├── render_releases()    # panels.rs
│   ├── render_velocity()    # panels.rs
│   └── render_security()    # panels.rs
├── render_status_bar()      # mod.rs — sync state, action hints
└── Overlays (rendered on top, in priority order):
    ├── render_help_overlay()        # help.rs
    ├── render_mini_map_overlay()    # mini_map.rs
    ├── render_zoom_overlay()        # zoom.rs
    ├── render_search_modal()        # mod.rs
    ├── render_fuzzy_overlay()       # fuzzy_finder.rs
    ├── render_diff_overlay()        # diff.rs
    └── render_command_palette()     # command_palette.rs
```

## Dependencies

- `ratatui` — TUI framework (Frame, Layout, widgets)
- `crossterm` — Terminal event polling (via ratatui re-export)
- `arboard` — Clipboard access (used in `mod.rs` for copy-to-clipboard)
- `chrono` — Date formatting (used in `utils.rs`)
