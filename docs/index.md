# forgeStat - Documentation Index

<!-- This file is a pure index to documentation. No architecture, no code, no descriptions beyond what's needed to find docs. Update when adding new documentation. -->

## Quick Links

- [PRD](../prd.txt)
- [AI Context](./ai-context.md)
- [Current Handover](./current-handover-prompt.md)
- [Update Handover](./update-handover-prompt.md)
- [AI Development Workflow](./ai-workflow/ai-development.md)

---

## Technical Documentation

### Core

| Document | Description |
|----------|-------------|
| [Cache History](./technical/cache-history.md) | Rolling snapshot history with 20-snapshot limit and 30-day purge |
| [Data Models](./technical/data-models.md) | RepoSnapshot and all metric structs with serialization |
| [Config Management](./technical/config-management.md) | GitHub PAT handling and TOML config file storage |
| [GitHub API Client](./technical/github-api-client.md) | Octocrab-based GitHub API integration with auth support |
| [Cache Layer](./technical/cache-layer.md) | Local JSON cache with TTL and offline mode support |
| [Star History](./technical/star-history.md) | Stargazer API timestamps and sparkline generation |
| [Star Milestone Prediction](./technical/star-milestone-prediction.md) | Predict next milestone based on 30d/90d growth trends |
| [Issues Metrics](./technical/issues-metrics.md) | Open issues grouped by label, sorted by age |
| [PR Metrics](./technical/pr-metrics.md) | PR stats: open, draft, merged, merge time |
| [Contributor Metrics](./technical/contributor-metrics.md) | Top 10 contributors, new contributors last 30 days |
| [Release Metrics](./technical/release-metrics.md) | Last 5 releases, days since, average release interval |
| [Velocity Metrics](./technical/velocity-metrics.md) | Weekly opened vs closed/merged for issues and PRs (8 weeks) |
| [Security Alerts](./technical/security-alerts-metrics.md) | Dependabot vulnerability alerts by severity |
| [CI Status](./technical/ci-status-metrics.md) | GitHub Actions workflow runs and success rates |
| [Community Health](./technical/community-health-metrics.md) | Community profile metrics from GitHub API |
| [Community Health Integration](./technical/community-health-integration.md) | Community Health factoring into HealthScore and TUI display |
| [Snapshot Orchestrator](./technical/snapshot-orchestrator.md) | Cache-first parallel fetching of all metrics into RepoSnapshot |
| [Theme System](./technical/theme-system.md) | 6 built-in themes + custom themes via TOML configuration |
| [Status Bar](./technical/status-bar.md) | Configurable status bar with up to 3 customizable metrics |
| [Contextual Action Hints](./technical/contextual-action-hints.md) | Dynamic panel-specific shortcuts in the status bar |
| [Braille Sparklines](./technical/braille-sparklines.md) | Unicode Braille patterns for 2x resolution star history charts |
| [Layout System](./technical/layout-system.md) | Resizable panel layouts with presets and persistence |
| [Mini-Map](./technical/mini-map.md) | Condensed overview mode showing all 12+ metrics (m key) |
| [Zoom Mode](./technical/zoom-mode.md) | Panel detach/zoom for detailed views (Enter key) |
| [Panel Search/Filter](./technical/panel-search-filter.md) | Interactive search with / key, label filters, count indicators |
| [Panel Timeframe Controls](./technical/panel-timeframe-controls.md) | Panel-specific +/- keys for timeframes, pagination, list sizes |
| [Mouse-Resizable Panels](./technical/mouse-resizable-panels.md) | Drag panel borders to resize with persistence |
| [Fuzzy Finder](./technical/fuzzy-finder.md) | Quick repo switching with 'f' key and CLI integration |
| [Diff Mode](./technical/diff-mode.md) | Split-screen comparison with previous snapshot (d key) |
| [Compare Mode](./technical/compare-mode.md) | Side-by-side comparison of two repositories with winner highlighting |
| [Health Score](./technical/health-score.md) | Repository health assessment with 4 sub-scores (0-100) |
| [Copy-to-Clipboard](./technical/copy-to-clipboard.md) | Contextual clipboard copy with toast notifications (c key) |
| [Command Palette](./technical/command-palette.md) | Vim-style command palette with autocomplete and history (: key) |
| [Animation System](./technical/animation-system.md) | Panel flash, count-up numbers, spinner, sync pulse with low-power fallback |

### TUI

| Document | Description |
|----------|-------------|
| [TUI Module Architecture](./technical/tui-module-architecture.md) | Modular `app/` directory structure, render pipeline, visibility conventions |
| [TUI Panels & Layout](./technical/tui-panels.md) | 8-panel grid layout, keyboard navigation, sync status bar |
| [TUI Event Handling](./technical/tui-event-handling.md) | Keyboard event loop, key bindings, panel navigation, help overlay |
| [Watchlist Dashboard](./technical/watchlist-dashboard.md) | Multi-repo table view with `--watchlist` flag |

### CLI

| Document | Description |
|----------|-------------|
| [CLI Integration](./technical/cli-integration.md) | Command-line parsing, repo validation, TUI/GUI mode dispatch |
| [CLI JSON Output](./technical/cli-json-output.md) | `--json` flag for JSON export without TUI |
| [CLI Summary Output](./technical/cli-summary-output.md) | `--summary` flag for compact human-readable metrics summary |
| [Markdown Report Export](./technical/markdown-report-export.md) | `--report` flag for generating MD health reports |

---

## Guides

| Guide | Description |
|-------|-------------|
| [Testing Progress](./testing-progress.md) | How to build, run tests, and verify progress through Task 6 |
| [Complete Testing Guide](./testing-guide.md) | Comprehensive manual and automated testing procedures for all features across all phases |
