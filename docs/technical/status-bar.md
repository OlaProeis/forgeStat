# Status Bar Configuration

## Overview

The status bar at the bottom of the TUI is fully configurable. Users can select up to 3 items from 6 available metrics to display, with sensible defaults that show sync status, API rate limit, and open PR count.

## Key Files

- `src/core/config.rs` - `StatusBarConfig` and `StatusBarItem` definitions, config file parsing
- `src/core/models.rs` - Helper methods on `RepoSnapshot` for dynamic value calculations
- `src/tui/app/mod.rs` — Status bar rendering with dynamic items
- `src/main.rs` - Loads status bar config on startup

## Configuration

### Config File Location

`~/.config/forgeStat/statusbar.toml`

### Default Configuration

```toml
items = ["sync_state", "rate_limit", "open_prs"]
```

### Available Items

| Item | Description | Example Output |
|------|-------------|----------------|
| `sync_state` | LIVE/STALE/OFFLINE with minutes since sync | `LIVE 5m` |
| `rate_limit` | GitHub API calls remaining | `API: 4987/5000` |
| `open_issues` | Count of open issues | `Issues: 42` |
| `open_prs` | Count of open PRs | `PRs: 12` |
| `last_release_age` | Days since last release | `Release: 14d` |
| `oldest_issue_age` | Age of oldest open issue | `Oldest: 90d` |

### Custom Configuration Example

```toml
items = ["sync_state", "open_issues", "oldest_issue_age"]
```

### Limits

- Maximum 3 items (enforced on save/load)
- Duplicate items are automatically deduplicated
- Invalid items are ignored

## Implementation Details

### StatusBarItem Enum

```rust
pub enum StatusBarItem {
    SyncState,
    RateLimit,
    OpenIssues,
    OpenPrs,
    LastReleaseAge,
    OldestIssueAge,
}
```

Uses `#[serde(rename_all = "snake_case")]` for TOML compatibility.

### Dynamic Value Calculation

Helper methods on `RepoSnapshot`:

- `open_issues_count()` - Returns `issues.total_open`
- `open_prs_count()` - Returns `pull_requests.open_count`
- `days_since_last_release()` - Returns `releases.first().days_since`
- `oldest_issue_age_days()` - Finds oldest issue across all labels and unlabelled

### Rendering

The `render_status_bar()` method:
1. Iterates through configured items
2. Renders each item with appropriate theme colors
3. Adds separators between items
4. Appends keyboard shortcut hint at the end

### Color Coding

- **Sync state**: `status_live_color()` (green), `status_stale_color()` (yellow), `status_offline_color()` (red)
- **Rate limit**: `indicator_error_color()` (red) if < 10 remaining, `indicator_warning_color()` (yellow) if < 10% remaining, `text_secondary_color()` otherwise
- **Metrics**: `text_primary_color()` (white) for values, `text_secondary_color()` (gray) for N/A states

## Testing

### Unit Tests

22 tests cover status bar functionality:

- All 6 StatusBarItem variants
- Config serialization/deserialization
- Max 3 item limit enforcement
- Deduplication of duplicate items
- Dynamic value calculations from RepoSnapshot

Run tests:
```bash
cargo test status_bar
```

### Manual Testing

Create a custom config and verify rendering:

```bash
# Create config with all 6 items (will be truncated to 3)
cat > ~/.config/forgeStat/statusbar.toml << 'EOF'
items = ["sync_state", "rate_limit", "open_issues", "open_prs", "last_release_age", "oldest_issue_age"]
EOF

# Run the app to see truncated config in action
cargo run -- owner/repo
```

## Dependencies

- `serde` - TOML serialization/deserialization
- `toml` - Config file parsing
- `std::collections::HashSet` - Deduplication of items
