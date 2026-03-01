# Star Milestone Prediction

## Overview

Predicts the next star milestone (100, 500, 1k, 5k, 10k, 25k, 50k, 100k) based on 30-day and 90-day growth trends. Calculates a weighted average daily growth rate and estimates days to reach the next milestone.

## Key Files

- `src/core/metrics/stars.rs` - Core prediction algorithm and `MilestonePrediction` struct
- `src/tui/app/zoom.rs` - Displays prediction in Stars zoom view
- `src/cli/summary.rs` - Adds prediction to `--summary` CLI output

## Implementation Details

### Algorithm

The prediction uses a weighted average of 30-day and 90-day growth rates:
- **30-day rate**: 70% weight - reflects recent momentum
- **90-day rate**: 30% weight - provides historical context

```rust
daily_rate = (rate_30d * 0.7) + (rate_90d * 0.3)
```

### Milestone Selection

Standard milestones in ascending order:
```rust
[100, 500, 1000, 5000, 10000, 25000, 50000, 100000]
```

The next milestone is the smallest value greater than the current star count.

### Days Calculation

```rust
stars_needed = next_milestone - current_stars
estimated_days = ceil(stars_needed / daily_rate)
```

### Edge Cases

- **Growth stalled** (rate <= 0): Returns `None`, displays "Growth stalled"
- **Past all milestones**: Returns `None` for repos with 100k+ stars
- **No history data**: Returns `None` when sparklines are empty

## Usage

### TUI Zoom View

Press `Enter` on the Stars panel to see the prediction:
```
Next milestone: 1,000★ in 45 days (5.2/day)
```

Or when growth is stalled:
```
Next milestone: Growth stalled
```

### CLI Summary

```bash
$ repowatch owner/repo --summary
★ Stars:         5,234 (+123 this month) | 10,000★ in 912 days (5.2/day)
```

## API

```rust
use crate::core::metrics::stars::{predict_milestone, MilestonePrediction};
use crate::core::models::StarHistory;

let history = StarHistory { /* ... */ };
if let Some(prediction) = predict_milestone(&history) {
    println!("Next: {}★ in {} days", 
        prediction.next_milestone,
        prediction.estimated_days
    );
}
```

## Testing

14 unit tests cover:
- Normal growth scenarios
- Stalled/negative growth handling
- Milestone boundary conditions
- Weighted rate calculation accuracy
- Empty history edge cases

Run tests:
```bash
cargo test core::metrics::stars::tests
```
