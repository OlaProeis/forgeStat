# Testing Progress

How to verify the project as tasks are completed. Updated through Task 8.

## 1. Build

```powershell
cd g:\DEV\forgeStat
cargo build
```

Expect: `Finished \`dev\` profile ...` with no errors.

## 2. Unit Tests

```powershell
cargo test
```

Expect: **36 tests** pass (as of Task 8), including:

- **Stars** (`core::metrics::stars::tests`): sparkline buckets, count preserved, 30d/90d/365d
- **Issues** (`core::metrics::issues::tests`): sort by age, label grouping
- **PRs** (`core::metrics::prs::tests`): PrStats default values, merged-PR handling
- **Models**: RepoSnapshot serde roundtrip, fields present, nested structs
- **Config**: token load/save, env precedence, TOML format
- **Cache**: path structure, save/load roundtrip, TTL/stale, clear
- **GitHub client**: unauthenticated and token-based `new()`

## 3. CLI

```powershell
# Default TUI mode
.\target\debug\forgeStat.exe ratatui-org/ratatui

# Explicit TUI
.\target\debug\forgeStat.exe ratatui-org/ratatui --tui

# GUI mode (if built with GUI)
.\target\debug\forgeStat.exe ratatui-org/ratatui --gui
```

Expect: repo and mode printed; no crash. (TUI/GUI rendering is not wired yet; that comes in later tasks.)

## 4. Live Star History (Optional)

To exercise the **Star History API and sparklines** against GitHub:

1. **Token (recommended):** Set `GITHUB_TOKEN` or add `github_token = "..."` to `~/.config/forgeStat/config.toml` to get 5,000 req/hr instead of 60.
2. Use the **library** from a small test binary or `cargo test` that calls the client (see below).

The binary currently does not call the GitHub client or cache; it only parses CLI and prints repo/mode. So “testing star history” is either:

- **Unit tests:** Already covered by `cargo test` (sparkline logic in `core::metrics::stars`).
- **Manual/integration:** Add a temporary `#[ignore]` test or a small example that builds `GitHubClient`, calls `client.stargazers("ratatui-org", "ratatui")`, and prints `total_count` and `sparkline_30d`/`sparkline_90d`/`sparkline_365d` (and/or run the same from a small `examples/` binary).

Example of what an integration-style check would do (conceptually):

```text
GitHubClient::new(token).stargazers("ratatui-org", "ratatui")
  -> StarHistory { total_count, sparkline_30d (len 30), sparkline_90d (len 13), sparkline_365d (len 12) }
```

## When to test

- **Every few tasks:** Run `cargo build` and `cargo test` (takes seconds). Catches regressions early.
- **Before a long run:** Quick smoke now (build + test) is better than finding a break after tasks 9–12.
- **Deep validation:** Optional live API check (e.g. full snapshot fetch) when you’re ready—e.g. after TUI is wired (later tasks).

## Summary

| Check              | Command / action        | What it validates                                |
|--------------------|-------------------------|--------------------------------------------------|
| Compile            | `cargo build`           | Code compiles                                    |
| Unit tests         | `cargo test`            | 36 tests (stars, issues, PRs, models, config, cache) |
| CLI                | `forgeStat owner/repo`  | CLI parsing and repo/mode output                 |
| Live API (opt)     | Example or integration test | Full snapshot against GitHub (when desired)  |
