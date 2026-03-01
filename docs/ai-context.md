# forgeStat - AI Context

<!-- This file is attached to every chat. Keep it lean, project-specific, and useful for ANY task. No roadmap, no history, no task-specific content. Max ~100 lines. -->

## Rules (DO NOT UPDATE)

- Never auto-update this file or current-handover.md — only update when explicitly requested
- Only do the task specified, do not start next task, or go over scope.
- Run `cargo build` after changes to verify code compiles
- Follow existing code patterns and conventions
- Use Context7 MCP tool to fetch library documentation when needed
- Document by feature (e.g., `cache-layer.md`), not by task
- Update `docs/index.md` when adding new documentation
- **Branch**: `master`

## Model Selection (by Task Complexity)

| Complexity | Model |
|------------|-------|
| 1–7 | Kimi 2.5k |
| 7–9 | Opus 4.6 |

Complexity comes from Task Master's analysis.

## Architecture

```
forgeStat/
├── core/              # GitHub API client, cache, data models
│   └── metrics/       # Metric computation (sparklines, aggregation)
├── tui/
│   ├── app/           # App struct + focused submodules
│   │   ├── mod.rs     # State, enums, render dispatch, status bar
│   │   ├── panels.rs  # 7 metric panel renderers
│   │   ├── zoom.rs    # Expanded panel overlays
│   │   ├── diff.rs    # Split-screen snapshot comparison
│   │   ├── command_palette.rs
│   │   ├── event_loop.rs
│   │   ├── mini_map.rs
│   │   ├── fuzzy_finder.rs
│   │   ├── help.rs
│   │   ├── mouse.rs
│   │   └── utils.rs
│   └── widgets/       # Reusable TUI widgets (braille sparklines)
└── main.rs            # CLI arg parsing and TUI launch
```

| Module | Purpose | Key Types |
|--------|---------|-----------|
| `core/` | GitHub API, caching, data models | `RepoSnapshot`, `StarHistory`, `IssueStats`, `PrStats` |
| `core/health.rs` | Repository health score computation | `HealthScore`, `HealthGrade`, `compute_health_score()` |
| `core/snapshot.rs` | Cache-first parallel fetch orchestrator | `fetch_snapshot()` |
| `core/metrics/` | Metric computation and API response types | `StargazerEvent`, `generate_sparkline()` |
| `tui/app/` | App struct + domain-specific submodules | `App`, `AppAction`, `Panel`, render pipeline |
| `tui/widgets/` | Reusable TUI components | `BrailleSparkline` |
| `main.rs` | CLI entry point (clap) | repo argument |

## Data Model

```rust
pub struct RepoSnapshot {
    pub fetched_at: DateTime<Utc>,
    pub previous_snapshot_at: Option<DateTime<Utc>>,  // For diff mode
    pub snapshot_history_id: Uuid,                      // History tracking
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

## Crate Stack

| Purpose | Crate |
|---------|-------|
| TUI rendering | `ratatui` |
| GitHub API | `octocrab` + `reqwest` + `serde_json` |
| Async runtime | `tokio` |
| Local cache (JSON) | `serde` + `serde_json` |
| Config (TOML) | `toml` + `serde` |
| Date/time | `chrono` |
| CLI arguments | `clap` |
| UUID generation | `uuid` |

## Authentication

| Mode | Rate Limit | Private Repos |
|------|------------|---------------|
| Unauthenticated | 60 req/hour | No |
| PAT (`GITHUB_TOKEN` env or `config.toml`) | 5,000 req/hour | Yes |

## Cache & Sync

- Cache at `~/.local/share/repowatch/<owner>/<repo>/`
- `cache.json` - Current snapshot (TTL: 15 min)
- `history/` - Rolling history (max 20 snapshots, purge >30 days)
- `state.json` - UI scroll positions and last viewed
- `~/.config/forgeStat/animation.toml` - Animation settings (enable/disable, low-power mode)
- Online: LIVE / STALE (> 15 min) — Offline: loads from cache

## Conventions

- **Modularity:** One feature per file. `tui/app/mod.rs` orchestrates; domain logic lives in submodules (`panels.rs`, `zoom.rs`, `diff.rs`, etc.). New TUI features get their own file in `tui/app/`.
- **Errors:** `anyhow::Result`, `?` operator, no `.unwrap()` in lib code
- **Logging:** `log::info!`, `log::error!` (not `println!`)
- **Async:** tokio runtime for all API calls
- **Serialization:** serde for all data models
- **Config:** TOML for user config, JSON for cache

## Where Things Live

| Want to... | Look in... |
|------------|------------|
| Fetch complete snapshot (with cache) | `core/snapshot.rs` |
| Add/modify GitHub API call | `core/github_client.rs` |
| Add a data model | `core/models.rs` |
| Modify cache behavior | `core/cache.rs` |
| Modify health score algorithm | `core/health.rs` |
| Add/edit a metric panel | `tui/app/panels.rs` (grid) or `tui/app/zoom.rs` (zoomed) |
| Add a TUI overlay/modal | New file in `tui/app/` with `impl App` block |
| Modify event handling/keybinds | `tui/app/event_loop.rs` |
| Add reusable TUI widget | `tui/widgets/` |
| Add CLI argument | `main.rs` |
| Add configuration option | `core/config.rs` |
| Add metric computation | `core/metrics/` |
