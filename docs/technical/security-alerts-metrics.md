# Security Alerts Metrics

## Overview

Security alerts metrics fetch Dependabot vulnerability alerts from GitHub repositories. The feature provides counts of open security alerts categorized by severity (critical, high, medium, low).

## Key Files

- `src/core/metrics/security.rs` - Security metrics module with `SecurityMetrics` struct and `fetch_stats()` method
- `src/core/github_client.rs` - `security_alerts()` method added to `GitHubClient`
- `src/core/metrics/mod.rs` - Module declaration for security metrics

## Implementation Details

### Authentication Requirements

The GitHub Dependabot alerts API requires a personal access token (PAT) with the `security_events` scope. Without this scope, the API returns 401/403 errors. The implementation gracefully handles these cases by returning `None` instead of failing.

### API Endpoint

Uses the GitHub REST API endpoint:
```
GET /repos/{owner}/{repo}/dependabot/alerts?state=open&per_page=100
```

### Error Handling

- **401/403 with scope message** → Returns `Ok(None)` (token lacks required scope)
- **401/403 without scope message** → Returns error (other auth issues)
- **Other API errors** → Returns error with details
- **Success** → Returns `Ok(Some(SecurityAlerts))` with severity counts

### Severity Classification

Alerts are categorized by the `security_advisory.severity` field:
- `critical`
- `high`
- `medium`
- `low`
- Unknown severities are logged at debug level and ignored

## Data Model

```rust
pub struct SecurityAlerts {
    pub total_open: u64,
    pub critical_count: u64,
    pub high_count: u64,
    pub medium_count: u64,
    pub low_count: u64,
}
```

The `security_alerts` field in `RepoSnapshot` is `Option<SecurityAlerts>` to represent the case where the token lacks the required scope.

## Usage

```rust
let client = GitHubClient::new(Some("ghp_xxx"))?;
let alerts = client.security_alerts("octocat", "Hello-World").await?;

match alerts {
    Some(a) => println!("Open alerts: {} ({} critical)", a.total_open, a.critical_count),
    None => println!("Security alerts unavailable (requires security_events scope)"),
}
```

## Testing

Unit tests in `src/core/metrics/security.rs` verify:
- `SecurityAlerts` struct with default values
- Zero-count scenarios

Manual testing requires a GitHub PAT with `security_events` scope on a repository with Dependabot alerts enabled.
