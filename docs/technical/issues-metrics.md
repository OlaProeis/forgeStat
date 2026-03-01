# Issues Metrics

## Overview

The Issues Metrics module provides functionality to fetch open issues from GitHub,
group them by label, and sort them by age (oldest first). This enables the TUI
to display issues organized by label with the oldest (and often most urgent)
issues shown first.

## Key Files

- `src/core/metrics/issues.rs` - Issues metrics computation and API client
- `src/core/metrics/mod.rs` - Module exports
- `src/core/github_client.rs` - Integration with GitHubClient

## Implementation Details

### IssuesMetrics Struct

The `IssuesMetrics` struct wraps an `Octocrab` client reference and provides
a `fetch_stats()` method to retrieve issue statistics:

```rust
pub struct IssuesMetrics<'a> {
    client: &'a Octocrab,
}

impl<'a> IssuesMetrics<'a> {
    pub fn new(client: &'a Octocrab) -> Self
    pub async fn fetch_stats(&self, owner: &str, repo: &str) -> Result<IssueStats>
}
```

### Label Grouping

Issues are grouped into two categories:

1. **By Label**: `HashMap<String, Vec<Issue>>` where each label maps to a list of issues
2. **Unlabelled**: `Vec<Issue>` for issues with no labels

An issue with multiple labels appears in each label group (cloned).

### Age Sorting

Issues within each group are sorted by `created_at` in **ascending order** (oldest first):

```rust
fn sort_vec_by_age_ascending(issues: &mut Vec<Issue>) {
    issues.sort_by(|a, b| a.created_at.cmp(&b.created_at));
}
```

This ensures the most urgent/aged issues are displayed first in the TUI.

### Integration

The `GitHubClient.issues()` method delegates to `IssuesMetrics`:

```rust
pub async fn issues(&self, owner: &str, repo: &str) -> Result<IssueStats> {
    let metrics = IssuesMetrics::new(&self.client);
    metrics.fetch_stats(owner, repo).await
}
```

## Dependencies Used

- `octocrab` - GitHub API client for fetching issues
- `chrono` - DateTime handling for created_at/updated_at
- `serde` - Serialization for data models

## Usage

### Fetching Issue Stats

```rust
use crate::core::github_client::GitHubClient;

let client = GitHubClient::new(None)?;
let stats = client.issues("octocat", "Hello-World").await?;

println!("Total open issues: {}", stats.total_open);

// Iterate by label
for (label, issues) in &stats.by_label {
    println!("Label '{}': {} issues", label, issues.len());
}

// Unlabelled issues
println!("Unlabelled: {} issues", stats.unlabelled.len());
```

### Testing

Run the issues module tests:

```bash
cargo test issues
```

Tests verify:
- Sorting by age (oldest first)
- Empty and single-issue edge cases
- Label grouping correctness
