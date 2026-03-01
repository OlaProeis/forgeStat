# GitHub API Client

## Overview

Core GitHub API integration using `octocrab` crate with support for both authenticated (PAT) and unauthenticated access. Provides methods to fetch repository metadata, stars, issues, pull requests, contributors, and releases.

## Key Files

- `src/core/github_client.rs` - `GitHubClient` struct and all API methods
- `src/core/mod.rs` - Module exports

## Implementation Details

### Authentication

- Unauthenticated: 60 req/hour, public repos only
- Authenticated (PAT via `GITHUB_TOKEN` env or config): 5,000 req/hour, private repos accessible
- Token passed to `Octocrab::builder().personal_token(token)`

### API Methods

| Method | Endpoint | Returns |
|--------|----------|---------|
| `repos(owner, repo)` | `GET /repos/{owner}/{repo}` | `RepoMeta` |
| `stargazers(owner, repo)` | From repo data | `StarHistory` (placeholder sparklines) |
| `issues(owner, repo)` | `GET /repos/{owner}/{repo}/issues` | `IssueStats` (grouped by label) |
| `pull_requests(owner, repo)` | `GET /repos/{owner}/{repo}/pulls` | `PrStats` (open/merged with merge time) |
| `contributors(owner, repo)` | `GET /repos/{owner}/{repo}/contributors` | `ContributorStats` (top 10) |
| `releases(owner, repo)` | `GET /repos/{owner}/{repo}/releases` | `Vec<Release>` |
| `fetch_snapshot(owner, repo)` | All above combined | `RepoSnapshot` |

### Type Mappings

Handles octocrab 0.43 type conversions:
- `repo.language: Option<Value>` → `Option<String>` via `as_str()`
- `issue.created_at: DateTime<Utc>` (direct, not Option) - validated with timestamp check
- `contributor.author: Author` (direct struct access, not Option)
- `author.avatar_url: Url` (direct, not Option) → converted to `String`

## Dependencies Used

- `octocrab = "0.43"` - GitHub API client
- `chrono` - Date/time handling
- `anyhow` - Error handling

## Usage

```rust
use forgeStat::core::github_client::GitHubClient;
use forgeStat::core::config::load_token;

// With authentication
let token = load_token().ok();
let client = GitHubClient::new(token.as_deref())?;

// Unauthenticated
let client = GitHubClient::new(None)?;

// Fetch single resource
let repo_meta = client.repos("rust-lang", "rust").await?;
let issues = client.issues("rust-lang", "rust").await?;

// Fetch complete snapshot
let snapshot = client.fetch_snapshot("rust-lang", "rust").await?;
```

## Testing

Unit tests verify:
- Client construction with/without token
- Placeholder sparkline generation

Integration tests would require mock server or real API calls (not included).
