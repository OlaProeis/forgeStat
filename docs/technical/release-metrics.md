# Release Metrics

## Overview

Release cadence metrics provide insights into a repository's release frequency and consistency. The module fetches the last 5 releases and calculates days since each release and the average interval between releases.

## Key Files

- `src/core/metrics/releases.rs` - Release metrics computation and API client extension
- `src/core/metrics/mod.rs` - Module exports
- `src/core/models.rs` - `Release` struct with `days_since` and `avg_interval` fields
- `src/core/github_client.rs` - Uses `ReleasesMetrics` for fetching release data

## Implementation Details

### Release Struct

```rust
pub struct Release {
    pub tag_name: String,
    pub name: Option<String>,
    pub created_at: DateTime<Utc>,
    pub published_at: Option<DateTime<Utc>>,
    pub prerelease: bool,
    pub draft: bool,
    pub days_since: Option<i64>,         // Days since release was published
    pub avg_interval: Option<f64>,       // Average days between releases
}
```

### Metrics Computed

1. **days_since**: Number of days between the release publication date and the current time
2. **avg_interval**: Average number of days between consecutive releases (calculated only for the most recent release)

### Algorithm

- Fetch last 5 releases using GitHub API (`per_page(5u8)`)
- Calculate `days_since` for each release using `(Utc::now() - published_at).num_days()`
- Use `.windows(2)` iterator to calculate intervals between consecutive releases
- Compute average interval only when 2+ releases have valid `published_at` timestamps

## Dependencies Used

- `octocrab` - GitHub API client
- `chrono` - Date/time calculations
- `anyhow` - Error handling

## Usage

```rust
use crate::core::metrics::releases::ReleasesMetrics;

let metrics = ReleasesMetrics::new(&octocrab_client);
let releases = metrics.fetch_stats("owner", "repo").await?;

for release in releases {
    println!("{}: {} days ago", release.tag_name, release.days_since.unwrap_or(0));
    if let Some(avg) = release.avg_interval {
        println!("  Average interval: {:.1} days", avg);
    }
}
```

## Testing

Run tests with:

```bash
cargo test core::metrics::releases
```

Tests cover:
- Average interval calculation with multiple releases
- Single release (no interval calculation)
- Two releases (single interval)
- Days since calculation
