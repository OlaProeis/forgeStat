# CLI Summary Output

The `--summary` flag provides a compact, human-readable summary of all 7 repository metrics with ANSI color coding, designed for quick inspection without launching the full TUI.

## Usage

```bash
# Show summary for a repository
forgeStat owner/repo --summary

# Combine with other flags (mutually exclusive with --json and --list)
echo "owner/repo" | forgeStat --from-stdin --summary
```

## Output Format

```
Repository: owner/repo
━━━━━━━━━━━━━━━━━━━━━━━━━━
Health Score: Calculating...

★ Stars:          5.2k     (+123 this month)
📋 Issues:         42 open  (oldest: 45 days)
🔀 Pull Requests:  8 open   (4 ready, 4 draft)
👥 Contributors:   156      (3 new this month)
🏷️  Releases:      v1.2.3   (5 days ago)
📊 Velocity:       12 issues/week, 8 PRs/week
🔒 Security:       2 alerts (1 high, 1 medium)
```

## Color Coding

Metrics are color-coded based on health indicators:

| Metric | Green | Yellow | Red |
|--------|-------|--------|-----|
| **Issues** | 0 open | 1-49 open | 50+ open |
| **Pull Requests** | 0 open | 1-9 open | 10+ open |
| **Security** | No alerts / only low | Medium alerts | Critical or high alerts |

Repository name and health score use **bold cyan** for prominence.

## Implementation

### Source Files

| File | Purpose |
|------|---------|
| `src/cli/summary.rs` | Summary formatting logic with ANSI colors |
| `src/cli/mod.rs` | Module exports |
| `src/main.rs` | CLI flag definition and handling |

### Key Function

```rust
pub fn format_summary(snapshot: &RepoSnapshot) -> String
```

Formats all metrics from a `RepoSnapshot` into a human-readable string with:
- Number formatting via `format_count()` (e.g., 5234 → "5.2k")
- 30-day star change calculation from sparkline data
- Issue age display (oldest open issue)
- PR breakdown (ready vs draft)
- New contributor count (last 30 days)
- Release timing
- Weekly velocity averages
- Security alert severity breakdown

### CLI Flag Definition

```rust
#[arg(long, conflicts_with_all = ["list", "from_stdin", "json"])]
summary: bool,
```

The flag conflicts with `--list`, `--from-stdin` (when not combined), and `--json` to ensure clear output modes.

### Exit Codes

| Exit Code | Meaning |
|-----------|---------|
| 0 | Success - summary printed to stdout |
| 1 | Error - snapshot fetch failed, error printed to stderr |

## Health Score Placeholder

The summary currently displays `Health Score: Calculating...` as a placeholder. The actual health score algorithm will be implemented in Task 3 (HealthScore Algorithm).
