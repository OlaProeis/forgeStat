# Contextual Action Hints

Dynamic status bar showing panel-specific shortcuts that change based on the currently selected panel.

## Overview

The status bar displays context-aware action hints that update dynamically based on:
- Currently selected panel
- Zoom mode state
- Mini-map mode state

## Implementation

### Location

`src/tui/app/mod.rs` — `render_status_bar()` method and `get_context_hints()` helper.

### Panel Display Names

| Panel | Display Name |
|-------|--------------|
| Stars | "Stars" |
| Issues | "Issues" |
| Pull Requests | "Pull Requests" |
| Contributors | "Contributors" |
| Releases | "Releases" |
| Velocity | "Velocity" |
| Security | "Security" |

### Panel-Specific Hints

| Panel | Hints |
|-------|-------|
| **Stars** | `[+/-] timeframe`, `[r] refresh` |
| **Issues** | `[/] search`, `[l] filter`, `[s] sort`, `[r] refresh` |
| **Pull Requests** | `[r] refresh` |
| **Contributors** | `[r] refresh` |
| **Releases** | `[r] refresh` |
| **Velocity** | `[r] refresh` |
| **Security** | `[r] refresh` |

### Mode-Specific Overrides

When in special modes, panel-specific hints are replaced with mode-specific hints:

| Mode | Hints |
|------|-------|
| **Zoom** | `[Enter/Esc] exit zoom`, `[↑/↓] scroll` |
| **Mini-map** | `[m] close map`, `[1-7] jump` |

### Additional Hints

Scrollable panels (Issues, Contributors, Releases) also show `[↑/↓] scroll` hint.

All panels show `[Enter] zoom` hint when not already zoomed.

## Styling

- **Panel name**: Bold, `text_highlight_color()`
- **Key bindings**: Bold, `text_highlight_color()`
- **Descriptions**: `text_secondary_color()`
- **Global shortcuts** (`Tab/←/→:cycle  ?:help  q:quit`): `text_secondary_color()`

## Data Structures

```rust
/// Action hint for the status bar (key binding + description)
struct ActionHint<'a> {
    key: &'a str,
    description: &'a str,
}
```

## Methods

### `Panel::display_name()`

Returns the user-friendly display name for each panel variant.

### `App::get_context_hints()`

Returns a vector of `ActionHint` structs appropriate for the current context:
1. Checks for zoom mode first (highest priority)
2. Checks for mini-map mode second
3. Falls back to panel-specific hints
4. Adds scroll hint for scrollable panels
5. Adds zoom hint when not already zoomed

### `App::render_status_bar()`

Renders the status bar with:
1. Configured status bar items (SyncState, RateLimit, etc.)
2. Current panel name (bold)
3. Context-aware hints
4. Global shortcuts (Tab/←/→:cycle, ?:help, q:quit)

## Event Handling

Hints update automatically when:
- Panel selection changes (Tab, ←, → keys, mouse click)
- Zoom mode toggles (Enter key)
- Mini-map mode toggles (m key)

## Future Enhancements

The Issues panel includes hints for `[/] search`, `[l] filter`, and `[s] sort` which are placeholders for future functionality.
