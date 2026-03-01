# Cache History System

## Overview

Implements a rolling history cache for repository snapshots, enabling diff functionality and historical comparison. The cache maintains up to 20 snapshots with automatic purging of files older than 30 days.

## Key Files

- `src/core/cache.rs` - Cache struct with history management, purge logic, and state persistence
- `src/core/snapshot.rs` - Snapshot orchestrator with history integration
- `src/core/models.rs` - RepoSnapshot with history tracking fields

## Implementation Details

### Directory Structure

The cache uses an XDG-compliant directory layout:

```
~/.local/share/repowatch/<owner>/<repo>/
├── cache.json          # Current snapshot
├── history/            # Rolling snapshot history
│   ├── <uuid-1>.json
│   ├── <uuid-2>.json
│   └── ... (max 20 files)
└── state.json          # UI scroll positions and last viewed
```

### State Persistence

The `StateEntry` struct tracks UI state:

```rust
pub struct StateEntry {
    pub scroll_positions: HashMap<String, u16>,  // Panel scroll positions
    pub last_viewed_at: Option<DateTime<Utc>>,   // Last view timestamp
}
```

### Snapshot History

Each `RepoSnapshot` now includes history tracking:

```rust
pub struct RepoSnapshot {
    pub fetched_at: DateTime<Utc>,
    pub previous_snapshot_at: Option<DateTime<Utc>>,  // Link to previous
    pub snapshot_history_id: Uuid,                     // Unique ID for this snapshot
    // ... other fields
}
```

### Rolling History Management

- **Max Snapshots**: 20 (`MAX_HISTORY_SNAPSHOTS` constant)
- **Rotation**: Oldest files deleted when limit exceeded
- **Purge**: Files older than 30 days removed on each fetch
- **Naming**: `<snapshot_history_id>.json`

### Cache Flow

1. Check cache freshness
2. If stale, fetch fresh snapshot
3. Link to previous snapshot via `previous_snapshot_at`
4. Generate new UUID for `snapshot_history_id`
5. Save to `cache.json`
6. Save to `history/<uuid>.json`
7. Purge old history files (>30 days)
8. Enforce 20-snapshot limit

## Dependencies Used

- `uuid` - UUID generation for snapshot identifiers
- `chrono` - DateTime handling for snapshot timestamps
- `serde` / `serde_json` - Serialization for cache files
- `tokio::fs` - Async filesystem operations
- `dirs` - XDG Base Directory compliance

## Usage

### Initialize Cache

```rust
let cache = Cache::new("owner", "repo")?;
cache.initialize().await?;  // Creates directory structure
```

### Save to History

```rust
let history_path = cache.save_to_history(&snapshot).await?;
```

### Purge Old Files

```rust
let deleted_count = cache.purge_history(30).await?;  // Purge >30 days
```

### Load/Save State

```rust
let state = cache.load_state().await?;
state.scroll_positions.insert("stars".to_string(), 5);
cache.save_state(&state).await?;
```

## Testing

Run cache-specific tests:

```bash
cargo test cache
```

Key test cases:
- `test_cache_initialize_creates_structure` - Directory creation
- `test_save_to_history_creates_file` - History file creation
- `test_save_to_history_rotation` - 20-snapshot limit enforcement
- `test_purge_history_deletes_old_files` - Age-based purging
- `test_state_save_and_load` - UI state persistence
