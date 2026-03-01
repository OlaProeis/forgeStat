# Mini-Map Overview Mode

The Mini-Map Overview Mode provides a condensed 2-3 line view showing all key metrics from the 7 panels in a single overlay.

## Usage

- Press `m` to toggle the mini-map overlay
- Press `1-7` to jump directly to a panel (also closes the mini-map)
- Press `q` or `m` to close the mini-map without jumping

## Display Format

Each panel is displayed in a compact 3-line format:

```
1. ★ Stars — Total: 5.0k
   30d: 150 | 90d: 500 | 1y: 2.1k

2. Issues — Open: 42
   Labels: 8 | Unlabelled: 5 | Oldest: 45d

3. Pull Requests — Open: 12
   Draft: 2 | Ready: 10 | Merged(30d): 8 | Avg: 24.5h

4. Contributors — Total: 150
   Top: username (42 commits) | 30d: 3 new

5. Releases — Total: 25
   Latest: v2.1.0 | 5d ago | Avg interval: 14d

6. Velocity (8 weeks)
   Issues: +5/-3 | PRs: +4/-2

7. Security — Total: 3
   C: 0 | H: 1 | M: 2 | L: 0
```

## Metrics Shown

| Panel | Metrics |
|-------|---------|
| Stars | Total count, 30d/90d/1y sparkline sums |
| Issues | Open count, label count, unlabelled count, oldest issue age |
| Pull Requests | Open/draft/ready counts, merged (30d), avg merge time |
| Contributors | Total unique, top contributor, new (30d) |
| Releases | Total count, latest tag, days since, avg interval |
| Velocity | Current week issues/PRs opened vs closed |
| Security | Total alerts, critical/high/medium/low breakdown |

## Implementation

The mini-map is implemented in `src/tui/app/mini_map.rs`:

- `show_mini_map: bool` - State field in `App` struct
- `toggle_mini_map()` - Toggles the overlay visibility
- `jump_to_panel(n)` - Jumps to panel 1-7 and closes mini-map
- `render_mini_map_overlay()` - Renders the 7-row mini-map display

The overlay uses a centered rectangle (85% x 75%) with a bordered block and renders each panel in a `Constraint::Length(3)` row.

## Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `m` | Toggle mini-map |
| `1` | Jump to Stars panel |
| `2` | Jump to Issues panel |
| `3` | Jump to Pull Requests panel |
| `4` | Jump to Contributors panel |
| `5` | Jump to Releases panel |
| `6` | Jump to Velocity panel |
| `7` | Jump to Security panel |
