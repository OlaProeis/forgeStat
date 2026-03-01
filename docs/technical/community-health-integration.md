# Community Health Integration

## Overview

Integration of GitHub's Community Health metrics into the repository health scoring system and TUI display. Community Health measures repository best practices like presence of README, LICENSE, contributing guidelines, and security policies.

## Key Files

- `src/core/health.rs` - Health score computation with CommunityHealth factor
- `src/core/models.rs` - `CommunityHealth` struct definition
- `src/tui/app/zoom.rs` - Security panel zoom view with Community Health checklist display
- `src/tui/app/mini_map.rs` - Mini-map health score display

## Implementation Details

### Health Score Integration

The Community Health score (0-100) now contributes to the **Maintenance** sub-score (0-25) in the health calculation:

| Community Health Score | Maintenance Bonus |
|------------------------|-------------------|
| 90-100 (Excellent)     | +5 points         |
| 75-89 (Good)           | +4 points         |
| 60-74 (Fair)           | +3 points         |
| 40-59 (Needs work)     | +2 points         |
| 20-39 (Poor)           | +1 point          |
| 0-19 (Critical)        | +0 points         |

The bonus is additive - repositories with strong community health practices get up to a 5-point boost to their maintenance score.

### TUI Display

In the **Security panel zoom view** (press `Enter` on Security panel), the right side now shows:

- Overall Community Health score (0-100) with color coding
- Checklist of community files with ✓/✗ indicators:
  - README.md
  - LICENSE
  - CONTRIBUTING.md
  - CODE_OF_CONDUCT.md
  - Issue Templates
  - PR Template
  - SECURITY.md

Color coding:
- **Green** (≥75): Good community health
- **Yellow** (50-74): Fair community health  
- **Red** (<50): Needs attention

### Data Model

```rust
pub struct CommunityHealth {
    pub has_readme: bool,
    pub has_license: bool,
    pub has_contributing: bool,
    pub has_code_of_conduct: bool,
    pub has_issue_templates: bool,
    pub has_pr_template: bool,
    pub has_security_policy: bool,
    pub score: u8,  // Overall 0-100
}
```

## Usage

### View Community Health in TUI

1. Launch: `cargo run -- owner/repo`
2. Navigate to **Security** panel (Tab/arrow keys)
3. Press `Enter` to zoom
4. View Community Health checklist on the right side

### CLI Summary

```bash
# View health score with community health contribution
cargo run -- owner/repo --summary
```

The Maintenance sub-score in the health breakdown includes the Community Health bonus.

### JSON Output

```bash
cargo run -- owner/repo --json
```

Returns full health score breakdown with all four sub-scores (Activity, Community, Maintenance, Growth).

## Testing

Three new tests verify the Community Health integration:

```bash
# Test perfect community health (score 100)
cargo test test_maintenance_score_with_community_health_perfect

# Test partial community health (score 50)
cargo test test_maintenance_score_with_community_health_partial

# Test missing community health data (None)
cargo test test_maintenance_score_with_community_health_none
```

## Dependencies Used

- `octocrab` - GitHub API client for fetching community health data
- `serde` - Serialization for caching community health metrics
