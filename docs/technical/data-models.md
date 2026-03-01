# Data Models

## Overview

The data model layer defines all structures for repository snapshot data, enabling serialization for local cache and deserialization from GitHub API responses.

## Key Files

- `src/core/models.rs` - All data structures and unit tests

## Implementation Details

### Main Structure

`RepoSnapshot` serves as the root container, capturing a complete point-in-time view of repository metrics:

```rust
pub struct RepoSnapshot {
    pub fetched_at: DateTime<Utc>,
    pub repo: RepoMeta,
    pub stars: StarHistory,
    pub issues: IssueStats,
    pub pull_requests: PrStats,
    pub contributors: ContributorStats,
    pub releases: Vec<Release>,
    pub velocity: VelocityStats,
    pub security_alerts: Option<SecurityAlerts>,
}
```

### Key Design Decisions

1. **Optional Security Alerts** - `Option<SecurityAlerts>` because this data requires authentication with `security_events` scope
2. **Sparkline Data** - `StarHistory` includes 30d, 90d, and 365d vectors for trend visualization
3. **Label Grouping** - `IssueStats.by_label` uses `HashMap<String, Vec<Issue>>` for efficient categorization
4. **Time-to-Merge** - Stored in hours as `f64` for precision in `PrStats`
5. **Serde Compatibility** - All structs derive `Serialize`/`Deserialize` for JSON cache storage

### Nested Structures

- **RepoMeta** - Repository metadata (owner, name, description, counts)
- **StarHistory** - Total stars + sparkline vectors for charts
- **IssueStats/Issue** - Issues grouped by label + unlabelled collection
- **PrStats/MergedPr** - PR metrics with merge time calculation
- **ContributorStats/Contributor** - Top contributors + new contributor tracking
- **Release** - Release tag names, dates, prerelease/draft flags
- **VelocityStats/WeeklyActivity** - Weekly opened/closed metrics for bar charts
- **SecurityAlerts** - Dependabot alert counts by severity

## Dependencies Used

- `chrono` - DateTime handling with UTC timezone
- `serde` - Serialization/deserialization with derive macros
- `std::collections::HashMap` - Label-based issue grouping

## Usage

### Creating a Snapshot

```rust
use forgeStat::models::{RepoSnapshot, RepoMeta, StarHistory, IssueStats};
use chrono::Utc;

let snapshot = RepoSnapshot {
    fetched_at: Utc::now(),
    repo: RepoMeta { ... },
    stars: StarHistory { ... },
    issues: IssueStats { ... },
    // ... other fields
    security_alerts: None, // Omit if unauthenticated
};
```

### Serialization for Cache

```rust
let json = serde_json::to_string(&snapshot)?;
std::fs::write(cache_path, json)?;
```

### Deserialization from Cache

```rust
let json = std::fs::read_to_string(cache_path)?;
let snapshot: RepoSnapshot = serde_json::from_str(&json)?;
```

## Testing

Run model tests:

```bash
cargo test --lib
```

Tests cover:
- Full JSON roundtrip serialization
- All fields present in serialized output
- Nested struct serialization
- Optional field handling (None)
- HashMap serialization for labels
