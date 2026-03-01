# Velocity Metrics

## Overview

Weekly velocity statistics for issues and PRs over the last 8 weeks. Tracks opened vs closed/merged counts per week, enabling trend analysis of repository activity.

## Key Files

- `src/core/metrics/velocity.rs` - Velocity metrics computation and API client extension
- `src/core/metrics/mod.rs` - Module exports
- `src/core/models.rs` - `VelocityStats` and `WeeklyActivity` structs
- `src/core/github_client.rs` - Uses `VelocityMetrics` for fetching velocity data

## Implementation Details

### Data Structures

```rust
pub struct VelocityStats {
    pub issues_weekly: Vec<WeeklyActivity>,  // Issues opened vs closed per week
    pub prs_weekly: Vec<WeeklyActivity>,     // PRs opened vs merged per week
}

pub struct WeeklyActivity {
    pub week_start: DateTime<Utc>,  // Monday 00:00:00 UTC
    pub opened: u64,
    pub closed: u64,
}
```

### Week Binning

- 8 weekly buckets, each starting on Monday at 00:00:00 UTC
- `compute_week_starts(8)` generates boundaries in chronological order (oldest first)
- `find_week_index()` maps a timestamp to its week bucket using reverse linear scan

### Issue Velocity

- Fetches all issues (state=all) via `issues().list()` with `per_page(100)`
- Filters out PRs returned by GitHub's issues API (`issue.pull_request.is_some()`)
- `created_at` within range increments `opened`; `closed_at` within range increments `closed`

### PR Velocity

- Fetches open and closed PRs separately via `pulls().list()` with `per_page(100)`
- `created_at` from both sets increments `opened`
- `merged_at` (not `closed_at`) from closed PRs increments `closed`, per the "opened vs merged" semantics

## Dependencies Used

- `octocrab` - GitHub API client
- `chrono` - Date/time calculations and week boundary computation
- `anyhow` - Error handling

## Usage

```rust
use crate::core::metrics::velocity::VelocityMetrics;

let metrics = VelocityMetrics::new(&octocrab_client);
let velocity = metrics.fetch_stats("owner", "repo").await?;

for week in &velocity.issues_weekly {
    println!("{}: +{} opened, -{} closed", week.week_start, week.opened, week.closed);
}
```

## Testing

Run tests with:

```bash
cargo test core::metrics::velocity
```

Tests cover:
- 8 weeks of Monday boundaries at midnight UTC
- Chronological ordering and 7-day intervals
- Week index lookup: in-range, boundaries, before-range
- Initialized weekly entries are zeroed
- Total span equals 7 weeks (8 starts)
