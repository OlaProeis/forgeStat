# Star History & Sparkline

## Overview

Fetches real stargazer timestamps from the GitHub Stargazers API and bins them into sparkline vectors for 30-day, 90-day, and 365-day periods.

## Key Files

- `src/core/metrics/stars.rs` — `StargazerEvent` type, `generate_sparkline()` algorithm
- `src/core/metrics/mod.rs` — Metrics submodule
- `src/core/github_client.rs` — `stargazers()` and `fetch_stargazer_timestamps()` methods

## Implementation Details

### API Approach

The standard GitHub Stargazers endpoint returns only user objects. By sending `Accept: application/vnd.github.star+json`, the response includes a `starred_at` timestamp per stargazer.

A dedicated `reqwest::Client` is used for this endpoint because octocrab doesn't expose per-request custom headers. The `GitHubClient` struct stores:
- `http: reqwest::Client` — for raw requests with custom headers
- `token: Option<String>` — for Authorization header

### Pagination Strategy

Stargazers are returned oldest-first. To get recent data efficiently:

1. Calculate the last page from total star count
2. Fetch pages backwards from the last page (newest first)
3. Stop when timestamps exceed the 365-day cutoff
4. Capped at 10 pages (1,000 stars) to stay within rate limits

### Sparkline Bucketing

`generate_sparkline(timestamps, period_start, bucket_count)` divides the time range into equal-width buckets:

| Period | Buckets | Bucket Width |
|--------|---------|-------------|
| 30d    | 30      | ~1 day      |
| 90d    | 13      | ~1 week     |
| 365d   | 12      | ~1 month    |

Timestamps outside the range are silently ignored.

## Dependencies Used

- `reqwest` (with `json` feature) — raw HTTP requests with custom Accept header
- `chrono` — timestamp arithmetic and bucketing
- `serde` — deserializing `StargazerEvent` from API JSON

## Testing

7 unit tests in `stars::tests` cover:
- Empty input, zero buckets
- Single star bucket placement
- Total count preservation across buckets
- Out-of-range timestamp filtering
- 90d weekly and 365d monthly aggregation
