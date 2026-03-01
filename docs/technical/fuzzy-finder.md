# Fuzzy Finder

## Overview

The fuzzy finder provides quick repository switching within the TUI and external CLI integration for listing and selecting cached repositories.

## Features

- **Internal fuzzy search** (`f` key): Overlay UI for searching cached repos by name/description
- **External list** (`--list`): Output all cached repos in tab-separated format
- **Stdin input** (`--from-stdin`): Accept repo selection via pipe (e.g., `fzf`)

## Key Files

- `src/tui/app/fuzzy_finder.rs` — Fuzzy overlay UI, search filtering, selection
- `src/tui/app/event_loop.rs` — Keyboard handling for fuzzy mode (`f` key)
- `src/tui/app/mod.rs` — `AppAction::SwitchRepo` enum variant
- `src/core/cache.rs` - `CachedRepoInfo` struct, `scan_all_repos()` method
- `src/main.rs` - CLI args (`--list`, `--from-stdin`), `run_app_loop()` for repo switching
- `src/core/mod.rs` - Re-export of `CachedRepoInfo`

## Implementation Details

### Cache Scanning

`Cache::scan_all_repos()` recursively scans `~/.local/share/repowatch/<owner>/<repo>/` directories:
- Loads `cache.json` for repo metadata (name, description)
- Loads `state.json` for `last_viewed_at` timestamp
- Returns repos sorted by most recently viewed first

### TUI Fuzzy Mode

- Press `f` to open the fuzzy finder overlay
- Real-time filtering by repo name and description (substring match)
- Navigate with `↑/↓`, select with `Enter`, cancel with `Esc`
- Shows repository name, description (truncated to 40 chars), and last viewed date

### Repo Switching

When `Enter` is pressed on a selected repo:
1. `AppAction::SwitchRepo(owner, repo)` is returned from event loop
2. Main loop catches this action and restarts with new owner/repo
3. Terminal is properly restored/reinitialized between switches
4. Fresh snapshot is fetched for the new repository

### CLI Integration

```bash
# List all cached repos (tab-separated)
forgeStat --list

# Pipe with fzf for interactive selection
forgeStat --list | fzf | forgeStat --from-stdin

# Direct pipe
echo "owner/repo" | forgeStat --from-stdin
```

## Usage

### Within TUI

1. Press `f` to open fuzzy finder
2. Type to filter repositories
3. Use arrow keys to navigate
4. Press `Enter` to switch to selected repo

### With External Tools

```bash
# fzf integration
forgeStat --list | fzf --delimiter='\t' --with-nth=1,2 | cut -f1 | xargs -I {} forgeStat {}

# Or use --from-stdin
forgeStat --list | fzf | forgeStat --from-stdin
```

## Data Structure

```rust
pub struct CachedRepoInfo {
    pub owner: String,
    pub name: String,
    pub full_name: String,  // owner/name format
    pub description: Option<String>,
    pub path: PathBuf,      // Full cache directory path
    pub last_viewed_at: Option<DateTime<Utc>>,
}
```
