# Session Handover - forgeStat Phase 3

## Environment

- **Project:** forgeStat — OSS Contributor Dashboard
- **Path:** `g:\DEV\repowatch`
- **Tech Stack:** Rust 2021, ratatui (TUI), octocrab/reqwest (GitHub API), tokio, serde, clap
- **Context file:** Always read `@docs/ai-context.md` first — it contains project rules, architecture, and model selection guide
- **Branch:** master
- **Task Context:** phase3 (Task Master tag)

---

## Current Task: Task 16 — Integrate milestone prediction into displays

### Task Details

| Field | Value |
|-------|-------|
| **ID** | 16 |
| **Title** | Integrate milestone prediction into displays |
| **Complexity** | 3 |
| **Priority** | low |
| **Status** | pending |
| **Dependencies** | Task 6 (Star History) |

### Description

Display star milestone prediction in Stars zoom panel, `--summary` output, and `--report` markdown.

### Implementation Details

Integrate `predict_milestone()` function into three display locations:

1. **TUI Zoom Panel** (`src/tui/app/zoom.rs` stars render):
   - Add line: "At current pace: {milestone} ★ in ~{days} days"
   - Display below the braille sparkline in the stars zoom view

2. **CLI Summary** (`src/cli/summary.rs`):
   - Add milestone prediction to the formatted summary output
   - Include next milestone and estimated days

3. **Markdown Report** (`src/cli/report.rs` Stars section):
   - Add prediction to the markdown report Stars section
   - Format: "**Next Milestone:** {milestone} stars (estimated ~{days} days at current pace)"

The `predict_milestone()` function is in `src/core/metrics/stars.rs` and takes a `&StarHistory` reference, returning `Option<(u64, u64)>` (milestone, days).

### Key Files

| File | Purpose |
|------|---------|
| `src/tui/app/zoom.rs` | Stars zoom panel renderer |
| `src/cli/summary.rs` | CLI summary formatter |
| `src/cli/report.rs` | Markdown report generator |
| `src/core/metrics/stars.rs` | `predict_milestone()` function (already implemented) |

### Test Strategy

- Verify prediction displays correctly in TUI zoom mode (press Enter on stars panel)
- Verify prediction appears in `--summary` CLI output
- Verify prediction appears in `--report` markdown Stars section
- Test edge cases: no history data, negative growth (should handle gracefully)

---

## Verification

Before starting:

```bash
cargo check    # Should compile successfully
cargo test     # All tests should pass
```

---

## Model Selection

Task complexity 3 → **Kimi 2.5k**
