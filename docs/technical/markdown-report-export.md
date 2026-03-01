# Markdown Report Export

## Overview

The `--report` CLI flag generates a formatted Markdown health report for any repository. This enables users to export repository metrics in a portable, human-readable format suitable for documentation, sharing, or archival.

## Key Files

- `src/cli/report.rs` - Report generation module with all formatting logic
- `src/cli/mod.rs` - Module exports
- `src/main.rs` - CLI argument handling and dispatch

## Implementation Details

### Report Structure

The generated report includes these sections:

1. **Header** - Repository name and generation timestamp
2. **Health Score** - Overall score, grade (A-F), and 4 sub-scores (Activity, Community, Maintenance, Growth)
3. **Stars** - Total count, milestone prediction, and 30d/90d growth summary
4. **Issues** - Open count, oldest issue age, and label breakdown table
5. **Pull Requests** - Open/ready/draft counts, merged count, and average merge time
6. **Contributors** - Total unique, new contributors (30d), and top 10 list
7. **Releases** - Last 5 releases with dates and average release interval
8. **Velocity** - 8-week activity tables for issues and PRs with weekly averages
9. **Security** - Alert counts by severity (requires authentication)

### CLI Arguments

```rust
#[arg(long, conflicts_with_all = ["list", "from_stdin", "json", "summary", "watchlist"])]
report: bool

#[arg(long, requires = "report")]
report_file: Option<PathBuf>
```

### Usage

```bash
# Output to stdout
forgeStat owner/repo --report

# Write to file
forgeStat owner/repo --report --report-file report.md
```

### Formatting Conventions

- Numbers use thousands separators (e.g., `5,234`)
- Dates formatted as `YYYY-MM-DD`
- Tables use standard Markdown syntax
- Emoji indicators for visual appeal (⭐ 📋 🔀 👥 🏷️ 📊 🔒)
- Grade displayed with color indicator emoji (🟢 🔵 🟡 🟠 🔴)

## Dependencies Used

- `chrono` - Date/time formatting
- Existing `core::health`, `core::metrics`, `core::models` modules

## Testing

The module includes 12 unit tests covering:
- Report structure and all sections
- Header generation with repository name
- Number formatting with thousands separators
- Conditional sections (security with/without data)
