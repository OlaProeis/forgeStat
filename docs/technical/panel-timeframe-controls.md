# Panel-Specific Timeframe Controls

Contextual `+`/`-` key controls for each panel to cycle through timeframes, pagination, and list sizes.

## Overview

Each panel has its own set of configurable options that can be adjusted using the `+`/`]` (increase/next) and `-`/`[` (decrease/previous) keys. The action performed depends on which panel is currently selected.

## Panel Controls

| Panel | `+`/`-` Action | Options |
|-------|-----------------|---------|
| **Stars** | Cycle timeframe | 30d → 90d → 1y → 30d |
| **Issues** | Change items per page | 10 → 15 → 25 → 50 → 10 |
| **Pull Requests** | Change items per page | 10 → 15 → 25 → 50 → 10 |
| **Contributors** | Cycle display limit | top 10 → top 25 → top 50 → top 10 |
| **Releases** | Cycle display limit | 5 → 10 → 15 → 5 |
| **Velocity** | Cycle weeks view | 4 weeks → 8 weeks → 12 weeks → 4 weeks |
| **Security** | *(no action defined)* | — |

## Title Indicators

When a panel with configurable options is selected, its title shows:
- The current setting (e.g., "8 weeks", "top 25")
- A `[+/- to change]` hint

Examples:
- `Velocity (8 weeks) [+/- to change]`
- `Contributors — 47 total (top 25 view) [+/- to change]`
- `Issues — 42 open (page 1/3) [+/- to change]`
- `Releases — Showing last 10 [+/- to change]`

When the panel is not selected, the hint is hidden but the current setting remains visible:
- `Velocity (8 weeks)`
- `Issues — 42 open (page 1/3)`

## Implementation Details

### State Fields (in `App` struct)

```rust
star_timeframe: StarTimeframe,           // Stars panel
velocity_timeframe: VelocityTimeframe,   // Velocity panel
contributors_limit: ContributorsLimit,   // Contributors panel
releases_limit: ReleasesLimit,           // Releases panel
issues_per_page: usize,                  // Issues panel pagination
prs_per_page: usize,                       // Pull Requests panel pagination
```

### Enum Types

**VelocityTimeframe**
- `Weeks4` → displays 4 weeks of history
- `Weeks8` → displays 8 weeks of history
- `Weeks12` → displays 12 weeks of history

**ContributorsLimit**
- `Top10` → shows top 10 contributors
- `Top25` → shows top 25 contributors
- `Top50` → shows top 50 contributors

**ReleasesLimit**
- `Last5` → shows last 5 releases
- `Last10` → shows last 10 releases
- `Last15` → shows last 15 releases

### Cycle Methods

Each enum type has `next()` and `prev()` methods that cycle through values:

```rust
fn cycle_velocity_timeframe_forward(&mut self) {
    self.velocity_timeframe = self.velocity_timeframe.next();
}
```

The `+`/`-` key handler routes to the appropriate cycle method based on `selected_panel`:

```rust
KeyCode::Char('+') | KeyCode::Char(']') => {
    match app.selected_panel {
        Panel::Stars => app.cycle_star_timeframe_forward(),
        Panel::Velocity => app.cycle_velocity_timeframe_forward(),
        Panel::Contributors => app.cycle_contributors_limit_forward(),
        Panel::Releases => app.cycle_releases_limit_forward(),
        Panel::Issues => app.cycle_issues_per_page_forward(),
        Panel::PullRequests => app.cycle_prs_per_page_forward(),
        Panel::Security => { /* no action */ }
    }
}
```

### Render Method Updates

Each render method:
1. Reads the current setting from state
2. Builds the title with setting indicator and `[+/- to change]` hint (when selected)
3. Uses the dynamic setting to determine how many items to display

Example from `render_velocity()`:
```rust
let timeframe_label = self.velocity_timeframe.label();
let title = if self.selected_panel == Panel::Velocity {
    format!(" Velocity ({}) [+/- to change] ", timeframe_label)
} else {
    format!(" Velocity ({}) ", timeframe_label)
};

let weeks_to_show = self.velocity_timeframe.count();
for week in vel.issues_weekly.iter().rev().take(weeks_to_show) {
    // ... render week data
}
```

## Files Changed

- `src/tui/app/mod.rs` — State fields and enum types for timeframe controls
- `src/tui/app/panels.rs` — Panel render methods using timeframe state
- `src/tui/app/event_loop.rs` — Key handlers for `+`/`-` timeframe cycling

## Default Values

| Setting | Default |
|---------|---------|
| Star timeframe | 30 days |
| Velocity timeframe | 8 weeks |
| Contributors limit | top 10 |
| Releases limit | last 5 |
| Issues per page | 15 |
| PRs per page | 15 |

## Testing

Test the feature:
1. Navigate to each panel using Tab/arrow keys or 1-7 keys
2. Press `+` or `]` to cycle forward
3. Press `-` or `[` to cycle backward
4. Verify the title indicator updates immediately
5. Verify the displayed content adjusts to the new setting
6. Switch to another panel and verify the hint disappears but the setting remains visible
