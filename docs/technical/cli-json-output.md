# CLI JSON Output

## Overview

The `--json` CLI flag outputs repository metrics as formatted JSON to stdout and exits without launching the TUI. This enables scripting, data export, and integration with other tools.

## Key Files

- `src/main.rs` - CLI argument parsing and JSON output handling
- `src/core/models.rs` - `RepoSnapshot` and related structs with `Serialize` derives

## Implementation Details

### CLI Flag

```rust
#[arg(long, conflicts_with_all = ["list", "from_stdin"])]
json: bool,
```

The flag conflicts with `--list` and `--from-stdin` since all three are output modes that don't launch the TUI.

### JSON Generation

When `--json` is specified:
1. Parse repository argument (positional or stdin)
2. Fetch snapshot via `snapshot::fetch_snapshot()` (uses cache if fresh)
3. Serialize with `serde_json::to_string_pretty()`
4. Print to stdout
5. Exit with code 0 on success, code 1 on error

### Data Structure

The JSON output includes all fields from `RepoSnapshot`:

```json
{
  "fetched_at": "2026-03-01T10:30:00Z",
  "previous_snapshot_at": null,
  "snapshot_history_id": "550e8400-e29b-41d4-a716-446655440000",
  "repo": { "owner": "...", "name": "...", ... },
  "stars": { "total_count": 5000, "sparkline_30d": [...], ... },
  "issues": { "total_open": 10, "by_label": {...}, ... },
  "pull_requests": { "open_count": 5, ... },
  "contributors": { "top_contributors": [...], ... },
  "releases": [...],
  "velocity": { "issues_weekly": [...], ... },
  "security_alerts": { "total_open": 2, ... }
}
```

## Dependencies Used

- `serde` / `serde_json` - Serialization (already used for cache)
- `clap` - CLI argument parsing

## Usage

```bash
# Output JSON to stdout
forgeStat ratatui-org/ratatui --json

# Save to file
forgeStat ratatui-org/ratatui --json > repo-data.json

# Pipe to jq for processing
forgeStat ratatui-org/ratatui --json | jq '.stars.total_count'

# Error handling (non-zero exit code)
forgeStat invalid/repo --json  # Exits with code 1, error to stderr
```
