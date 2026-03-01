# Cache Layer

## Overview

Local JSON-based cache system for storing repository snapshots with TTL-based staleness detection. Enables offline mode and reduces GitHub API calls.

## Key Files

- `src/core/cache.rs` - Cache implementation with save/load/stale detection
- `src/core/models.rs` - `RepoSnapshot` and related data structures for serialization

## Implementation Details

### Cache Location

Uses `dirs::data_local_dir()` for cross-platform paths:

| Platform | Path |
|----------|------|
| Linux | `~/.local/share/forgeStat/<owner>/<repo>/cache.json` |
| Windows | `%LOCALAPPDATA%\forgeStat\<owner>\<repo>\cache.json` |
| macOS | `~/Library/Application Support/forgeStat/<owner>/<repo>/cache.json` |

### Cache Entry Structure

```rust
struct CacheEntry {
    pub fetched_at: DateTime<Utc>,
    pub snapshot: RepoSnapshot,
}
```

Stores both the snapshot data and its fetch timestamp for staleness detection.

### TTL and Staleness

Default TTL: 15 minutes

```rust
pub fn is_stale(&self, ttl_mins: u64) -> bool
```

Returns `true` if:
- Cache file doesn't exist
- Cannot read file metadata
- File modified time is older than TTL

### API

```rust
// Create cache for a repository
let cache = Cache::new("owner", "repo")?;

// Save snapshot
cache.save(&snapshot).await?;

// Load with timestamp
if let Some((snapshot, fetched_at)) = cache.load().await? {
    // Use cached data
}

// Check staleness
if cache.is_stale(15) {
    // Fetch fresh data
}
```

## Dependencies Used

- `serde` + `serde_json` - Serialization
- `tokio::fs` - Async file operations
- `dirs` - Cross-platform data directory detection
- `chrono` - Timestamps and duration calculations
- `anyhow` - Error handling

## Usage

### Offline Mode

```rust
if cache.exists() && !cache.is_stale(ttl) {
    // Serve from cache
} else if let Some((snapshot, _)) = cache.load().await? {
    // Offline: use stale cache
}
```

### Cache Invalidation

```rust
cache.clear().await?;  // Remove cache file
```

## Tests

Run cache-specific tests:

```bash
cargo test cache::
```

Test coverage:
- Path structure verification
- Save/load roundtrip
- TTL staleness detection
- Non-existent cache handling
- Directory auto-creation
