# CI Status Metrics

## Overview

GitHub Actions CI/CD status tracking for repositories. Provides workflow run statistics including success rates, average duration, and recent run history.

## Key Files

- `src/core/metrics/ci.rs` - CI metrics computation and API response types
- `src/core/models.rs` - `CIStatus` and `WorkflowRun` data structures
- `src/core/github_client.rs` - `ci_status()` method for fetching CI data
- `src/core/metrics/mod.rs` - CI module exports

## Data Model

### CIStatus

```rust
pub struct CIStatus {
    pub total_runs_30d: u64,           // Total workflow runs in last 30 days
    pub success_rate: f64,           // Success percentage (0.0 - 100.0)
    pub avg_duration_seconds: u64,    // Average workflow duration
    pub recent_runs: Vec<WorkflowRun>, // Up to 10 most recent runs
}
```

### WorkflowRun

```rust
pub struct WorkflowRun {
    pub name: String,                  // Workflow name
    pub status: String,                // Current status (completed, in_progress, etc.)
    pub conclusion: Option<String>,    // Final result (success, failure, etc.)
    pub created_at: DateTime<Utc>,     // Run creation time
    pub duration_seconds: u64,         // Run duration in seconds
}
```

## Implementation Details

### API Endpoint

- **URL**: `GET /repos/{owner}/{repo}/actions/runs?per_page=20`
- **Auth**: Optional (60 req/hour unauthenticated, 5,000 with PAT)
- **Scope**: No special scope required for public repos

### Computed Aggregates

1. **Total Runs (30d)**: Count of runs from the last 30 days
2. **Success Rate**: Percentage of runs with `conclusion == "success"`
3. **Average Duration**: Mean time between `run_started_at` and `updated_at`

### Error Handling

- Returns `None` gracefully if Actions is not enabled (404/403)
- Handles missing `conclusion` for in-progress runs
- Logs API errors without blocking other metrics

## Dependencies Used

- `reqwest` - HTTP client for GitHub API
- `serde` - Deserialization of workflow run responses
- `chrono` - Date/time handling for duration calculations

## Usage

### Fetch CI Status

```rust
use forgeStat::core::github_client::GitHubClient;

let client = GitHubClient::new(None)?; // or Some("token")
let ci_status = client.ci_status("owner", "repo").await?;

if let Some(status) = ci_status {
    println!("Success rate: {:.1}%", status.success_rate);
    println!("Avg duration: {}s", status.avg_duration_seconds);
}
```

### Testing

```bash
# Run unit tests
cargo test core::metrics::ci

# Run integration tests (requires network)
cargo test --test ci_integration -- --nocapture
```

## TUI Integration

CI Status is displayed as the 8th panel in the TUI grid (row 3, rightmost):

- **Compact view**: Success rate %, last run status (✓/✗/⊘/○), runs count (30d), avg duration
- **Zoom view**: Full table of recent workflow runs with name, status, conclusion, duration, age
- **Keyboard**: Press `8` to jump directly to CI Status panel, `Enter` for zoom view

The panel gracefully shows "GitHub Actions not available" if Actions is disabled or inaccessible.
