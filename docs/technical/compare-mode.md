# Compare Mode

## Overview

Compare Mode provides a side-by-side TUI comparison of two repositories, enabling direct visual comparison of metrics with winner highlighting and delta calculations.

## Key Files

- `src/tui/app/compare.rs` - Compare mode overlay implementation with split-screen rendering
- `src/main.rs` - CLI argument parsing and compare mode dispatch
- `src/tui/app/mod.rs` - Compare mode state management (App struct extensions)
- `src/tui/app/event_loop.rs` - Keyboard event handling for compare mode

## Implementation Details

### Architecture

The compare mode is implemented as an overlay that takes over the entire screen, similar to diff mode and mini-map. It displays two repositories side-by-side with a 50/50 horizontal split.

### State Management

The App struct maintains compare-specific state:

```rust
compare_mode: bool,                    // Is compare mode active
compare_snapshot: Option<RepoSnapshot>, // Second repository data
compare_health_score: Option<HealthScore>,
compare_focus: CompareFocus,           // Which side is focused (Left/Right)
```

### Rendering

Each side of the comparison displays:

1. **Health Score** (prominent, 5 lines) - Shows overall score and sub-scores with winner indicator
2. **Stars** (7 lines) - Total count with delta vs other repo, sparkline chart
3. **Issues** (5 lines) - Open count, by-label count, unlabelled count
4. **Pull Requests** (5 lines) - Open, draft, ready, merged (30d) counts
5. **Contributors** (4 lines) - Total unique, new (30d) counts
6. **Releases** (4 lines) - Count and latest release info
7. **Security** (fill) - Alert counts by severity

### Winner Highlighting

Colors indicate relative performance:
- **Green** - Metric is better than the other repo (higher for stars/contributors/releases, lower for issues/PRs/security alerts)
- **Red** - Metric is worse than the other repo
- **Neutral** - Equal or no comparison data

Delta calculations shown as `+N` or `-N` vs other repository.

### Keyboard Navigation

| Key | Action |
|-----|--------|
| Tab | Switch focus between left and right panels |
| q / Esc | Exit compare mode |
| r | Refresh both repositories |

## Dependencies Used

- `ratatui` - Layout, widgets, rendering
- `tokio` - Parallel fetching of both repositories
- `crate::core::health` - Health score computation
- `crate::tui::widgets::BrailleSparkline` - Star history visualization

## Usage

### CLI

```bash
# Compare two repositories
forgeStat owner/repo1 --compare owner/repo2

# Example
forgeStat ratatui-org/ratatui --compare tikv/tikv
```

### Navigation

Once in compare mode:
- Use **Tab** to switch focus between repositories
- Press **q** or **Esc** to exit compare mode
- Press **r** to refresh both repositories simultaneously

The status bar at the bottom shows the current focus and overall winner based on health score.
