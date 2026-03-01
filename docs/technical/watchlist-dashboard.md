# Watchlist Dashboard

## Overview

The Watchlist Dashboard is a multi-repo TUI mode that displays multiple repositories in a table format with key metrics. It enables monitoring several projects simultaneously and quickly switching between them.

## Key Files

- `src/tui/app/watchlist.rs` - Main watchlist module with `WatchlistApp` struct and event loop
- `src/main.rs` - CLI flag handling (`--watchlist`) and `run_watchlist_loop()` function
- `src/core/config.rs` - `WatchlistConfig` for loading `watchlist.toml` config file

## Implementation Details

### Architecture

The watchlist uses a separate `WatchlistApp` struct from the main app, with its own:
- Event loop (`run_event_loop`)
- Rendering pipeline (`draw`, `draw_table`, `draw_header`, `draw_status_bar`)
- State management (`selected_index`, `snapshots`, `is_fetching`)

### Data Flow

1. CLI parses `--watchlist` flag (comma-separated repos or uses config file)
2. `run_watchlist_loop()` creates `WatchlistApp` and fetches repos in parallel
3. `futures::future::join_all()` fetches all snapshots concurrently
4. After fetch, app renders the table with all repo data
5. Event loop handles keyboard/mouse navigation

### Table Columns

| Column | Description | Data Source |
|--------|-------------|-------------|
| Repository | `owner/repo` name | Repo list from CLI or config |
| Health | Score + Grade (e.g., "87/100 (G)") | `compute_health_score()` |
| Stars | Total star count | `snapshot.stars.total_count` |
| 30d | Stars gained in 30 days | Sum of `sparkline_30d` |
| Issues | Open issues count | `snapshot.open_issues_count()` |
| PRs | Open PR count | `snapshot.open_prs_count()` |
| Release | Days since last release | `snapshot.days_since_last_release()` |
| Security | Alerts count or checkmark | `security_alerts.total_open` |

### Row Coloring

Row colors indicate health grade:
- **Green** - Excellent (90-100)
- **Cyan** - Good (75-89)
- **Yellow** - Fair/Needs Attention (50-74)
- **Red** - Critical (<50)

Selected row uses inverted colors (highlight background with black foreground).

### Keyboard Controls

| Key | Action |
|-----|--------|
| `↑/↓` or mouse scroll | Navigate repos |
| `Enter` | Switch to single-repo view for selected repo |
| `r` | Refresh all repos (re-fetch from GitHub) |
| `q` or `Esc` | Quit watchlist |
| `Home/End` | Jump to first/last repo |
| `PageUp/PageDown` | Page through list |

### Configuration File

`~/.config/forgeStat/watchlist.toml`:

```toml
repos = [
    "owner/repo1",
    "owner/repo2",
    "owner/repo3"
]
```

## Dependencies Used

- `futures` - Parallel async fetching with `join_all()`
- `ratatui` - Table widget and TUI rendering
- `serde` + `toml` - Config file parsing

## Usage

```bash
# Use watchlist.toml config file
forgeStat --watchlist

# Specify repos directly
forgeStat --watchlist owner/repo1,owner/repo2,owner/repo3

# Short flag
forgeStat -w owner/repo1,owner/repo2
```

## Testing

Run the watchlist-specific tests:

```bash
cargo test watchlist
```

Tests cover:
- Number formatting with K/M suffixes
- App initialization
- Snapshot storage and retrieval
- Health grade color mapping
