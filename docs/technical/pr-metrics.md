# PR Metrics

## Overview

The PR metrics module provides functionality to fetch and compute pull request statistics from GitHub. It extracts PR data including open PRs, draft vs ready counts, and recently merged PRs with merge time analysis.

## Key Files

- `src/core/metrics/prs.rs` - PR metrics computation and GitHub API integration
- `src/core/metrics/mod.rs` - Module exports
- `src/core/github_client.rs` - Uses `PrsMetrics` for `pull_requests()` method
- `src/core/models.rs` - `PrStats` and `MergedPr` data structures

## Implementation Details

### Architecture

Following the same pattern as `IssuesMetrics`, the `PrsMetrics` struct wraps an octocrab client reference:

```rust
pub struct PrsMetrics<'a> {
    client: &'a Octocrab,
}
```

### Metrics Computed

1. **Open PRs** - Count of currently open pull requests
2. **Draft vs Ready** - Separate counts for draft and ready-to-review PRs
3. **Merged (30 days)** - List of PRs merged in the last 30 days with:
   - PR number and title
   - Author
   - Created and merged timestamps
   - Time to merge (in hours)
4. **Average Merge Time** - Mean hours from creation to merge for recent PRs

### API Calls

The module makes two GitHub API calls:

1. `pulls.list().state(Open)` - Fetch open PRs for counts
2. `pulls.list().state(Closed)` - Fetch closed PRs to find recently merged ones

Both use `per_page(100)` for efficiency.

## Dependencies Used

- `octocrab` - GitHub API client
- `chrono` - Date/time calculations for 30-day window and merge time
- `anyhow` - Error handling

## Usage

```rust
use crate::core::metrics::prs::PrsMetrics;
use octocrab::Octocrab;

let client = Octocrab::builder().build()?;
let metrics = PrsMetrics::new(&client);
let pr_stats = metrics.fetch_stats("owner", "repo").await?;

println!("Open: {} ({} draft, {} ready)", 
    pr_stats.open_count,
    pr_stats.draft_count,
    pr_stats.ready_count
);

if let Some(avg_hours) = pr_stats.avg_time_to_merge_hours {
    println!("Avg merge time: {:.1} hours", avg_hours);
}
```

## Testing

Unit tests cover:
- Default `PrStats` values
- `PrStats` with merged PRs data

Run with: `cargo test core::metrics::prs`
