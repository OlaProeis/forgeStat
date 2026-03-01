# CLI Integration and Launch Modes

## Overview

Command-line interface for Repowatch. Handles repository argument parsing and launches the TUI (terminal) mode.

## Key Files

- `src/main.rs` - CLI entry point with clap argument parsing and mode dispatch

## Implementation Details

### CLI Arguments

```rust
#[derive(Parser)]
struct Cli {
    repo: String,              // Repository in owner/repo format
}
```

### Repository Format Validation

The repo argument undergoes strict validation:

1. **Whitespace trimming** - Leading/trailing spaces are trimmed
2. **Format check** - Must contain exactly one forward slash
3. **Empty parts check** - Neither owner nor repo name can be empty

**Error Cases Handled:**

| Input | Error Message |
|-------|---------------|
| `invalid` | Missing forward slash with examples |
| `a/b/c` | Extra slashes with examples |
| `/repo` | Empty owner name |
| `owner/` | Empty repo name |
| `" owner/repo"` | Trimmed automatically |

### Launch

```rust
// TUI mode
let mut terminal = ratatui::init();
let result = loop {
    match forgeStat::tui::app::run_event_loop(&mut terminal, &mut app) {
        // ... event handling
    }
};
ratatui::restore();
```

### Initial Data Fetch

Before launching the TUI, the app attempts to fetch repository data:

1. Try live fetch from GitHub API
2. On failure: attempt cache load
3. If cache fresh (< 15 min): show LIVE status
4. If cache stale (> 15 min): show STALE status
5. If no cache: show OFFLINE status

## Dependencies Used

- `clap` - Command-line argument parsing with derive macros
- `dotenvy` - Environment variable loading from `.env` file
- `anyhow` - Error handling and context

## Usage

```bash
# Run with a repository
cargo run -- ratatui-org/ratatui

# Help and version
cargo run -- --help
cargo run -- --version

# Error cases
cargo run -- invalid        # Error: needs owner/repo format
cargo run -- a/b/c         # Error: too many slashes
```

## Error Messages

All error messages include:
- The invalid input received
- Clear explanation of the problem
- Correct usage examples

Example:
```
Error: Invalid repository format: 'invalid'

Repository must be in 'owner/repo' format with a forward slash.

Examples:
  forgeStat ratatui-org/ratatui
  forgeStat torvalds/linux
```
