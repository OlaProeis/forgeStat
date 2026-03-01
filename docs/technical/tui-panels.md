# TUI Panels & Layout

## Overview

Ratatui-based terminal UI with 8 metric panels in a grid layout, status bar with sync state indicator, and keyboard navigation. The TUI is the default mode when running `forgeStat`.

## Key Files

- `src/tui/mod.rs` — TUI module root
- `src/tui/app/mod.rs` — App struct, state management, render dispatch
- `src/tui/app/panels.rs` — Individual metric panel renderers
- `src/tui/app/event_loop.rs` — Event loop and key binding dispatch
- `src/main.rs` — CLI entry point, data fetching, TUI lifecycle

See [TUI Module Architecture](./tui-module-architecture.md) for the full module breakdown.

## Layout Grid

```
┌──────────── Repowatch ─────────────┐  Header (3 lines)
│ ★ Stars (sparkline) │ Issues (table)│  Row 1 — 40%
├──────────┬──────────┬──────────────┤
│ PRs      │ Contrib  │ Releases     │  Row 2 — 30%
├──────────┼──────────┼──────────────┤
│ Velocity │ Security │ CI Status    │  Row 3 — 30%
├──────────┴──────────┴──────────────┤
│ Status bar                         │  1 line
└────────────────────────────────────┘
```

## 8 Metric Panels

| # | Panel | Widget | Data Source |
|---|-------|--------|-------------|
| 0 | Stars | `Sparkline` (30d trend) | `StarHistory.sparkline_30d` |
| 1 | Issues | `Table` (#, title, author, age) | `IssueStats.by_label` + `unlabelled` |
| 2 | Pull Requests | `Paragraph` (open/draft/ready/merged) | `PrStats` |
| 3 | Contributors | `Paragraph` (top 5 + new count) | `ContributorStats` |
| 4 | Releases | `Paragraph` (last 5 + avg interval) | `Vec<Release>` |
| 5 | Velocity | `Paragraph` (weekly opened/closed) | `VelocityStats` |
| 6 | Security | `Paragraph` (alerts by severity) | `Option<SecurityAlerts>` |
| 7 | CI Status | `Paragraph` (success %, last run, duration) | `Option<CIStatus>` |

## Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `Tab` | Next panel |
| `Shift+Tab` | Previous panel |
| `1-8` | Jump to panel 1-8 |
| `r` | Refresh data (re-fetch from GitHub) |
| `q` | Quit |
| `?` | Toggle help overlay |

## Sync States

| State | Meaning | Indicator |
|-------|---------|-----------|
| LIVE | Fresh data (< 15 min) | Green |
| STALE | Cached data > 15 min old | Yellow |
| OFFLINE | Fetch failed, using cache or no data | Red |

## Data Flow

1. `main.rs` loads token, creates `GitHubClient` and `Cache`
2. Calls `fetch_snapshot()` — cache-first, falls back to API
3. On failure, loads from cache (STALE) or shows OFFLINE
4. Initializes ratatui terminal via `ratatui::init()`
5. Runs `run_event_loop()` — polls crossterm events at 250ms intervals
6. On `r` key, exits loop → re-fetches → re-enters loop
7. On `q` key, exits loop → `ratatui::restore()` → clean exit

## Dependencies Used

- `ratatui` 0.29 — TUI rendering (includes crossterm backend)
- `chrono` — Age formatting in issue table and status bar

## Selected Panel Highlight

The currently focused panel uses a cyan bold border; unfocused panels use dark gray borders. Panel cycling wraps around (0→7→0).
