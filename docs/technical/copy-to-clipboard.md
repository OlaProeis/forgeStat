# Copy-to-Clipboard Integration

Cross-platform clipboard copy with contextual content and toast notifications.

## Overview

Press `c` to copy contextually relevant content to the system clipboard:

| Panel | Content Copied | Example |
|-------|---------------|---------|
| **Issues** | Issue reference | `owner/repo#123` |
| **Contributors** | Username | `octocat` |
| **Releases** | Tag name | `v1.0.0` |
| **Other panels** | Repository URL | `https://github.com/owner/repo` |

## Usage

1. Navigate to the desired panel (use `Tab`, arrow keys, or `1-7`)
2. Scroll to the item you want to copy
3. Press `c` to copy
4. A toast notification appears in the status bar for 2 seconds

## Implementation

### Dependencies

Uses the [`arboard`](https://crates.io/crates/arboard) crate for cross-platform clipboard access:

```toml
arboard = "3"
```

### Key Components

**`src/core/models.rs`**
- `RepoSnapshot::repo_url()` — Returns full GitHub URL
- `RepoSnapshot::format_issue_reference(issue_number)` — Formats `owner/repo#123`

**`src/tui/app/mod.rs`** (clipboard handling in core App methods)
- `copy_to_clipboard()` — Main copy method with contextual dispatch
- `get_selected_issue_reference()` — Gets issue at scroll position
- `get_selected_contributor_username()` — Gets contributor at scroll position
- `get_selected_release_tag()` — Gets release at scroll position
- `show_toast(message)` — Displays toast notification
- `get_toast_message()` — Returns active toast if within duration

### Toast Notification

- Displays for 2 seconds (configurable via `toast_duration`)
- Shows checkmark icon (✓) and copied content preview
- Appears in the status bar, temporarily replacing status content
- Clears automatically after duration expires

### Error Handling

- Logs errors via `log::error!`
- Shows "Copy failed!" or "Clipboard unavailable" toast on failure
- Shows "Nothing to copy" if no content available

## Key Bindings

- `c` — Copy to clipboard (normal mode)
- `c` — Copy to clipboard (zoom mode, all panels)

## Context Hints

Status bar shows contextual copy hints:
- Issues panel: `c copy issue`
- Contributors panel: `c copy user`
- Releases panel: `c copy tag`
- Other panels: `c copy repo`
