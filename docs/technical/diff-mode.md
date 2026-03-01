# Diff Mode (Split-Screen Comparison)

Split-screen comparison showing current vs previous snapshot with change highlighting.

## Overview

Diff mode allows users to compare the current repository snapshot with the previous one, highlighting changes in metrics like stars, issues, pull requests, and security alerts.

## Activation

- **Key**: Press `d` to toggle diff mode
- **Exit**: Press `Esc` or `d` again to exit

## Display

### Layout
- Split screen 50/50 (left: Current, right: Previous)
- Four panels per side: Stars, Issues, Pull Requests, Security
- Header shows "Last viewed X ago" (time since previous snapshot)

### Change Highlighting

| Metric | Positive Change | Negative Change |
|--------|----------------|-----------------|
| Stars | Green `+N` | Red `-N` |
| Issues | Red `+N` (more issues) | Green `-N` (issues closed) |
| Pull Requests | Yellow `+N` | Green `-N` |
| Security | New alerts in red | — |

### Status Bar
When in diff mode, the status bar shows:
- "Diff Mode" label
- Summary of changes (e.g., "Stars +5", "Issues -3")
- "NEW SECURITY ALERTS" warning if any new alerts detected
- Exit instruction ("Press Esc or 'd' to exit")

## Implementation Details

### Data Flow
1. When `d` is pressed, `toggle_diff_mode()` is called
2. If enabling, `try_enable_diff_mode()` checks for previous snapshot reference
3. On first render, `load_previous_snapshot()` loads from history using blocking thread scope
4. `SnapshotDiff::compute()` calculates deltas between current and previous

### Key Files

| File | Purpose |
|------|-----------|
| `src/core/cache.rs` | `load_previous_snapshot()` - loads most recent history entry |
| `src/core/models.rs` | `SnapshotDiff` struct and `compute()` method |
| `src/tui/app/diff.rs` | Diff overlay rendering, split-screen comparison |
| `src/tui/app/event_loop.rs` | Key handling for diff mode (`d` key) |
| `src/core/theme.rs` | Color definitions for change indicators |

### SnapshotDiff Structure
```rust
pub struct SnapshotDiff {
    pub previous_fetched_at: DateTime<Utc>,
    pub stars_delta: i64,
    pub issues_delta: i64,
    pub prs_delta: i64,
    pub new_security_critical: u64,
    pub new_security_high: u64,
    pub new_security_medium: u64,
    pub new_security_low: u64,
    pub contributors_delta: i64,
    pub forks_delta: i64,
    pub watchers_delta: i64,
    pub releases_delta: i64,
}
```

### Async Loading Pattern
Since the render context is synchronous, diff mode uses a blocking thread scope pattern:

```rust
let prev_result = std::thread::scope(|s| {
    s.spawn(move || {
        let rt = tokio::runtime::Runtime::new().ok()?;
        rt.block_on(async {
            let cache = Cache::new(&owner, &repo).ok()?;
            cache.load_previous_snapshot(&current_id).await.ok().flatten()
        })
    })
    .join()
    .ok()
    .flatten()
});
```

## Cache Integration

- Previous snapshots are loaded from `~/.local/share/repowatch/<owner>/<repo>/history/`
- Uses the rolling history (max 20 snapshots)
- Falls back to empty diff if no previous snapshot available

## Testing

Test strategy (from task definition):
- Create 2 snapshots with changes
- Test diff highlighting accuracy for all panels
- Verify "Last viewed X ago" header formatting
