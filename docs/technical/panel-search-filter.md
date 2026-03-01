# Panel Search/Filter

## Overview

Interactive search and filtering functionality for the TUI panels. Users can search within Issues, Contributors, and Releases panels using the `/` key, with panel-specific filtering options. The search modal appears at the bottom of the screen with a blinking cursor indicator.

## Key Files

- `src/tui/app/mod.rs` — Search state fields, filter logic, and search modal rendering
- `src/tui/app/event_loop.rs` — Key handlers for `/` search mode

## Implementation Details

### Search State

The App struct maintains search-related state:

```rust
search_mode: bool,                    // Whether search modal is active
search_query: String,                 // Current search text
issues_label_filter: Option<String>,  // Label filter for Issues panel
releases_prerelease_filter: Option<bool>, // Prerelease filter for Releases panel
```

### Key Bindings

| Key | Panel | Action |
|-----|-------|--------|
| `/` | Issues, Contributors, Releases | Open search prompt |
| `l` | Issues | Cycle through label filters |
| `p` | Releases | Cycle prerelease filter (All → Stable → Pre-release) |
| `c` / `Esc` | All | Clear search and exit search mode |
| `Enter` | Search mode | Apply search and close modal |
| `Backspace` | Search mode | Delete last character |

### Filter Logic

**Issues Panel:**
- Title substring search (case-insensitive)
- Optional label filter (cycles through all available labels + "all")
- Combined filter shows only issues matching both criteria

**Contributors Panel:**
- Username substring search (case-insensitive)
- Filters top contributors list

**Releases Panel:**
- Tag name substring search (case-insensitive)
- Optional prerelease filter cycles through: All → Stable only → Prerelease only → All

### Count Indicators

When filters are active, panel titles show "Showing X of Y" format:
- Issues: "Showing 5 of 23" (filtered vs total open)
- Contributors: "Showing 3 of 10" (filtered vs total unique)
- Releases: "Showing 2 of 15" (filtered vs total releases)

### UI Rendering

The search modal appears at the bottom of the screen when active:
- 3-line height with bordered box
- Panel-specific prompt (e.g., "Search Issues: ")
- Blinking cursor (█) after typed text
- Clears on `c`, `Esc`, or `Enter`

### Zoom Mode Support

Search works in both normal and zoomed panel views:
- `/` key opens search from zoom mode
- Filters apply to zoomed content
- Search modal overlays zoom view

## Dependencies Used

- `ratatui` - TUI rendering (Paragraph, Block widgets)
- `crossterm` - Keyboard event handling (KeyCode)

## Usage

### Basic Search

1. Navigate to Issues, Contributors, or Releases panel
2. Press `/` to open search modal
3. Type search query (e.g., "bug" for issues with "bug" in title)
4. Press `Enter` to apply or `Esc` to cancel

### Label Filtering (Issues)

1. Select Issues panel
2. Press `l` to cycle through available labels
3. Panel shows only issues with selected label
4. Press `l` again to cycle to next label or "all"

### Combined Search + Filter

1. Select Issues panel
2. Press `l` to select label filter
3. Press `/` and type search query
4. Results match both label AND search query

### Clearing Filters

- Press `c` or `Esc` to clear all filters and exit search mode
- Panel returns to showing all items

## Testing

- Test search on each supported panel (Issues, Contributors, Releases)
- Verify count indicator shows correct "X of Y" format
- Verify label cycling works on Issues panel
- Verify prerelease cycling works on Releases panel
- Verify clear functionality (`c` key and `Esc`)
- Test search with no matches shows appropriate message
- Test search works in both normal and zoom modes
