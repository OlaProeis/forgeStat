# Snapshot Orchestrator

## Overview

Central entry point for fetching a complete `RepoSnapshot`. Coordinates between the GitHub API client and local cache, fetching all 9 metrics in parallel when a fresh snapshot is needed.

## Key Files

- `src/core/snapshot.rs` - Orchestrator with cache-first logic and parallel fetching
- `src/core/github_client.rs` - Individual metric fetch methods called in parallel
- `src/core/cache.rs` - JSON cache with TTL-based staleness checks

## Implementation Details

### Cache-First Flow

1. If `force_refresh` is false and cache is not stale (< 15 min TTL), return cached snapshot.
2. Otherwise, fetch all metrics from GitHub API in parallel.
3. Save assembled snapshot to cache before returning.

### Parallel Fetching

Uses `tokio::try_join!` to run all 8 API calls concurrently:

| Call | Returns |
|------|---------|
| `repos()` | `RepoMeta` |
| `stargazers()` | `StarHistory` |
| `issues()` | `IssueStats` |
| `pull_requests()` | `PrStats` |
| `contributors()` | `ContributorStats` |
| `releases()` | `Vec<Release>` |
| `velocity()` | `VelocityStats` |
| `security_alerts()` | `Option<SecurityAlerts>` |

If any call fails, `try_join!` cancels remaining futures and propagates the error.

### Legacy

`GitHubClient::fetch_snapshot()` still exists for backward compatibility but fetches sequentially without caching. New code should use `snapshot::fetch_snapshot()`.

## Dependencies Used

- `tokio` - `try_join!` macro for parallel async execution
- `anyhow` - Error propagation
- `chrono` - Timestamp for `fetched_at`
- `log` - Cache hit/miss logging

## Usage

```rust
use forgeStat::core::snapshot::fetch_snapshot;
use forgeStat::core::github_client::GitHubClient;
use forgeStat::core::cache::Cache;

let client = GitHubClient::new(Some("ghp_token"))?;
let cache = Cache::new("owner", "repo")?;
let snapshot = fetch_snapshot(&client, &cache, "owner", "repo", false).await?;
```
