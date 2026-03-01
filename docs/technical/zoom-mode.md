# Zoom Mode (Panel Detach)

The zoom mode allows users to expand any panel to 80% of screen real estate for detailed viewing.

## Activation

- **Enter** - Toggle zoom for the currently selected panel
- **Esc** or **Enter** (when zoomed) - Exit zoom mode
- **Mouse click** (when zoomed) - Exit zoom mode

## Zoom Views

### Stars Panel (Zoomed)

Shows all three sparkline charts simultaneously:
- 30-day daily trend
- 90-day weekly trend  
- 1-year monthly trend

Plus summary with totals for each timeframe.

### Issues Panel (Zoomed)

Full table view with all columns:
- Issue number
- Title (extended width)
- Author
- Labels (comma-separated)
- Age
- Comment count

Scrolling shows more issues (dynamic based on terminal height).

### Pull Requests Panel (Zoomed)

Split view with:
- Summary stats (open, draft, ready, merged count)
- Average merge time
- Merged PRs list with merge times (scrollable)

### Contributors Panel (Zoomed)

Paginated contributor list showing:
- Rank
- Username
- Commit count
- Summary with total unique and new contributors (30d)

### Releases Panel (Zoomed)

Full table view:
- Version (tag name)
- Release name
- Published date
- Status (stable/pre-release/draft)

### Velocity Panel (Zoomed)

Side-by-side 8-week view:
- Issues (opened/closed) per week with full dates
- PRs (opened/merged) per week with full dates

### Security Panel (Zoomed)

Detailed alert breakdown:
- Total open count
- Per-severity counts (Critical, High, Medium, Low)
- Color-coded severity levels

## Implementation

```rust
// App state fields
zoom_panel: Option<Panel>,          // Which panel is zoomed (None = not zoomed)
zoom_issues_scroll: usize,          // Independent scroll for zoomed views
zoom_contributors_scroll: usize,
zoom_releases_scroll: usize,
zoom_stars_scroll: usize,
zoom_prs_scroll: usize,
```

Key methods:
- `toggle_zoom()` - Enter/exit zoom mode for selected panel
- `exit_zoom()` - Always exit zoom mode
- `is_zoomed()` - Check if currently zoomed

Zoom overlay uses `centered_rect(80, 80, frame.area())` for 80% screen coverage.
