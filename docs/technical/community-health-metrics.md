# Community Health Metrics

## Overview

The Community Health Audit feature fetches repository community profile data from GitHub's Community Profile API. It provides insights into the presence of essential community files and calculates an overall health score.

## Key Files

- `src/core/metrics/community.rs` - Community health metrics implementation
- `src/core/models.rs` - `CommunityHealth` struct definition
- `src/core/github_client.rs` - `community_health()` client method
- `src/core/snapshot.rs` - Integration into snapshot fetching
- `src/core/metrics/mod.rs` - Module exports

## Implementation Details

### Data Model

The `CommunityHealth` struct tracks presence of community files:

```rust
pub struct CommunityHealth {
    pub has_readme: bool,
    pub has_license: bool,
    pub has_contributing: bool,
    pub has_code_of_conduct: bool,
    pub has_issue_templates: bool,
    pub has_pr_template: bool,
    pub has_security_policy: bool,
    pub score: u8,  // GitHub's health percentage (0-100)
}
```

### API Integration

Uses GitHub's Community Profile API endpoint:
- Endpoint: `GET /repos/{owner}/{repo}/community/profile`
- Header: `Accept: application/vnd.github.black-panther-preview+json`
- Returns 204 for empty repos, 404 if unavailable
- Gracefully handles auth errors (401/403) by returning `None`

### Architecture

Following existing patterns in the codebase:
- `CommunityMetrics` struct wraps the HTTP client
- `fetch_stats()` method returns `Result<Option<CommunityHealth>>`
- Integrated into parallel snapshot fetching via `tokio::try_join!`
- Optional field in `RepoSnapshot` with `#[serde(default)]`

## Dependencies Used

- `reqwest` - HTTP client for API requests
- `serde` - JSON deserialization
- `anyhow` - Error handling

## Usage

The community health data is automatically fetched when retrieving a repository snapshot:

```rust
let snapshot = fetch_snapshot(&client, &cache, "owner", "repo", false).await?;
if let Some(health) = &snapshot.community_health {
    println!("Community Score: {}/100", health.score);
    println!("Has README: {}", health.has_readme);
    println!("Has LICENSE: {}", health.has_license);
}
```

## Testing

Unit tests cover:
- All fields present (100% score)
- No fields present (0% score)
- Mixed configuration
- Serde serialization roundtrip

Run tests with:
```bash
cargo test core::metrics::community
```
