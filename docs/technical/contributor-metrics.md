# Contributor Metrics

## Overview

The contributor metrics module provides statistics about repository contributors, including top contributors by commit count, new contributors in the last 30 days, and total unique contributor count.

## Key Files

- `src/core/metrics/contributors.rs` - Contributor metrics computation and GitHub API integration
- `src/core/metrics/mod.rs` - Module exports
- `src/core/github_client.rs` - Uses `ContributorsMetrics` for fetching contributor stats
- `src/core/models.rs` - `ContributorStats` and `Contributor` data models

## Implementation Details

### Architecture

The `ContributorsMetrics` struct follows the same pattern as other metrics modules (`IssuesMetrics`, `PrsMetrics`):

```rust
pub struct ContributorsMetrics<'a> {
    client: &'a Octocrab,
}
```

### Features

1. **Top 10 Contributors**: Fetched from GitHub's contributors API (pre-sorted by contribution count)
2. **New Contributors (Last 30 Days)**: Identified by analyzing recent commits and checking if each contributor has any commits before the 30-day window
3. **Total Unique Count**: Total number of unique contributors to the repository

### New Contributor Detection Algorithm

1. Fetch all commits from the last 30 days
2. Track the earliest commit date for each contributor in this window
3. For each contributor, check if they have any commits before 30 days ago
4. If no older commits exist, they are flagged as a new contributor

## Usage

```rust
use crate::core::metrics::contributors::ContributorsMetrics;

let metrics = ContributorsMetrics::new(&octocrab_client);
let stats = metrics.fetch_stats("owner", "repo").await?;

println!("Total contributors: {}", stats.total_unique);
println!("Top contributor: {} ({} commits)", 
    stats.top_contributors[0].username,
    stats.top_contributors[0].commit_count
);
println!("New contributors: {:?}", stats.new_contributors_last_30d);
```

## Dependencies Used

- `octocrab` - GitHub API client for fetching contributors and commits
- `chrono` - Date/time handling for 30-day window calculations
- `anyhow` - Error handling

## Testing

Run tests with:

```bash
cargo test core::metrics::contributors
```

Tests cover:
- Default/empty stats
- Stats with data
- Top contributor list handling
