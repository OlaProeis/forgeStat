# DEV.to Weekend Challenge Submission - forgeStat

<!-- cover_image: ./assets/devto-cover.png -->

*This is a submission for the [DEV Weekend Challenge: Community](https://dev.to/challenges/weekend-2026-02-28)*

---

## The Community

I built **forgeStat** for the open-source community — the maintainers, contributors, and users who make GitHub the heartbeat of collaborative software development.

If you've ever maintained an open-source project, you know the struggle: you juggle GitHub's web UI, email notifications, third-party analytics tools, and browser tabs just to answer basic questions:
- Is my project growing?
- Are issues piling up faster than I can close them?
- Who's contributing and how active are they?
- Are there security vulnerabilities I missed?

This tool is for the solo maintainer burning the midnight oil, the team managing hundreds of repos, and the curious developer who wants to understand any project's health at a glance. The terminal is our natural habitat — why leave it to check GitHub?

---

## What I Built

**forgeStat** is a real-time GitHub repository dashboard that runs entirely in your terminal. It gives you a single-screen view of everything happening in a repository — stars, issues, PRs, contributors, releases, velocity metrics, and security alerts — all without leaving the command line.

### Key Features

** 8 Real-Time Metric Panels**
- **Stars**: Sparkline charts showing 30-day, 90-day, and 1-year trends with milestone predictions
- **Issues**: Open issues grouped by label, sortable by age and activity
- **Pull Requests**: Open, draft, ready, and merged counts with average merge time
- **Contributors**: Top contributors by commits + new contributors tracking
- **Releases**: Release history with publish dates and average release intervals
- **Velocity**: Weekly opened vs closed/merged metrics (4/8/12 week views)
- **Security**: Dependabot vulnerability alerts broken down by severity
- **CI Status**: GitHub Actions success rate and recent run history

** Repository Health Score (0-100)**
A comprehensive grade based on four dimensions:
- Activity (25%): Commit velocity, PR merge rate, CI success
- Community (25%): Contributor diversity, new contributors, issue engagement
- Maintenance (25%): Release cadence, security alerts, health files
- Growth (25%): Star trends, forks, watchers

** Interactive TUI Experience**
- Zoom mode for full-screen panel details (Enter)
- Mini-map for bird's-eye overview (m)
- Fuzzy finder for quick repo switching (f)
- Diff mode to compare snapshots (d)
- Command palette with Vim-style commands (:)
- Mouse support with draggable panel resizing
- 6 built-in themes + custom theme support
- ** Pong mini-game during loading** - Play while fetching large repos (>5k stars)

** CLI Output Modes**
When you don't need the full TUI:
- `--json` for complete data export
- `--summary` for compact status checks
- `--report` for markdown health reports
- `--watchlist` for multi-repo dashboards
- `--compare` for side-by-side repo comparison

** Loading Screen Experience**
Fetching data from GitHub can take time — especially for popular repos. The loading screen turns this wait into a delightful experience:
- **Twinkling starfield background** with subtle animations
- **Real-time progress tracking** — see which endpoint is being fetched and page-by-page progress for star history
- **Animated cyan border** with pulsing glow effect
- ** Pong mini-game** — automatically appears for repos with >5,000 stars. Use ↑/↓ to play against the AI while you wait!
- **Auto-refresh every 10 minutes** — keeps your data fresh while you leave the app running

For huge repos like `torvalds/linux` or `facebook/react`, you'll get a warning: "⚠ This repo has 182.2k stars — loading may take a while!" — then you can challenge the AI to a game of Pong.

---

## Demo

![forgeStat demo](https://raw.githubusercontent.com/OlaProeis/forgeStat/main/assets/demo.gif)

### Quick Demo Commands

```bash
# Interactive TUI - the full experience
forgeStat ratatui-org/ratatui

# Try a large repo to see the Pong game while loading!
forgeStat torvalds/linux

# Quick summary for CI pipelines
forgeStat facebook/react --summary

# Export data for analysis
forgeStat microsoft/vscode --json > vscode-metrics.json

# Compare two competing projects
forgeStat react --compare vue

# Multi-repo dashboard
forgeStat --watchlist torvalds/linux,rust-lang/rust
```

### The 8 Metric Panels

![forgeStat's 8 metric panels](https://raw.githubusercontent.com/OlaProeis/forgeStat/main/assets/metric-panels-overview.png)

### Repository Health Score

![Health Score System](https://raw.githubusercontent.com/OlaProeis/forgeStat/main/assets/health-score-diagram.png)

---

## Code

```rust
// A taste of the architecture - the core data model
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

// Health score computation combines 4 dimensions
pub struct HealthScore {
    pub total: u8,           // 0-100
    pub activity: u8,        // 25% weight
    pub community: u8,       // 25% weight
    pub maintenance: u8,     // 25% weight
    pub growth: u8,          // 25% weight
    pub grade: HealthGrade,  // Excellent/Good/Fair/Needs Attention/Critical
}
```

**Repository**: https://github.com/OlaProeis/forgeStat

Built with **Rust** using:
- `ratatui` for the terminal UI
- `octocrab` for GitHub API
- `tokio` for async runtime
- `serde` for serialization

---

## How I Built It

### The Stack

| Purpose | Technology |
|---------|------------|
| Language | Rust 1.74+ |
| TUI Framework | ratatui |
| GitHub API | octocrab + reqwest |
| Async Runtime | tokio |
| CLI Parsing | clap |
| Config/Cache | serde + toml + serde_json |

### Architecture

![forgeStat architecture](https://raw.githubusercontent.com/OlaProeis/forgeStat/main/assets/architecture-diagram.png)

![Data flow pipeline](https://raw.githubusercontent.com/OlaProeis/forgeStat/main/assets/data-flow-diagram.png)

### Key Technical Decisions

**Cache-First Architecture**: Data is cached locally with a 15-minute TTL. This enables:
- Instant startup when revisiting repos
- Full offline mode support
- Respect for GitHub's rate limits

**Parallel Fetching**: All 8 metrics are fetched concurrently using `tokio::join!`, making the most of the async runtime.

**Modular TUI**: Each feature gets its own file in `tui/app/` - keeps the codebase maintainable as features grow.

**Zero-Config by Default**: Works out of the box with sensible defaults. Optional GitHub token unlocks higher rate limits and private repo access.

### Challenges Faced

1. **Rate Limiting**: GitHub's 60 req/hour for unauthenticated requests required smart caching and batching strategies.

2. **TUI Layout Complexity**: 8 panels that need to work on any terminal size, with resizable borders, zoom states, and a mini-map overlay. Solved with ratatui's constraint system and careful state management.

3. **Health Score Algorithm**: Balancing four dimensions into a single meaningful score required tuning weights and defining what "healthy" means across different project types.

4. **Large Repository Loading Times**: Repos with 100k+ stars can take 1-2 minutes to fetch due to GitHub API pagination. Rather than showing a boring spinner, I built an engaging loading screen with twinkling starfield background, animated progress bars, and a playable **Pong mini-game** that appears automatically for large repos. Users can pass the time playing while their data loads!

### What I'd Do Differently

- Start with a more robust testing strategy from day one
- Consider a plugin architecture for custom metrics
- Plan for internationalization earlier (date formats, etc.)

---

## Try It Out

### Installation

**One-line install** (downloads from GitHub releases):

```bash
# Windows (PowerShell)
iwr https://github.com/OlaProeis/forgeStat/releases/latest/download/install.ps1 -UseBasicParsing | iex

# macOS / Linux
curl -fsSL https://github.com/OlaProeis/forgeStat/releases/latest/download/install.sh | bash
```

**Other methods:**

```bash
# Homebrew (macOS/Linux)
brew tap olaproeis/tap
brew install forgeStat

# Cargo (any platform with Rust installed)
cargo install forgeStat

# Or download directly from releases:
# https://github.com/OlaProeis/forgeStat/releases/latest
```

### First Run

```bash
# No setup required - just run it!
forgeStat torvalds/linux

# Add your GitHub token for more features
# Press ':' then type ':set-token' and paste your PAT
```

---

## Closing Thoughts

forgeStat started as a personal itch: I wanted to check my project's health without opening 5 browser tabs. As I built it, I realized this could help the broader open-source community — especially maintainers who don't have time to wrangle analytics dashboards.

The terminal is where developers live. Bringing GitHub insights there feels natural. I hope this tool helps others keep their projects healthy and their communities thriving.

**Star the repo** ⭐ if you find it useful, and **open an issue** if you have ideas for improvement!

---

*Thanks for reading! Built with ❤️ for the open-source community.*

<!-- Team Submissions: Please pick one member to publish the submission and credit teammates by listing their DEV usernames directly in the body of the post. -->

<!-- Thanks for participating! -->
